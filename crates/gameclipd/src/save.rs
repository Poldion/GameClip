// SPDX-License-Identifier: GPL-3.0-or-later
//! Speichern eines Clips aus dem Ringpuffer (Design, „Speichern per Hotkey“ und Datei 43).
//!
//! Reihenfolge: Zeitstempel → auf das Videopaket warten → Schnappschuss (t0 = erster Keyframe)
//! → Dateiname → Index `pending` → MP4 (Video + Mischung) und `.m4a` (Einzelspuren) als `.part`
//! schreiben, sichern, umbenennen → Index `ok`.
//!
//! Zeitstempel: Video in µs relativ zu t0. Ton wird relativ zu t0 auf das Sample genau
//! gerundet (Zeitbasis 1/48 000), damit Mischung und Einzelspuren identische Raster haben
//! (S4: ±1 Sample). Die Ringzeiten sind aus Samples abgeleitet und um < 1 µs abgerundet; die
//! Rundung trifft also immer das ursprüngliche Sample.

use std::path::{Path, PathBuf};
use std::time::Duration;

use gameclip_capture_win::Track;
use gameclip_capture_win::ring::{PacketRef, PacketSource};
use gameclip_core::naming::{clip_file_name, dedupe};
use gameclip_core::retention::FolderKind;
use gameclip_core::state::SaveError;
use gameclip_media::MediaError;
use gameclip_media::mux::{Track as MuxTrack, write_m4a, write_mp4};
use rsmpeg::avcodec::{AVCodecParameters, AVPacket};
use rsmpeg::ffi;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use windows::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
use windows_core::HSTRING;

use crate::index::{Index, NewClip};
use crate::paths::Paths;

pub const SAVE_WAIT_TIMEOUT_MS: u64 = 500;
const MICROS: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 1_000_000,
};
const SAMPLES: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 48_000,
};

/// Was dauerhaft zum Speichern gebraucht wird.
pub struct SaveInputs<'a> {
    pub source: &'a dyn PacketSource,
    pub video_params: &'a AVCodecParameters,
    pub audio_params: &'a [(Track, AVCodecParameters)],
    pub buffer_us: i64,
    pub fps: u32,
    pub paths: &'a Paths,
    pub index: &'a Index,
}

pub struct SaveRequest {
    /// QPC-Zeit des Tastendrucks in µs.
    pub pressed_us: i64,
    pub program: Option<String>,
    /// Ortszeit für den Dateinamen.
    pub now_local: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub row_id: u32,
    pub tag_id: String,
    pub path: PathBuf,
    pub duration_us: u64,
    pub size_bytes: u64,
    pub has_tracks: bool,
    /// Kürzer als verlangt (Abschnittswechsel oder Lücke).
    pub truncated: bool,
    /// Die `.m4a` ließ sich nicht schreiben; die MP4 ist trotzdem gültig.
    pub tracks_error: Option<String>,
}

pub fn save_clip(inp: &SaveInputs<'_>, req: &SaveRequest) -> Result<Saved, SaveError> {
    // 1–2: auf das Videopaket zum Tastendruck warten (ohne B-Frames höchstens 2 Bilder).
    if !inp
        .source
        .wait_for(req.pressed_us, Duration::from_millis(SAVE_WAIT_TIMEOUT_MS))
    {
        return Err(SaveError::Timeout);
    }
    // 3: Schnappschuss.
    let snap = inp
        .source
        .snapshot(req.pressed_us - inp.buffer_us, req.pressed_us);
    if snap.is_empty() {
        return Err(SaveError::BufferEmpty);
    }
    let t0 = snap.t0_us;

    // 4: Dateiname im Clip-Ordner.
    if !inp.paths.clips.is_dir() {
        return Err(SaveError::FolderMissing);
    }
    let name = dedupe(
        &clip_file_name(req.program.as_deref(), req.now_local),
        |n| {
            let p = inp.paths.clips.join(n);
            p.exists() || part_path(&p).exists()
        },
    );
    let final_path = inp.paths.clips.join(&name);
    let tag_id = uuid::Uuid::new_v4().to_string();

    // 5: Index-Eintrag, bevor eine Datei sichtbar wird.
    let row_id = inp
        .index
        .insert_pending(&NewClip {
            tag_id: &tag_id,
            folder: FolderKind::Clips,
            path: &final_path,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: None,
        })
        .map_err(|_| SaveError::DiskFull)?;

    let created = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339");
    let version = env!("CARGO_PKG_VERSION");
    let mp4_tags = [
        ("gameclip_id", tag_id.as_str()),
        ("gameclip_created", created.as_str()),
        ("gameclip_version", version),
    ];

    // 6: MP4 mit Video und Mischung.
    let video: Vec<AVPacket> = snap.video.iter().map(|p| video_packet(p, t0)).collect();
    let mix = audio_track(&snap.audio, Track::Mix, t0);
    let mix_params = params_for(inp.audio_params, Track::Mix);
    let mp4_part = part_path(&final_path);
    let mp4_result = (|| -> Result<(), SaveError> {
        let audio: Vec<MuxTrack<'_>> = match (&mix, mix_params) {
            (Some(packets), Some(params)) => vec![MuxTrack {
                params,
                time_base: SAMPLES,
                packets,
                title: Track::Mix.title(),
                default: true,
            }],
            _ => Vec::new(),
        };
        write_mp4(
            &mp4_part,
            &MuxTrack {
                params: inp.video_params,
                time_base: MICROS,
                packets: &video,
                title: "Video",
                default: true,
            },
            &audio,
            &mp4_tags,
            0,
            false,
        )
        .map_err(|e| map_media_error(&e))?;
        rename_through(&mp4_part, &final_path)
    })();
    if let Err(e) = mp4_result {
        let _ = std::fs::remove_file(&mp4_part);
        let _ = inp.index.remove(row_id);
        return Err(e);
    }

    // 6–7: Einzelspuren; ein Fehler hier lässt die MP4 gültig.
    let tracks_error = write_tracks(inp, &snap.audio, &tag_id, t0).err();
    let has_tracks = tracks_error.is_none() && snap.audio.iter().any(|(t, _)| *t != Track::Mix);

    // 8: Eintrag gültig.
    let size_bytes = std::fs::metadata(&final_path).map(|m| m.len()).unwrap_or(0);
    let frame_us = 1_000_000 / i64::from(inp.fps.max(1));
    let duration_us = (snap.end_us - t0 + frame_us).max(0) as u64;
    inp.index
        .mark_ok(row_id, &final_path, size_bytes, duration_us, has_tracks)
        .map_err(|_| SaveError::DiskFull)?;

    Ok(Saved {
        row_id,
        tag_id,
        path: final_path,
        duration_us,
        size_bytes,
        has_tracks,
        truncated: snap.truncated,
        tracks_error,
    })
}

fn write_tracks(
    inp: &SaveInputs<'_>,
    audio: &[(Track, Vec<PacketRef>)],
    tag_id: &str,
    t0: i64,
) -> Result<(), String> {
    let singles: Vec<(Track, Vec<AVPacket>)> = audio
        .iter()
        .filter(|(t, _)| *t != Track::Mix)
        .filter_map(|(t, _)| audio_track(audio, *t, t0).map(|p| (*t, p)))
        .collect();
    if singles.is_empty() {
        return Ok(());
    }
    let final_path = inp.paths.track_file(tag_id);
    let part = part_path(&final_path);
    let result = (|| {
        std::fs::create_dir_all(&inp.paths.tracks).map_err(|e| e.to_string())?;
        let tracks: Vec<MuxTrack<'_>> = singles
            .iter()
            .filter_map(|(t, packets)| {
                Some(MuxTrack {
                    params: params_for(inp.audio_params, *t)?,
                    time_base: SAMPLES,
                    packets,
                    title: t.title(),
                    default: *t == Track::System,
                })
            })
            .collect();
        write_m4a(&part, &tracks, &[("gameclip_id", tag_id)], 0).map_err(|e| e.to_string())?;
        rename_through(&part, &final_path).map_err(|e| format!("{e:?}"))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&final_path);
    }
    result
}

fn part_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

fn params_for(all: &[(Track, AVCodecParameters)], track: Track) -> Option<&AVCodecParameters> {
    all.iter().find(|(t, _)| *t == track).map(|(_, p)| p)
}

fn video_packet(p: &PacketRef, t0: i64) -> AVPacket {
    let mut a = p.to_avpacket();
    a.set_pts(p.pts_us - t0);
    a.set_dts(p.dts_us - t0);
    a
}

/// Tonpakete einer Spur in Samples relativ zu t0.
fn audio_track(audio: &[(Track, Vec<PacketRef>)], track: Track, t0: i64) -> Option<Vec<AVPacket>> {
    let (_, packets) = audio.iter().find(|(t, _)| *t == track)?;
    Some(
        packets
            .iter()
            .map(|p| {
                let mut a = p.to_avpacket();
                let samples = ((p.pts_us - t0) as f64 * 48_000.0 / 1e6).round() as i64;
                a.set_pts(samples);
                a.set_dts(samples);
                a.set_duration(1024);
                a
            })
            .collect(),
    )
}

/// Umbenennen mit `MOVEFILE_WRITE_THROUGH` (kehrt erst zurück, wenn es auf der Platte ist).
fn rename_through(from: &Path, to: &Path) -> Result<(), SaveError> {
    unsafe {
        MoveFileExW(
            &HSTRING::from(from.as_os_str()),
            &HSTRING::from(to.as_os_str()),
            MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|e| win_error(e.code().0 as u32 & 0xFFFF))
}

fn win_error(code: u32) -> SaveError {
    match code {
        // ERROR_HANDLE_DISK_FULL, ERROR_DISK_FULL
        39 | 112 => SaveError::DiskFull,
        // ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_ACCESS_DENIED
        2 | 3 | 5 => SaveError::FolderMissing,
        _ => SaveError::EncoderFailed,
    }
}

fn map_media_error(e: &MediaError) -> SaveError {
    match e {
        MediaError::Io(io) => match io.raw_os_error() {
            Some(code) => win_error(code as u32),
            None => SaveError::EncoderFailed,
        },
        MediaError::Ffmpeg(rsmpeg::error::RsmpegError::AVError(code)) => match -code {
            // ENOSPC
            28 => SaveError::DiskFull,
            // ENOENT, EACCES
            2 | 13 => SaveError::FolderMissing,
            _ => SaveError::EncoderFailed,
        },
        MediaError::InvalidPath(_) => SaveError::FolderMissing,
        _ => SaveError::EncoderFailed,
    }
}
