// SPDX-License-Identifier: GPL-3.0-or-later
//! Echte Tonaufnahme: System per Prozess-Loopback (ein Hilfsprozess ausgeschlossen), Mikrofon
//! falls vorhanden.
//!
//! * `records_system_and_mic_on_one_timeline` prüft Spuren, Paketzahl, Raster und Nullpunkt.
//! * `system_tone_lands_on_timeline` (ignoriert) spielt einen leisen 1-kHz-Ton über ffplay und
//!   sucht ihn frequenzgenau in der dekodierten Spur „System“. Er braucht einen ruhigen Rechner
//!   mit Lautsprechern: Auf dem Messrechner kam der Ton bei laufendem Spiel nur in einem von
//!   drei Läufen im Loopback an (offen in S4). Start: `cargo test -- --ignored`.
#![cfg(windows)]

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use gameclip_capture_win::audio::mic;
use gameclip_capture_win::pacer::qpc_us;
use gameclip_capture_win::{AudioConfig, AudioEvent, AudioPipeline, MicConfig, Track};
use gameclip_media::demux::read_tags;
use gameclip_media::mux::{Track as MuxTrack, write_m4a};
use rsmpeg::avcodec::{AVCodecParameters, AVPacket};
use rsmpeg::ffi;

struct Recording {
    start_us: i64,
    /// QPC-µs, zu der ffplay gestartet wurde.
    tone_start_us: Option<i64>,
    recorded: Duration,
    params: Vec<(Track, AVCodecParameters)>,
    packets: Vec<(Track, AVPacket, i64)>,
}

fn record(play_tone: bool) -> Recording {
    record_with(play_tone, false)
}

fn record_with(play_tone: bool, discord: bool) -> Recording {
    let has_mic = mic::list_devices().map(|d| !d.is_empty()).unwrap_or(false);
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
            discord_gain_db: discord.then_some(0.0),
        },
        tx,
    )
    .unwrap();
    let start_us = pipeline.start_us();
    let begin = Instant::now();
    std::thread::sleep(Duration::from_millis(800));
    let mut tone_start_us = None;
    if play_tone {
        let ffplay =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../third_party/ffmpeg/bin/ffplay.exe");
        tone_start_us = Some(qpc_us());
        let mut player = Command::new(ffplay)
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-volume", "4"])
            .args(["-f", "lavfi", "-i", "sine=frequency=1000:duration=1"])
            .spawn()
            .expect("ffplay startet");
        // Auf das Ende von ffplay warten (Startzeit schwankt), dann noch 300 ms aufnehmen.
        let _ = player.wait();
        std::thread::sleep(Duration::from_millis(300));
    } else {
        std::thread::sleep(Duration::from_millis(2_000));
    }
    let recorded = begin.elapsed();
    let params = pipeline.codec_parameters().to_vec();
    pipeline.stop();
    let _ = idle.kill();
    let _ = idle.wait();

    let mut packets = Vec::new();
    for ev in rx.try_iter() {
        match ev {
            AudioEvent::Packet(p) => packets.push((p.track, p.packet, p.pts_us)),
            AudioEvent::Levels { .. } => {}
            AudioEvent::SourceFailed { track, error } => panic!("{track:?} ausgefallen: {error}"),
        }
    }
    Recording {
        start_us,
        tone_start_us,
        recorded,
        params,
        packets,
    }
}

#[test]
fn records_system_and_mic_on_one_timeline() {
    println!("Mikrofone: {:?}", mic::list_devices());
    let r = record(false);
    let tracks: Vec<Track> = r.params.iter().map(|(t, _)| *t).collect();
    assert!(tracks.contains(&Track::Mix) && tracks.contains(&Track::System));
    for track in &tracks {
        let pts: Vec<i64> = r
            .packets
            .iter()
            .filter(|p| p.0 == *track)
            .map(|p| p.2)
            .collect();
        println!(
            "{track:?}: {} Pakete, erstes bei {} µs",
            pts.len(),
            pts[0] - r.start_us
        );
        // Aufnahmedauer abzüglich 100 ms Mixer-Verzögerung, 1024 Samples je Paket (±10 %).
        let expected = (r.recorded.as_secs_f64() - 0.1) * 48_000.0 / 1024.0;
        assert!(
            (pts.len() as f64 - expected).abs() <= expected * 0.1 + 3.0,
            "{track:?}: {} Pakete, erwartet ≈ {expected:.0}",
            pts.len()
        );
        // Erstes Paket: AAC-Anlauf 1024 Samples vor dem Nullpunkt.
        assert_eq!(pts[0] - r.start_us, -21_333);
        assert!(pts.windows(2).all(|w| (w[1] - w[0] - 21_333).abs() <= 1));
    }

    // Jede Spur lässt sich dekodieren.
    for (track, params) in &r.params {
        let pcm = decode_left(
            params,
            r.packets.iter().filter(|p| p.0 == *track).map(|p| &p.1),
        );
        assert!(pcm.len() > 48_000, "{track:?}: nur {} Samples", pcm.len());
    }

    // Einzelspuren lassen sich als M4A schreiben.
    let path = std::env::temp_dir().join(format!("gameclip-audio-test-{}.m4a", std::process::id()));
    let mut by_track: Vec<(Track, Vec<AVPacket>)> = tracks
        .iter()
        .filter(|t| **t != Track::Mix)
        .map(|t| (*t, Vec::new()))
        .collect();
    for (track, packet, _) in r.packets {
        if let Some((_, list)) = by_track.iter_mut().find(|(t, _)| *t == track) {
            list.push(packet);
        }
    }
    let mux_tracks: Vec<MuxTrack> = by_track
        .iter()
        .map(|(t, pk)| MuxTrack {
            params: &r.params.iter().find(|(pt, _)| pt == t).unwrap().1,
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

#[test]
#[ignore = "braucht ruhigen Rechner mit Lautsprechern; spielt einen leisen Ton (S4)"]
fn system_tone_lands_on_timeline() {
    let r = record(true);
    let tone_block = (r.tone_start_us.unwrap() - r.start_us) / 100_000;
    // Frequenzgenau statt über den Pegel: Leistung bei 1 kHz je 100 ms (Goertzel). Ein reiner
    // Sinus hat gleichbleibende Leistung: Tonblöcke liegen höchstens 3 dB unter dem Maximum, und
    // das Maximum muss 20 dB über dem Grund vor dem Ton liegen.
    let sys = &r
        .params
        .iter()
        .find(|(t, _)| *t == Track::System)
        .unwrap()
        .1;
    let pcm = decode_left(
        sys,
        r.packets
            .iter()
            .filter(|p| p.0 == Track::System)
            .map(|p| &p.1),
    );
    let power: Vec<f64> = pcm
        .as_chunks::<4_800>()
        .0
        .iter()
        .map(|w| goertzel(w, 1_000.0))
        .collect();
    println!(
        "1-kHz-Leistung je Block (dB): {:?}",
        power
            .iter()
            .map(|p| (10.0 * p.max(1e-20).log10()) as i32)
            .collect::<Vec<_>>()
    );
    let mut quiet: Vec<f64> = power[..tone_block as usize].to_vec();
    quiet.sort_by(f64::total_cmp);
    let floor = quiet[quiet.len() / 2].max(1e-12);
    let peak = power.iter().cloned().fold(0.0f64, f64::max);
    assert!(peak > floor * 100.0, "kein 1-kHz-Ton über dem Grund");
    let tone: Vec<i64> = (0..power.len() as i64)
        .filter(|&i| power[i as usize] >= peak / 2.0)
        .collect();
    println!("Ton gestartet bei Block {tone_block}; 1-kHz-Blöcke (×100 ms): {tone:?}");
    let mut runs: Vec<(i64, i64)> = Vec::new();
    for &b in &tone {
        match runs.last_mut() {
            Some((_, end)) if *end + 1 == b => *end = b,
            _ => runs.push((b, b)),
        }
    }
    let (first, last) = runs.iter().copied().max_by_key(|(a, b)| b - a).unwrap();
    let len = last - first + 1;
    // Der Sinus dauert 1 s; ffplay braucht bis zu 500 ms zum Starten.
    assert!((9..=11).contains(&len), "Tondauer {len} × 100 ms");
    assert!(
        (tone_block..=tone_block + 5).contains(&first),
        "Ton beginnt bei Block {first}, gestartet bei {tone_block}"
    );
}

/// Dekodiert AAC-Pakete (Zeitbasis Samples) und liefert den linken Kanal ab Sample 0;
/// das Anlaufpaket mit negativer PTS fällt weg.
fn decode_left<'a>(
    params: &AVCodecParameters,
    packets: impl Iterator<Item = &'a AVPacket>,
) -> Vec<f32> {
    use rsmpeg::avcodec::{AVCodec, AVCodecContext};
    let codec = AVCodec::find_decoder(ffi::AV_CODEC_ID_AAC).unwrap();
    let mut ctx = AVCodecContext::new(&codec);
    ctx.apply_codecpar(params).unwrap();
    ctx.set_pkt_timebase(ffi::AVRational {
        num: 1,
        den: 48_000,
    });
    ctx.open(None).unwrap();
    let mut out = Vec::new();
    let mut take = |ctx: &mut AVCodecContext| {
        while let Ok(f) = ctx.receive_frame() {
            if f.pts < 0 {
                continue;
            }
            let left = unsafe {
                std::slice::from_raw_parts(f.data[0] as *const f32, f.nb_samples as usize)
            };
            out.extend_from_slice(left);
        }
    };
    for p in packets {
        ctx.send_packet(Some(p)).unwrap();
        take(&mut ctx);
    }
    ctx.send_packet(None).unwrap();
    take(&mut ctx);
    out
}

/// Leistung bei `freq` in einem Block zu 48 kHz (Goertzel).
fn goertzel(samples: &[f32], freq: f64) -> f64 {
    let k = 2.0 * (std::f64::consts::TAU * freq / 48_000.0).cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &x in samples {
        let s = f64::from(x) + k * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    (s1 * s1 + s2 * s2 - k * s1 * s2) / (samples.len() as f64).powi(2)
}

/// Mit Discord-Spur: vier Spuren auf derselben Zeitachse. Läuft Discord nicht, ist die Spur still,
/// aber vollständig; läuft es, schließt „System“ Discord aus statt des Hilfsprozesses.
#[test]
fn records_discord_track_on_the_same_timeline() {
    let root = gameclip_capture_win::audio::discord::find_root();
    println!("Discord-Wurzelprozess: {root:?}");
    let r = record_with(false, true);
    let tracks: Vec<Track> = r.params.iter().map(|(t, _)| *t).collect();
    assert!(tracks.contains(&Track::Discord), "{tracks:?}");
    let count = |t: Track| r.packets.iter().filter(|p| p.0 == t).count();
    let (mix, discord) = (count(Track::Mix), count(Track::Discord));
    assert!(
        discord > 0 && discord.abs_diff(mix) <= 2,
        "Mischung {mix}, Discord {discord}"
    );
    let first = r.packets.iter().find(|p| p.0 == Track::Discord).unwrap().2;
    assert_eq!(first - r.start_us, -21_333);
}
