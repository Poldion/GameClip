// SPDX-License-Identifier: GPL-3.0-or-later
//! MP4- und M4A-Dateien aus fertig kodierten Paketen schreiben (Design, Datei 29).
//!
//! Alle Spuren werden um `t0_us` verschoben, sodass der Clip bei 0 beginnt. Pakete, die danach
//! negativ sind (AAC-Anlauf, Ton kurz vor dem ersten Bild), blendet der mov-Muxer über die
//! Edit-List aus. Die Pakete werden nach Dekodierzeit verschränkt geschrieben.

use std::ffi::CString;
use std::path::Path;

use rsmpeg::UnsafeDerefMut;
use rsmpeg::avcodec::{AVCodecParameters, AVPacket};
use rsmpeg::avformat::AVFormatContextOutput;
use rsmpeg::avutil::AVDictionary;
use rsmpeg::ffi;

use crate::{MediaError, Result};

const MICROS: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 1_000_000,
};

/// Eine Spur mit ihren Paketen in der Zeitbasis `time_base`.
pub struct Track<'a> {
    pub params: &'a AVCodecParameters,
    pub time_base: ffi::AVRational,
    pub packets: &'a [AVPacket],
    /// Spurtitel, landet als `handler_name` in der Datei.
    pub title: &'a str,
    pub default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// Clips und Exporte (`.mp4`).
    Mp4,
    /// Einzelspuren unter `tracks\<tag_id>.m4a`.
    M4a,
}

impl Container {
    fn format_name(self) -> &'static std::ffi::CStr {
        match self {
            Container::Mp4 => c"mp4",
            Container::M4a => c"ipod",
        }
    }
}

/// Clip oder Export: Video als erste Spur, danach die Tonspuren. `faststart` für Exporte.
pub fn write_mp4(
    path: &Path,
    video: &Track<'_>,
    audio: &[Track<'_>],
    tags: &[(&str, &str)],
    t0_us: i64,
    faststart: bool,
) -> Result<()> {
    let tracks: Vec<&Track<'_>> = std::iter::once(video).chain(audio).collect();
    write(path, Container::Mp4, &tracks, tags, t0_us, faststart)
}

/// Einzelspuren eines Clips als M4A.
pub fn write_m4a(
    path: &Path,
    audio: &[Track<'_>],
    tags: &[(&str, &str)],
    t0_us: i64,
) -> Result<()> {
    let tracks: Vec<&Track<'_>> = audio.iter().collect();
    write(path, Container::M4a, &tracks, tags, t0_us, false)
}

/// Schreibt `tracks` nach `path`. Das Format wird ausdrücklich übergeben, weil die Endung
/// `.part` es nicht verrät.
pub fn write(
    path: &Path,
    container: Container,
    tracks: &[&Track<'_>],
    tags: &[(&str, &str)],
    t0_us: i64,
    faststart: bool,
) -> Result<()> {
    let filename = path
        .to_str()
        .and_then(|s| CString::new(s).ok())
        .ok_or_else(|| MediaError::InvalidPath(path.display().to_string()))?;
    {
        let mut out = AVFormatContextOutput::builder()
            .format_name(container.format_name())
            .filename(&filename)
            .build()?;

        for track in tracks {
            let mut stream = out.new_stream();
            stream.set_codecpar(track.params.clone());
            stream.set_time_base(track.time_base);
            if track.default {
                stream.set_disposition(ffi::AV_DISPOSITION_DEFAULT as i32);
            }
            let title = cstring(track.title)?;
            stream.set_metadata(Some(AVDictionary::new(c"handler_name", &title, 0)));
            unsafe { stream.codecpar_mut().deref_mut().codec_tag = 0 };
        }

        if !tags.is_empty() {
            let mut dict: Option<AVDictionary> = None;
            for (key, value) in tags {
                let (key, value) = (cstring(key)?, cstring(value)?);
                dict = Some(match dict {
                    None => AVDictionary::new(&key, &value, 0),
                    Some(d) => d.set(&key, &value, 0),
                });
            }
            if let Some(d) = dict {
                unsafe { out.deref_mut().metadata = d.into_raw().as_ptr() };
            }
        }

        let movflags = if faststart {
            c"+use_metadata_tags+faststart"
        } else {
            c"+use_metadata_tags"
        };
        let mut options = Some(AVDictionary::new(c"movflags", movflags, 0));
        out.write_header(&mut options)?;

        // Die Muxer-Zeitbasis kann nach write_header von der Spur abweichen.
        let stream_tb: Vec<ffi::AVRational> = out.streams().iter().map(|s| s.time_base).collect();

        for (index, packet, duration) in interleave(tracks) {
            let track = tracks[index];
            let shift = unsafe { ffi::av_rescale_q(t0_us, MICROS, track.time_base) };
            let mut p = ref_packet(packet)?;
            p.set_stream_index(index as i32);
            if p.duration <= 0 {
                p.set_duration(duration);
            }
            p.set_pts(p.pts - shift);
            p.set_dts(p.dts - shift);
            p.rescale_ts(track.time_base, stream_tb[index]);
            out.interleaved_write_frame(&mut p)?;
        }
        out.write_trailer()?;
        // `out` schließt beim Verlassen des Blocks die Datei.
    }
    // FlushFileBuffers braucht ein Handle mit Schreibrecht.
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .sync_all()?;
    Ok(())
}

/// Alle Pakete als (Spur, Paket, Ersatzdauer), aufsteigend nach Dekodierzeit in µs; bei
/// Gleichstand bleibt die Spurreihenfolge erhalten (stabile Sortierung).
fn interleave<'a>(tracks: &[&'a Track<'a>]) -> Vec<(usize, &'a AVPacket, i64)> {
    let mut all: Vec<(i64, usize, &AVPacket, i64)> = Vec::new();
    for (i, t) in tracks.iter().enumerate() {
        let dts: Vec<i64> = t.packets.iter().map(decode_ts).collect();
        let durations = fill_durations(&dts);
        for ((p, ts), d) in t.packets.iter().zip(&dts).zip(durations) {
            all.push((
                unsafe { ffi::av_rescale_q(*ts, t.time_base, MICROS) },
                i,
                p,
                d,
            ));
        }
    }
    all.sort_by_key(|(ts, _, _, _)| *ts);
    all.into_iter().map(|(_, i, p, d)| (i, p, d)).collect()
}

fn decode_ts(p: &AVPacket) -> i64 {
    if p.dts == ffi::AV_NOPTS_VALUE {
        p.pts
    } else {
        p.dts
    }
}

/// Ersatzdauer je Paket aus dem Abstand zum nächsten; das letzte erbt die vorige.
/// Hardware-Encoder (AMF, gemessen 2026-11) setzen keine Dauer; ohne sie gibt der mov-Muxer
/// dem letzten Bild die Länge 0 und es fehlt beim Abspielen.
pub(crate) fn fill_durations(dts: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = dts.windows(2).map(|w| (w[1] - w[0]).max(0)).collect();
    if !dts.is_empty() {
        out.push(out.last().copied().unwrap_or(0));
    }
    out
}

/// Neue Referenz auf dieselben Daten; das Original bleibt im Ringpuffer.
fn ref_packet(src: &AVPacket) -> Result<AVPacket> {
    let mut p = AVPacket::new();
    let ret = unsafe { ffi::av_packet_ref(p.as_mut_ptr(), src.as_ptr()) };
    if ret < 0 {
        return Err(MediaError::Ffmpeg(rsmpeg::error::RsmpegError::AVError(ret)));
    }
    Ok(p)
}

fn cstring(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| MediaError::InvalidPath(s.into()))
}

/// Schreibt `src` ohne Neukodieren nach `dst` um und setzt dabei Tags (z. B. eine neue
/// `gameclip_id` beim Übernehmen einer Kopie). Übrige Tags, Spurtitel und Standard-Kennzeichen
/// bleiben; die Zeitstempel bleiben, eine Edit-List entsteht daraus erneut.
pub fn remux_with_tags(src: &Path, dst: &Path, overrides: &[(&str, &str)]) -> Result<()> {
    let mut input = crate::demux::open_input(src)?;
    struct In {
        params: AVCodecParameters,
        time_base: ffi::AVRational,
        title: String,
        default: bool,
        packets: Vec<AVPacket>,
    }
    let mut streams: Vec<In> = input
        .streams()
        .iter()
        .map(|s| In {
            params: s.codecpar().clone(),
            time_base: s.time_base,
            title: s
                .metadata()
                .and_then(|m| {
                    m.iter()
                        .find(|e| e.key() == c"handler_name")
                        .map(|e| e.value().to_string_lossy().into_owned())
                })
                .unwrap_or_default(),
            default: s.disposition & ffi::AV_DISPOSITION_DEFAULT as i32 != 0,
            packets: Vec::new(),
        })
        .collect();
    let mut tags: Vec<(String, String)> = input
        .metadata()
        .map(|d| {
            d.iter()
                .map(|e| {
                    (
                        e.key().to_string_lossy().into_owned(),
                        e.value().to_string_lossy().into_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    // Vom Muxer selbst gesetzte Schlüssel nicht doppelt schreiben.
    tags.retain(|(k, _)| {
        !overrides.iter().any(|(o, _)| o == k)
            && !matches!(
                k.as_str(),
                "major_brand" | "minor_version" | "compatible_brands" | "encoder"
            )
    });
    tags.extend(
        overrides
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string())),
    );
    while let Some(p) = input.read_packet()? {
        if let Some(s) = streams.get_mut(p.stream_index as usize) {
            s.packets.push(p);
        }
    }
    drop(input);
    let tracks: Vec<Track<'_>> = streams
        .iter()
        .map(|s| Track {
            params: &s.params,
            time_base: s.time_base,
            packets: &s.packets,
            title: &s.title,
            default: s.default,
        })
        .collect();
    let refs: Vec<&Track<'_>> = tracks.iter().collect();
    let tag_refs: Vec<(&str, &str)> = tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let container = if dst
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("m4a"))
    {
        Container::M4a
    } else {
        Container::Mp4
    };
    write(dst, container, &refs, &tag_refs, 0, true)
}
#[cfg(test)]
mod tests {
    use super::fill_durations;

    #[test]
    fn last_packet_inherits_previous_duration() {
        assert_eq!(
            fill_durations(&[0, 16_666, 33_333, 50_000]),
            [16_666, 16_667, 16_667, 16_667]
        );
        assert_eq!(fill_durations(&[5]), [0]);
        assert!(fill_durations(&[]).is_empty());
    }
}
