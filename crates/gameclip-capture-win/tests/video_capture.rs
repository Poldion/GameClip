// SPDX-License-Identifier: GPL-3.0-or-later
//! Echte Aufnahme des Hauptmonitors über 2 s. Braucht eine Desktop-Sitzung und eine GPU.
//! Die Testdatei enthält den Bildschirminhalt und wird am Ende gelöscht.
#![cfg(windows)]

use std::time::{Duration, Instant};

use gameclip_capture_win::{VideoConfig, VideoEvent, VideoPipeline};
use gameclip_gpu::MonitorTarget;
use gameclip_media::demux::frame_times_us;
use gameclip_media::mux::{Track, write_mp4};
use rsmpeg::ffi;

const FPS: u32 = 60;

#[test]
fn captures_primary_monitor_at_fixed_rate() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let cfg = VideoConfig {
        monitor: MonitorTarget::Primary,
        width: 1920,
        height: 1080,
        fps: FPS,
        bitrate_bps: 12_000_000,
        cursor: true,
    };
    let pipeline = VideoPipeline::start(cfg, tx).unwrap();
    let info = pipeline.info();
    println!(
        "Encoder {} (Hardware: {}), Adapter {}, Rahmen entfernt: {}, HDR: {}",
        info.encoder, info.hardware, info.adapter, info.border_removed, info.hdr_active
    );

    let start = Instant::now();
    let mut forced_at = None;
    while start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(50));
        // 1,4 s: zwischen den regulären Keyframes bei 1 s und 2 s.
        if forced_at.is_none() && start.elapsed() >= Duration::from_millis(1400) {
            pipeline.force_keyframe();
            forced_at = Some(gameclip_capture_win::pacer::qpc_us());
        }
    }
    let params = info.params.clone();
    let stats = pipeline.stop();
    println!("{stats:?}");

    let mut packets = Vec::new();
    for ev in rx.try_iter() {
        match ev {
            VideoEvent::Packet(p) => packets.push(p),
            VideoEvent::DeviceLost(e) => panic!("Gerät verloren: {e}"),
            VideoEvent::HdrChanged(h) => println!("HDR geändert: {h}"),
            VideoEvent::FormatChanged { width, height } => println!("Format {width}×{height}"),
        }
    }
    assert!(
        packets.len() >= 110 && packets.len() <= 125,
        "{} Pakete in 2 s",
        packets.len()
    );
    assert!(packets[0].keyframe, "erstes Paket ist ein Keyframe");
    assert_eq!(packets.len() as u64, stats.ticks, "ein Paket je Takt");

    // Festes Raster: Abstände 16 666/16 667 µs, außer bei verpassten Takten (Vielfache).
    for w in packets.windows(2) {
        let d = w[1].pts_us - w[0].pts_us;
        let frames = (d as f64 / (1e6 / f64::from(FPS))).round();
        assert!(
            frames >= 1.0 && (d as f64 - frames * 1e6 / f64::from(FPS)).abs() <= 1.0,
            "Abstand {d} µs"
        );
    }
    let forced = forced_at.unwrap();
    let keys: Vec<i64> = packets
        .iter()
        .filter(|p| p.keyframe)
        .map(|p| p.pts_us - packets[0].pts_us)
        .collect();
    println!("Keyframes bei µs {keys:?}");
    assert!(keys.contains(&1_000_000), "regulärer Keyframe nach 1 s");
    assert!(
        packets
            .iter()
            .any(|p| p.keyframe && p.pts_us >= forced && p.pts_us < forced + 100_000),
        "IDR kurz nach force_keyframe"
    );

    // Abspielbare Datei: jedes Paket wird zu einem Bild.
    let path =
        std::env::temp_dir().join(format!("gameclip-capture-test-{}.mp4", std::process::id()));
    let t0 = packets[0].pts_us;
    let raw: Vec<_> = packets.into_iter().map(|p| p.packet).collect();
    write_mp4(
        &path,
        &Track {
            params: &params,
            time_base: ffi::AVRational {
                num: 1,
                den: 1_000_000,
            },
            packets: &raw,
            title: "Video",
            default: true,
        },
        &[],
        &[],
        t0,
        false,
    )
    .unwrap();
    let frames = frame_times_us(&path).unwrap().len();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(frames, raw.len());
}
