// SPDX-License-Identifier: GPL-3.0-or-later
//! Kodieren und Muxen ohne GPU (libx264 + aac), Prüfung mit ffprobe.
#![cfg(windows)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gameclip_media::encoder::TIME_BASE_DEN;
use gameclip_media::mux::{Track, write_m4a, write_mp4};
use gameclip_media::{AudioEncoder, EncodeProfile, EncoderConfig, VideoEncoder};
use rsmpeg::avcodec::AVPacket;
use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi;
use serde_json::Value;

const FPS: u32 = 30;
const SECONDS: i64 = 10;

fn ffprobe(path: &Path, args: &[&str]) -> Value {
    let exe =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ffmpeg/bin/ffprobe.exe");
    let out = Command::new(exe)
        .args(["-v", "error", "-of", "json"])
        .args(args)
        .arg(path)
        .output()
        .expect("ffprobe startet");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gameclip-media-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn encode_video() -> (VideoEncoder, Vec<AVPacket>) {
    let cfg = EncoderConfig {
        width: 320,
        height: 240,
        fps: FPS,
        bitrate_bps: 1_000_000,
        profile: EncodeProfile::Capture,
    };
    let mut enc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let frame_us = i64::from(TIME_BASE_DEN) / i64::from(FPS);
    let mut packets = Vec::new();
    for i in 0..SECONDS * i64::from(FPS) {
        let mut f = AVFrame::new();
        f.set_format(ffi::AV_PIX_FMT_NV12);
        f.set_width(320);
        f.set_height(240);
        f.get_buffer(0).unwrap();
        // Helligkeit wechselt, damit der Encoder echte Bilder erzeugt.
        let luma =
            unsafe { std::slice::from_raw_parts_mut(f.data[0], (f.linesize[0] * 240) as usize) };
        luma.fill((16 + (i * 7) % 200) as u8);
        let chroma =
            unsafe { std::slice::from_raw_parts_mut(f.data[1], (f.linesize[1] * 120) as usize) };
        chroma.fill(128);
        enc.send(&mut f, i * frame_us, i == 0).unwrap();
        while let Some(p) = enc.receive().unwrap() {
            packets.push(p);
        }
    }
    enc.flush().unwrap();
    while let Some(p) = enc.receive().unwrap() {
        packets.push(p);
    }
    (enc, packets)
}

fn encode_audio(freq: f32) -> (AudioEncoder, Vec<AVPacket>) {
    let mut enc = AudioEncoder::new(128_000).unwrap();
    let mut packets = Vec::new();
    let total = 48_000 * SECONDS as usize;
    for start in (0..total).step_by(480) {
        let block: Vec<f32> = (start..start + 480)
            .flat_map(|i| {
                let s = (i as f32 * freq * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
                [s, s]
            })
            .collect();
        packets.extend(enc.push(&block).unwrap());
    }
    packets.extend(enc.finish().unwrap());
    (enc, packets)
}

fn tb(den: i32) -> ffi::AVRational {
    ffi::AVRational { num: 1, den }
}

fn f(v: &Value) -> f64 {
    v.as_str().unwrap().parse().unwrap()
}

#[test]
fn mp4_has_tracks_titles_tags_and_aligned_start() {
    let (venc, vpk) = encode_video();
    let (mix, mix_pk) = encode_audio(440.0);
    let (mic, mic_pk) = encode_audio(660.0);
    let (vpar, mpar, cpar) = (
        venc.codec_parameters(),
        mix.codec_parameters(),
        mic.codec_parameters(),
    );

    let path = temp_file("clip.mp4.part");
    write_mp4(
        &path,
        &Track {
            params: &vpar,
            time_base: tb(TIME_BASE_DEN),
            packets: &vpk,
            title: "Video",
            default: true,
        },
        &[
            Track {
                params: &mpar,
                time_base: tb(48_000),
                packets: &mix_pk,
                title: "Mischung",
                default: true,
            },
            Track {
                params: &cpar,
                time_base: tb(48_000),
                packets: &mic_pk,
                title: "Mikrofon",
                default: false,
            },
        ],
        &[("gameclip_tag_id", "0b6f2c1e"), ("title", "Testclip")],
        0,
        true,
    )
    .unwrap();

    let info = ffprobe(&path, &["-show_streams", "-show_format"]);
    let streams = info["streams"].as_array().unwrap();
    assert_eq!(streams.len(), 3);
    let titles: Vec<&str> = streams
        .iter()
        .map(|s| s["tags"]["handler_name"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Video", "Mischung", "Mikrofon"]);
    assert_eq!(streams[1]["disposition"]["default"], 1);
    assert_eq!(streams[2]["disposition"]["default"], 0);
    assert_eq!(info["format"]["tags"]["gameclip_tag_id"], "0b6f2c1e");
    assert_eq!(info["format"]["tags"]["title"], "Testclip");

    let duration = f(&info["format"]["duration"]);
    assert!((duration - SECONDS as f64).abs() < 0.1, "Dauer {duration}");
    let v_start = f(&streams[0]["start_time"]);
    let a_start = f(&streams[1]["start_time"]);
    println!("Dauer {duration}, Start Video {v_start}, Ton {a_start}");
    // A/V-Versatz kleiner als ein Bild; die AAC-Anlaufzeit ist über die Edit-List ausgeblendet.
    assert!((v_start - a_start).abs() < 1.0 / f64::from(FPS));

    let first = ffprobe(
        &path,
        &[
            "-select_streams",
            "v",
            "-show_packets",
            "-read_intervals",
            "%+#1",
        ],
    );
    assert!(
        first["packets"][0]["flags"]
            .as_str()
            .unwrap()
            .starts_with('K'),
        "erstes Bild ist Keyframe"
    );
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn m4a_holds_separate_tracks() {
    let (sys, sys_pk) = encode_audio(440.0);
    let (mic, mic_pk) = encode_audio(660.0);
    let (spar, mpar) = (sys.codec_parameters(), mic.codec_parameters());
    let path = temp_file("tracks.m4a.part");
    write_m4a(
        &path,
        &[
            Track {
                params: &spar,
                time_base: tb(48_000),
                packets: &sys_pk,
                title: "System",
                default: true,
            },
            Track {
                params: &mpar,
                time_base: tb(48_000),
                packets: &mic_pk,
                title: "Mikrofon",
                default: false,
            },
        ],
        &[("gameclip_tag_id", "0b6f2c1e")],
        0,
    )
    .unwrap();
    let info = ffprobe(&path, &["-show_streams", "-show_format"]);
    assert_eq!(
        info["format"]["format_name"].as_str().unwrap(),
        "mov,mp4,m4a,3gp,3g2,mj2"
    );
    let titles: Vec<&str> = info["streams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["tags"]["handler_name"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["System", "Mikrofon"]);
    assert_eq!(info["format"]["tags"]["gameclip_tag_id"], "0b6f2c1e");
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn t0_shifts_clip_to_zero() {
    // Clip beginnt am ersten Keyframe ab 4 s, wie ein Schnappschuss des Ringpuffers.
    // (x264 setzt bei Szenenwechseln zusätzliche Keyframes, die Lage ist also nicht fest.)
    let (venc, vpk) = encode_video();
    let (aenc, apk) = encode_audio(440.0);
    let t0_us = vpk
        .iter()
        .find(|p| p.pts >= 4_000_000 && p.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0)
        .unwrap()
        .pts;
    let expected = (SECONDS * 1_000_000 - t0_us) as f64 / 1e6;
    let video: Vec<AVPacket> = vpk.into_iter().filter(|p| p.pts >= t0_us).collect();
    let t0_samples = t0_us * 48_000 / 1_000_000;
    // Ein Paket vor t0 mitnehmen, damit der Ton ab dem ersten Bild vollständig ist.
    let audio: Vec<AVPacket> = apk
        .into_iter()
        .filter(|p| p.pts + 1024 > t0_samples)
        .collect();
    let (vpar, apar) = (venc.codec_parameters(), aenc.codec_parameters());
    let path = temp_file("shifted.mp4.part");
    write_mp4(
        &path,
        &Track {
            params: &vpar,
            time_base: tb(TIME_BASE_DEN),
            packets: &video,
            title: "Video",
            default: true,
        },
        &[Track {
            params: &apar,
            time_base: tb(48_000),
            packets: &audio,
            title: "Mischung",
            default: true,
        }],
        &[],
        t0_us,
        false,
    )
    .unwrap();
    let info = ffprobe(&path, &["-show_streams", "-show_format"]);
    let duration = f(&info["format"]["duration"]);
    let v_start = f(&info["streams"][0]["start_time"]);
    let a_start = f(&info["streams"][1]["start_time"]);
    println!("verschoben: Dauer {duration}, Start Video {v_start}, Ton {a_start}");
    assert!(
        (duration - expected).abs() < 0.1,
        "Dauer {duration}, erwartet {expected}"
    );
    assert!(v_start.abs() < 0.001);
    assert!(a_start.abs() < 1.0 / f64::from(FPS));
    std::fs::remove_file(&path).unwrap();
}
