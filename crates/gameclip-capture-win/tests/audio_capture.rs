// SPDX-License-Identifier: GPL-3.0-or-later
//! Echte Tonaufnahme über 3 s: System per Prozess-Loopback (ein Hilfsprozess ausgeschlossen),
//! Mikrofon falls vorhanden. Ein leiser 1-kHz-Ton (ffplay, 1 s) prüft, dass der Systemton
//! ankommt und zur richtigen Zeit auf der Zeitachse liegt.
#![cfg(windows)]

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use gameclip_capture_win::audio::mic;
use gameclip_capture_win::pacer::qpc_us;
use gameclip_capture_win::{AudioConfig, AudioEvent, AudioPipeline, MicConfig, Track};
use gameclip_media::demux::read_tags;
use gameclip_media::mux::{Track as MuxTrack, write_m4a};
use rsmpeg::ffi;

#[test]
fn records_system_and_mic_on_one_timeline() {
    let has_mic = mic::list_devices().map(|d| !d.is_empty()).unwrap_or(false);
    println!("Mikrofone: {:?}", mic::list_devices());
    // Ausgeschlossen wird ein untätiger Hilfsprozess, nicht der Test selbst: EXCLUDE gilt für den
    // ganzen Prozessbaum, und ffplay ist ein Kind des Tests (sein Ton fehlte sonst zu Recht).
    let mut idle = Command::new("cmd")
        .args(["/c", "ping -n 8 127.0.0.1 >nul"])
        .spawn()
        .expect("Hilfsprozess startet");
    let (tx, rx) = crossbeam_channel::unbounded();
    let pipeline = AudioPipeline::start(
        AudioConfig {
            exclude_pid: idle.id(),
            system_gain_db: 0.0,
            mic: has_mic.then_some(MicConfig {
                device_id: None,
                gain_db: 0.0,
            }),
        },
        tx,
    )
    .unwrap();
    let start_us = pipeline.start_us();

    let ffplay =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ffmpeg/bin/ffplay.exe");
    let begin = Instant::now();
    std::thread::sleep(Duration::from_millis(800));
    let tone_start = qpc_us();
    let mut player = Command::new(ffplay)
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-volume", "4"])
        .args(["-f", "lavfi", "-i", "sine=frequency=1000:duration=1"])
        .spawn()
        .expect("ffplay startet");
    while begin.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = player.wait();
    let params = pipeline.codec_parameters().to_vec();
    pipeline.stop();
    let _ = idle.kill();
    let _ = idle.wait();

    let mut packets: Vec<(Track, rsmpeg::avcodec::AVPacket, i64)> = Vec::new();
    let mut levels = Vec::new();
    for ev in rx.try_iter() {
        match ev {
            AudioEvent::Packet(p) => packets.push((p.track, p.packet, p.pts_us)),
            AudioEvent::Levels {
                time_us,
                system_dbfs,
                mic_dbfs,
            } => levels.push((time_us - start_us, system_dbfs, mic_dbfs)),
            AudioEvent::SourceFailed { track, error } => panic!("{track:?} ausgefallen: {error}"),
        }
    }

    let tracks: Vec<Track> = params.iter().map(|(t, _)| *t).collect();
    for track in &tracks {
        let pts: Vec<i64> = packets
            .iter()
            .filter(|p| p.0 == *track)
            .map(|p| p.2)
            .collect();
        println!(
            "{track:?}: {} Pakete, erstes bei {} µs",
            pts.len(),
            pts[0] - start_us
        );
        // ~3 s abzüglich 100 ms Mixer-Verzögerung → etwa 135 Pakete zu 1024 Samples.
        assert!(
            (115..=145).contains(&pts.len()),
            "{track:?}: {} Pakete",
            pts.len()
        );
        // Erstes Paket: AAC-Anlauf 1024 Samples vor dem Nullpunkt.
        assert_eq!(pts[0] - start_us, -21_333);
        assert!(pts.windows(2).all(|w| (w[1] - w[0] - 21_333).abs() <= 1));
    }

    let tone_us = tone_start - start_us;
    let during: Vec<f32> = levels
        .iter()
        .filter(|(t, ..)| *t > tone_us + 200_000 && *t < tone_us + 800_000)
        .map(|l| l.1)
        .collect();
    let before: Vec<f32> = levels
        .iter()
        .filter(|(t, ..)| *t < tone_us)
        .map(|l| l.1)
        .collect();
    println!("Ton ab {} ms", tone_us / 1000);
    println!(
        "Alle Pegel (ms, dBFS): {:?}",
        levels
            .iter()
            .map(|l| (l.0 / 1000, l.1 as i32))
            .collect::<Vec<_>>()
    );
    println!("System-Pegel vor dem Ton {before:?}");
    println!("System-Pegel während des Tons {during:?}");
    let loudest = during.iter().cloned().fold(-100.0f32, f32::max);
    assert!(loudest > -60.0, "Ton nicht im Systemton: {loudest} dBFS");
    // Der Sinus dauert 1 s: so viele 100-ms-Pegel liegen auf seinem Niveau (±3 dB), und zwar
    // zusammenhängend nach dem Start von ffplay.
    let tone: Vec<i64> = levels
        .iter()
        .filter(|(t, s, _)| *t >= tone_us && (s - loudest).abs() <= 3.0)
        .map(|l| l.0 / 100_000)
        .collect();
    println!("Tonblöcke (×100 ms): {tone:?}");
    assert!(
        (9..=11).contains(&tone.len()),
        "Tondauer {} × 100 ms",
        tone.len()
    );
    assert!(
        tone.windows(2).all(|w| w[1] - w[0] == 1),
        "Ton zusammenhängend"
    );

    // Einzelspuren lassen sich als M4A schreiben.
    let path = std::env::temp_dir().join(format!("gameclip-audio-test-{}.m4a", std::process::id()));
    let mut by_track: Vec<(Track, Vec<rsmpeg::avcodec::AVPacket>)> = tracks
        .iter()
        .filter(|t| **t != Track::Mix)
        .map(|t| (*t, Vec::new()))
        .collect();
    for (track, packet, _) in packets {
        if let Some((_, list)) = by_track.iter_mut().find(|(t, _)| *t == track) {
            list.push(packet);
        }
    }
    let mux_tracks: Vec<MuxTrack> = by_track
        .iter()
        .map(|(t, pk)| MuxTrack {
            params: &params.iter().find(|(pt, _)| pt == t).unwrap().1,
            time_base: ffi::AVRational {
                num: 1,
                den: 48_000,
            },
            packets: pk,
            title: t.title(),
            default: *t == Track::System,
        })
        .collect();
    write_m4a(&path, &mux_tracks, &[("gameclip_id", "test")], 0).unwrap();
    assert_eq!(read_tags(&path).unwrap()["gameclip_id"], "test");
    std::fs::remove_file(&path).unwrap();
}
