// SPDX-License-Identifier: GPL-3.0-or-later
//! Speichern aus einem Ringpuffer mit echt kodierten Paketen (x264 + AAC), Prüfung mit ffprobe.
#![cfg(windows)]

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use gameclip_capture_win::Track;
use gameclip_capture_win::ring::{PacketSource, Ring, RingConfig, Snapshot, Stream};
use gameclip_core::retention::ClipStatus;
use gameclip_core::state::SaveError;
use gameclip_media::demux::{frame_times_us, read_tags};
use gameclip_media::{AudioEncoder, EncodeProfile, EncoderConfig, VideoEncoder};
use gameclip_testsupport::synth::{nv12_frame, sine};
use gameclipd::index::Index;
use gameclipd::paths::Paths;
use gameclipd::save::{SaveInputs, SaveRequest, save_clip};
use rsmpeg::avcodec::AVCodecParameters;
use rsmpeg::ffi;
use time::OffsetDateTime;

const FPS: u32 = 30;
/// Krummer QPC-Startwert, damit die Rundung auf Samples geprüft wird.
const START_US: i64 = 10_000_003;
const SECONDS: i64 = 5;

struct Env {
    _dir: tempfile::TempDir,
    paths: Paths,
    index: Index,
    ring: Ring,
    video_params: AVCodecParameters,
    audio_params: Vec<(Track, AVCodecParameters)>,
    video_count: usize,
}

fn bytes(p: &rsmpeg::avcodec::AVPacket) -> &[u8] {
    unsafe { std::slice::from_raw_parts(p.data, p.size as usize) }
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths {
        clips: dir.path().join("clips"),
        exports: dir.path().join("exports"),
        tracks: dir.path().join("tracks"),
        index: dir.path().join("index.sqlite"),
        settings: dir.path().join("settings.toml"),
        logs: dir.path().join("logs"),
    };
    paths.create_all().unwrap();
    let index = Index::open(&paths.index).unwrap();
    let ring = Ring::new(RingConfig {
        pool_blocks: 32,
        duration_us: 60_000_000,
        block_bytes: 2 * 1024 * 1024,
    });

    let cfg = EncoderConfig {
        width: 320,
        height: 240,
        fps: FPS,
        bitrate_bps: 1_000_000,
        profile: EncodeProfile::Capture,
    };
    let mut venc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let mut video_count = 0;
    let mut push_video = |enc: &mut VideoEncoder, ring: &Ring| {
        while let Some(p) = enc.receive().unwrap() {
            let key = p.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0;
            ring.push(Stream::Video, p.pts, p.dts, key, bytes(&p));
            video_count += 1;
        }
    };
    for n in 0..SECONDS * i64::from(FPS) {
        let mut f = nv12_frame(n as u32, 320, 240);
        venc.send(&mut f, START_US + n * 1_000_000 / i64::from(FPS), false)
            .unwrap();
        push_video(&mut venc, &ring);
    }
    venc.flush().unwrap();
    push_video(&mut venc, &ring);

    let mut audio_params = Vec::new();
    for (track, freq) in [
        (Track::Mix, 440.0),
        (Track::System, 440.0),
        (Track::Mic, 660.0),
    ] {
        let mut enc = AudioEncoder::new(128_000).unwrap();
        let mut packets = Vec::new();
        for start in (0..48_000 * SECONDS as usize).step_by(480) {
            packets.extend(enc.push(&sine(start, 480, freq)).unwrap());
        }
        packets.extend(enc.finish().unwrap());
        for p in &packets {
            // Wie die Audio-Pipeline: Samples → QPC-µs, abgerundet.
            let pts_us = START_US + p.pts * 1_000_000 / 48_000;
            ring.push(Stream::Audio(track), pts_us, pts_us, true, bytes(p));
        }
        audio_params.push((track, enc.codec_parameters()));
    }
    Env {
        _dir: dir,
        paths,
        index,
        ring,
        video_params: venc.codec_parameters(),
        audio_params,
        video_count,
    }
}

fn inputs<'a>(e: &'a Env, source: &'a dyn PacketSource) -> SaveInputs<'a> {
    SaveInputs {
        source,
        video_params: &e.video_params,
        audio_params: &e.audio_params,
        buffer_us: 3_000_000,
        fps: FPS,
        paths: &e.paths,
        index: &e.index,
    }
}

fn request() -> SaveRequest {
    SaveRequest {
        // Tastendruck beim letzten Bild (im Betrieb folgt das Bild nach dem Tastendruck).
        pressed_us: START_US + (SECONDS * i64::from(FPS) - 1) * 1_000_000 / i64::from(FPS),
        program: Some("TestGame".into()),
        now_local: OffsetDateTime::now_utc(),
    }
}

fn ffprobe(path: &Path) -> serde_json::Value {
    let exe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ffmpeg/bin/ffprobe.exe");
    let out = Command::new(exe)
        .args([
            "-v",
            "error",
            "-of",
            "json",
            "-show_streams",
            "-show_format",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn f(v: &serde_json::Value) -> f64 {
    v.as_str().unwrap().parse().unwrap()
}

#[test]
fn saves_mp4_with_mix_and_m4a_with_single_tracks() {
    let e = env();
    let saved = save_clip(&inputs(&e, &e.ring), &request()).unwrap();
    println!("{saved:?}");
    assert!(saved.path.ends_with(saved.path.file_name().unwrap()));
    assert!(
        saved
            .path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("TestGame_")
    );
    assert!(saved.has_tracks && saved.tracks_error.is_none());
    // Start am letzten Keyframe ≤ Tastendruck − 3 s (1,97 s → Keyframe bei 1 s): der Clip ist
    // zwischen Pufferlänge und Pufferlänge plus einer Bildgruppe lang, hier 4 s.
    assert_eq!(saved.duration_us, 4 * 1_000_000 - 1);

    let row = e.index.get(saved.row_id).unwrap().unwrap();
    assert_eq!(row.status, ClipStatus::Ok);
    assert_eq!(row.path, saved.path);
    assert_eq!(row.size_bytes, saved.size_bytes);
    assert!(row.has_tracks);
    assert!(!saved.path.with_extension("mp4.part").exists());

    // MP4: Video + Mischung, Tags.
    let tags = read_tags(&saved.path).unwrap();
    assert_eq!(tags["gameclip_id"], saved.tag_id);
    assert!(tags.contains_key("gameclip_created") && tags.contains_key("gameclip_version"));
    let mp4 = ffprobe(&saved.path);
    let streams = mp4["streams"].as_array().unwrap();
    assert_eq!(streams.len(), 2);
    assert_eq!(streams[1]["tags"]["handler_name"], "Mischung");
    assert_eq!(frame_times_us(&saved.path).unwrap().len(), 120);

    // .m4a: System + Mikrofon, gleicher Tag, deckungsgleich mit der Mischung.
    let m4a_path = e.paths.track_file(&saved.tag_id);
    let m4a = ffprobe(&m4a_path);
    let tracks = m4a["streams"].as_array().unwrap();
    let titles: Vec<&str> = tracks
        .iter()
        .map(|s| s["tags"]["handler_name"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["System", "Mikrofon"]);
    assert_eq!(read_tags(&m4a_path).unwrap()["gameclip_id"], saved.tag_id);
    let mix_start = f(&streams[1]["start_time"]);
    let mix_frames: i64 = streams[1]["duration_ts"].as_i64().unwrap();
    for t in tracks {
        println!(
            "{}: start {} duration_ts {} (Mischung {mix_start} / {mix_frames})",
            t["tags"]["handler_name"], t["start_time"], t["duration_ts"]
        );
        assert!(
            (f(&t["start_time"]) - mix_start).abs() < 1.0 / 48_000.0,
            "Versatz zur Mischung"
        );
        assert_eq!(
            t["duration_ts"].as_i64().unwrap(),
            mix_frames,
            "gleiche Länge in Samples"
        );
    }
    assert!(e.video_count >= 150);
}

struct NeverReady;

impl PacketSource for NeverReady {
    fn wait_for(&self, _: i64, _: Duration) -> bool {
        false
    }
    fn snapshot(&self, _: i64, _: i64) -> Snapshot {
        unreachable!()
    }
}

#[test]
fn timeout_leaves_no_entry() {
    let e = env();
    let err = save_clip(&inputs(&e, &NeverReady), &request()).unwrap_err();
    assert_eq!(err, SaveError::Timeout);
    assert!(
        e.index
            .list(gameclip_core::retention::FolderKind::Clips)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn missing_folder_is_reported() {
    let mut e = env();
    e.paths.clips = e.paths.clips.join("gibt-es-nicht");
    let err = save_clip(&inputs(&e, &e.ring), &request()).unwrap_err();
    assert_eq!(err, SaveError::FolderMissing);
}

#[test]
fn failed_tracks_keep_the_mp4() {
    let mut e = env();
    // Der Spurordner ist eine Datei: die .m4a lässt sich nicht anlegen.
    let blocker = e.paths.tracks.with_extension("blocker");
    std::fs::write(&blocker, b"x").unwrap();
    e.paths.tracks = blocker;
    let saved = save_clip(&inputs(&e, &e.ring), &request()).unwrap();
    assert!(saved.tracks_error.is_some());
    assert!(!saved.has_tracks);
    assert!(saved.path.exists());
    assert!(!e.index.get(saved.row_id).unwrap().unwrap().has_tracks);
}
