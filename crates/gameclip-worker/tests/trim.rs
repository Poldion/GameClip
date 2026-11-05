// SPDX-License-Identifier: GPL-3.0-or-later
//! Kürzen und Export mit nummerierten Testbildern (Design, Datei 57).
#![cfg(windows)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gameclip_core::export_plan::plan;
use gameclip_media::audio_decode::decode_audio;
use gameclip_media::decode::Decoder;
use gameclip_media::demux::{frame_times_us, read_tags, video_info};
use gameclip_media::encoder::TIME_BASE_DEN;
use gameclip_media::mux::{Track, write_m4a, write_mp4};
use gameclip_media::{AudioEncoder, EncodeProfile, EncoderConfig, VideoEncoder};
use gameclip_protocol::messages::ExportProfile;
use gameclip_protocol::worker::{WorkerError, WorkerJob, WorkerMessage};
use gameclip_testsupport::synth::{nv12_frame, read_number, sine};
use gameclip_worker::trim::run;
use rsmpeg::ffi;

const W: u32 = 640;
const H: u32 = 360;
const FPS: i64 = 30;
const FRAMES: u32 = 150;

fn frame_us(n: i64) -> i64 {
    n * 1_000_000 / FPS
}

struct Source {
    _dir: tempfile::TempDir,
    clip: PathBuf,
    tracks: PathBuf,
}

/// 5-s-Clip wie vom Dienst: MP4 mit Video und Mischung, `.m4a` mit System und Mikrofon.
fn source() -> Source {
    source_with(FRAMES, 4_000_000)
}

/// Wie `source`, mit `frames` Bildern bei `bitrate_bps`.
fn source_with(frames: u32, bitrate_bps: u64) -> Source {
    let dir = tempfile::tempdir().unwrap();
    let clip = dir.path().join("clip.mp4");
    let tracks = dir.path().join("tracks.m4a");
    let cfg = EncoderConfig {
        width: W,
        height: H,
        fps: FPS as u32,
        bitrate_bps,
        profile: EncodeProfile::Capture,
    };
    let mut venc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let mut vpk = Vec::new();
    for i in 0..frames {
        let mut f = nv12_frame(i, W, H);
        venc.send(&mut f, frame_us(i64::from(i)), false).unwrap();
        while let Some(p) = venc.receive().unwrap() {
            vpk.push(p);
        }
    }
    venc.flush().unwrap();
    while let Some(p) = venc.receive().unwrap() {
        vpk.push(p);
    }
    let audio = |freq: f32| {
        let mut enc = AudioEncoder::new(128_000).unwrap();
        let mut pk = Vec::new();
        let samples = i64::from(frames) * 48_000 / FPS;
        for start in (0..samples as usize).step_by(480) {
            pk.extend(enc.push(&sine(start, 480, freq)).unwrap());
        }
        pk.extend(enc.finish().unwrap());
        (enc.codec_parameters(), pk)
    };
    let (mix_par, mix) = audio(440.0);
    let (sys_par, sys) = audio(440.0);
    let (mic_par, mic) = audio(660.0);
    let tb = |den| ffi::AVRational { num: 1, den };
    let vpar = venc.codec_parameters();
    write_mp4(
        &clip,
        &Track {
            params: &vpar,
            time_base: tb(TIME_BASE_DEN),
            packets: &vpk,
            title: "Video",
            default: true,
        },
        &[Track {
            params: &mix_par,
            time_base: tb(48_000),
            packets: &mix,
            title: "Mischung",
            default: true,
        }],
        &[("gameclip_id", "quelle")],
        0,
        false,
    )
    .unwrap();
    write_m4a(
        &tracks,
        &[
            Track {
                params: &sys_par,
                time_base: tb(48_000),
                packets: &sys,
                title: "System",
                default: true,
            },
            Track {
                params: &mic_par,
                time_base: tb(48_000),
                packets: &mic,
                title: "Mikrofon",
                default: false,
            },
        ],
        &[("gameclip_id", "quelle")],
        0,
    )
    .unwrap();
    Source {
        _dir: dir,
        clip,
        tracks,
    }
}

fn job(src: &Source, out: &Path, in_us: i64, out_us: i64, profile: ExportProfile) -> WorkerJob {
    WorkerJob {
        source: src.clip.to_string_lossy().into(),
        tracks_path: Some(src.tracks.to_string_lossy().into()),
        target_part_path: out.to_string_lossy().into(),
        output_tag_id: "ausgabe-1".into(),
        in_us: in_us as u64,
        out_us: out_us as u64,
        profile,
        encoder_order: Vec::new(),
        adapter_luid: 0,
    }
}

/// Bildnummern aller Bilder einer Datei (Software-Decoder).
fn numbers(path: &Path) -> Vec<u32> {
    let mut dec = Decoder::open(path, None).unwrap();
    let mut out = Vec::new();
    while let Some(f) = dec.next_frame().unwrap() {
        let luma = unsafe {
            std::slice::from_raw_parts(
                f.frame.data[0],
                (f.frame.linesize[0] * f.frame.height) as usize,
            )
        };
        out.push(read_number(luma, f.frame.linesize[0] as usize));
    }
    out
}

fn ffprobe_streams(path: &Path) -> Vec<serde_json::Value> {
    let exe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ffmpeg/bin/ffprobe.exe");
    let out = Command::new(exe)
        .args(["-v", "error", "-of", "json", "-show_streams"])
        .arg(path)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["streams"].as_array().unwrap().clone()
}

#[test]
fn cut_at_frame_n_starts_with_frame_n() {
    let src = source();
    let out = src._dir.path().join("trim.mp4.part");
    let (first, end) = (37, 97);
    let mut messages = Vec::new();
    let result = run(
        &job(
            &src,
            &out,
            frame_us(first),
            frame_us(end),
            ExportProfile::Mix,
        ),
        &mut |m| messages.push(m),
    )
    .unwrap();
    println!("{result:?}; Meldungen: {:?}", &messages[..2]);
    assert!(matches!(messages[0], WorkerMessage::Encoder { .. }));
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, WorkerMessage::Progress { percent } if *percent == 100.0))
    );

    let n = numbers(&out);
    assert_eq!(n.first(), Some(&(first as u32)), "erstes Bild ist Bild N");
    assert_eq!(
        n.last(),
        Some(&(end as u32 - 1)),
        "out ist ausschließliche Grenze"
    );
    assert_eq!(n.len() as i64, end - first);
    assert_eq!(frame_times_us(&out).unwrap()[0], 0);
    assert_eq!(read_tags(&out).unwrap()["gameclip_id"], "ausgabe-1");

    // Ton so lang wie das Video (±1 ms), gemessen an der Spurdauer im Container. Der Decoder
    // liefert zusätzlich die AAC-Füllung des letzten 1024er-Blocks (hier 256 Samples).
    let video_us = (end - first) * 1_000_000 / FPS;
    let audio = decode_audio(&out).unwrap();
    assert_eq!(audio.len(), 1);
    let streams = ffprobe_streams(&out);
    let audio_us = (streams[1]["duration"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap()
        * 1e6) as i64;
    println!(
        "Video {video_us} µs, Ton laut Container {audio_us} µs, dekodiert {} Samples",
        audio[0].frames()
    );
    assert!(
        (audio_us - video_us).abs() <= 1_000,
        "Ton {audio_us} µs, Video {video_us} µs"
    );
    assert_eq!(result.duration_us as i64, video_us);
}

#[test]
fn separate_tracks_keep_order_and_default() {
    let src = source();
    let out = src._dir.path().join("tracks.mp4.part");
    run(
        &job(&src, &out, 0, frame_us(60), ExportProfile::SeparateTracks),
        &mut |_| {},
    )
    .unwrap();
    let streams = ffprobe_streams(&out);
    let titles: Vec<&str> = streams
        .iter()
        .map(|s| s["tags"]["handler_name"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Video", "Mischung", "System", "Mikrofon"]);
    let defaults: Vec<i64> = streams[1..]
        .iter()
        .map(|s| s["disposition"]["default"].as_i64().unwrap())
        .collect();
    assert_eq!(defaults, [1, 0, 0], "nur die Mischung ist Standardspur");
}

#[test]
fn target_size_stays_below_target_or_reports_too_large() {
    let src = source();
    let info = video_info(&src.clip).unwrap();
    for (seconds, target) in [(4, 2_000_000u64), (4, 600_000), (2, 250_000), (4, 60_000)] {
        let out = src
            ._dir
            .path()
            .join(format!("size_{seconds}_{target}.mp4.part"));
        let span = frame_us(FPS * seconds);
        let predicted = plan(span as u64, target, (info.width, info.height, info.fps));
        let result = run(
            &job(
                &src,
                &out,
                0,
                span,
                ExportProfile::TargetSize { bytes: target },
            ),
            &mut |_| {},
        );
        println!("{seconds} s → {target} Bytes: Plan {predicted:?}, Ergebnis {result:?}");
        match (predicted, result) {
            (Err(_), Err(f)) => assert_eq!(f.0, WorkerError::TooLarge),
            (Ok(_), Ok(o)) => assert!(o.bytes <= target, "{} > {target}", o.bytes),
            (Ok(_), Err(f)) => {
                assert_eq!(f.0, WorkerError::TooLarge, "nur TooLarge erlaubt: {f:?}")
            }
            (Err(_), Ok(o)) => panic!("Plan sagt TooSmall, Export ergab {o:?}"),
        }
    }
}

#[test]
fn binary_speaks_json_lines() {
    let src = source();
    let out = src._dir.path().join("bin.mp4.part");
    let exe = env!("CARGO_BIN_EXE_gameclip-worker");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let j = job(&src, &out, 0, frame_us(30), ExportProfile::Mix);
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&j).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let lines: Vec<WorkerMessage> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(output.status.code(), Some(0));
    assert!(matches!(lines.last(), Some(WorkerMessage::Done { .. })));
    assert!(out.exists());

    // Fehlende Quelle: Exit 1, letzte Meldung ist der Fehler, keine .part bleibt liegen.
    let mut bad = j.clone();
    bad.source = "C:\\gibt\\es\\nicht.mp4".into();
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(&bad).unwrap().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let last: WorkerMessage = serde_json::from_str(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        last,
        WorkerMessage::Error {
            code: WorkerError::InputMissing,
            ..
        }
    ));
}

/// S11: 20 Größen-Exporte von 5 bis 300 s aus einer 300-s-Quelle mit bewegtem Rauschen. Jeder
/// bleibt höchstens bei der Zielgröße oder endet mit `TooLarge`, wenn `export_plan` das vorhersagt.
/// Dauert einige Minuten; Start: `cargo test -p gameclip-worker --test trim -- --ignored s11`.
#[test]
#[ignore]
fn s11_twenty_target_size_exports() {
    let src = source_with(300 * FPS as u32, 1_500_000);
    let info = video_info(&src.clip).unwrap();
    let seconds = [
        5, 10, 15, 20, 30, 40, 50, 60, 75, 90, 105, 120, 140, 160, 180, 200, 225, 250, 275, 300,
    ];
    let targets = [10_000_000u64, 8_000_000, 25_000_000, 4_000_000, 1_500_000];
    let mut rows = Vec::new();
    for (i, s) in seconds.iter().enumerate() {
        let target = targets[i % targets.len()];
        let span = frame_us(FPS * s);
        let out = src._dir.path().join(format!("s11_{s}.mp4.part"));
        let predicted = plan(span as u64, target, (info.width, info.height, info.fps));
        let started = std::time::Instant::now();
        let result = run(
            &job(
                &src,
                &out,
                0,
                span,
                ExportProfile::TargetSize { bytes: target },
            ),
            &mut |_| {},
        );
        let secs = started.elapsed().as_secs_f64();
        let line = match (&predicted, &result) {
            (Err(_), Err(f)) => {
                assert_eq!(f.0, WorkerError::TooLarge);
                format!(
                    "{s} s | {} MB | TooSmall vorhergesagt | TooLarge | {secs:.1} s",
                    target / 1_000_000
                )
            }
            (Ok(p), Ok(o)) => {
                assert!(o.bytes <= target, "{s} s: {} > {target}", o.bytes);
                format!(
                    "{s} s | {} MB | {}p{} {} kbit/s | {:.2} MB ({:.0} %) | {secs:.1} s",
                    target / 1_000_000,
                    p.height,
                    p.fps,
                    p.video_bps / 1000,
                    o.bytes as f64 / 1e6,
                    o.bytes as f64 * 100.0 / target as f64
                )
            }
            (Ok(_), Err(f)) => {
                assert_eq!(
                    f.0,
                    WorkerError::TooLarge,
                    "{s} s: nur TooLarge erlaubt: {f:?}"
                );
                format!(
                    "{s} s | {} MB | Plan ok | TooLarge nach Korrektur | {secs:.1} s",
                    target / 1_000_000
                )
            }
            (Err(_), Ok(o)) => panic!("{s} s: Plan sagt TooSmall, Export ergab {o:?}"),
        };
        println!("{line}");
        rows.push(line);
        let _ = std::fs::remove_file(&out);
    }
    assert_eq!(rows.len(), 20);
}
