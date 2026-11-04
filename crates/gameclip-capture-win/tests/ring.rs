// SPDX-License-Identifier: GPL-3.0-or-later
//! Ringpuffer mit synthetischen Paketen (Design, Datei 39). Jedes Paket trägt seine PTS als
//! Inhalt, damit sich prüfen lässt, dass gehaltene Daten nicht überschrieben werden.
#![cfg(windows)]

use std::sync::Arc;
use std::time::Duration;

use gameclip_capture_win::Track;
use gameclip_capture_win::ring::{PacketRef, PacketSource, PushOutcome, Ring, RingConfig, Stream};

/// Zeit von Bild `n` bei 60 fps, exakt wie der Taktgeber.
fn frame_time(n: i64) -> i64 {
    n * 1_000_000 / 60
}
const AUDIO_US: i64 = 21_333;
const SECOND: i64 = 1_000_000;

fn ring(blocks: usize, block_bytes: usize, seconds: i64) -> Ring {
    Ring::new(RingConfig {
        pool_blocks: blocks,
        duration_us: seconds * SECOND,
        block_bytes,
    })
}

fn payload(pts: i64, len: usize) -> Vec<u8> {
    let mut v = pts.to_le_bytes().to_vec();
    v.resize(len.max(8), (pts % 251) as u8);
    v
}

fn pts_of(p: &PacketRef) -> i64 {
    i64::from_le_bytes(p.data()[..8].try_into().unwrap())
}

/// Schreibt Video (60 fps, Keyframe je Sekunde) und Ton (Mischung) von `from_us` bis `to_us`.
/// Liefert alle Ergebnisse der Videopakete.
fn feed(r: &Ring, from_us: i64, to_us: i64, video_len: usize) -> Vec<(i64, PushOutcome)> {
    let mut out = Vec::new();
    let mut next_audio = from_us - from_us % AUDIO_US;
    let mut n = (from_us * 60 + 999_999) / 1_000_000;
    loop {
        let pts = frame_time(n);
        if pts >= to_us {
            break;
        }
        while next_audio <= pts {
            r.push(
                Stream::Audio(Track::Mix),
                next_audio,
                next_audio,
                true,
                &payload(next_audio, 50),
            );
            next_audio += AUDIO_US;
        }
        let key = n % 60 == 0;
        out.push((
            pts,
            r.push(Stream::Video, pts, pts, key, &payload(pts, video_len)),
        ));
        n += 1;
    }
    out
}

#[test]
fn evicts_only_whole_gops_by_time() {
    let r = ring(64, 64 * 1024, 5);
    feed(&r, 0, 10 * SECOND, 200);
    let s = r.snapshot(0, i64::MAX);
    assert!(s.video[0].keyframe, "älteste Bildgruppe vollständig");
    let newest = s.video.last().unwrap().pts_us;
    let span = newest - s.t0_us;
    println!("Puffer {} … {} µs ({} µs)", s.t0_us, newest, span);
    assert!((5 * SECOND..6 * SECOND).contains(&span), "Spanne {span}");
    // Keyframes liegen auf ganzen Sekunden.
    assert_eq!(s.t0_us % SECOND, 0);
    for p in &s.video {
        assert_eq!(pts_of(p), p.pts_us);
    }
}

#[test]
fn audio_matches_first_keyframe() {
    let r = ring(64, 64 * 1024, 5);
    feed(&r, 0, 10 * SECOND, 200);
    let s = r.snapshot(0, i64::MAX);
    let (track, audio) = &s.audio[0];
    assert_eq!(*track, Track::Mix);
    let first = audio[0].pts_us;
    println!("t0 {} µs, erstes Tonpaket {first} µs", s.t0_us);
    // Das erste Tonpaket überdeckt t0, keins endet vorher.
    assert!(first <= s.t0_us && first + AUDIO_US > s.t0_us - 1);
    assert!(
        audio
            .windows(2)
            .all(|w| w[1].pts_us - w[0].pts_us == AUDIO_US)
    );
}

#[test]
fn snapshot_survives_continued_recording() {
    let r = ring(16, 64 * 1024, 5);
    feed(&r, 0, 6 * SECOND, 2_000);
    let s = r.snapshot(2 * SECOND, 6 * SECOND);
    assert_eq!(s.t0_us, 2 * SECOND);
    // 30 s weiterschreiben: der Pool wird mehrfach umgewälzt, der Schnappschuss bleibt.
    feed(&r, 6 * SECOND, 36 * SECOND, 2_000);
    for p in s.video.iter().chain(s.audio[0].1.iter()) {
        assert_eq!(pts_of(p), p.pts_us, "Daten überschrieben");
    }
}

#[test]
fn full_pool_with_held_snapshot_makes_a_gap() {
    // 12 Blöcke zu 64 KiB = 768 KiB; Video 2000 Byte/Bild ≈ 120 KB/s → ~6 s Platz, davon hält
    // der Schnappschuss ~3 s. (Ist der Pool kleiner als Schnappschuss plus zwei Bildgruppen,
    // kommt die Aufnahme erst nach dessen Freigabe wieder in Gang; die Reserve verhindert das.)
    let r = ring(12, 64 * 1024, 60);
    feed(&r, 0, 3 * SECOND, 2_000);
    let held = r.snapshot(0, 3 * SECOND);
    // Kurz sehr große Bilder (40 KB): die laufende Bildgruppe passt nicht mehr in den freien Rest.
    let mut results = feed(&r, 3 * SECOND, 3 * SECOND + 500_000, 40_000);
    // Danach nur so viel, wie neben dem Schnappschuss Platz hat (sonst verdrängt das Verwerfen
    // nach Platz die Bildgruppe an der Lücke, was ebenfalls richtig wäre).
    results.extend(feed(&r, 3 * SECOND + 500_000, 6 * SECOND, 2_000));

    let need: Vec<i64> = results
        .iter()
        .filter(|(_, o)| *o == PushOutcome::NeedKeyframe)
        .map(|(p, _)| *p)
        .collect();
    assert_eq!(need.len(), 1, "genau eine IDR-Anforderung je Lücke");
    let gap_start = need[0];
    // Bis zum nächsten Keyframe wird verworfen, dann geht es weiter.
    let resumed = results
        .iter()
        .find(|(p, o)| *p > gap_start && *o == PushOutcome::Stored)
        .unwrap()
        .0;
    println!("Lücke ab {gap_start} µs, weiter ab {resumed} µs");
    assert_eq!(resumed % SECOND, 0, "weiter erst mit einem Keyframe");
    let stats = r.stats();
    assert_eq!(stats.gaps, 1);
    assert!(stats.dropped_packets > 0);

    // Ein Schnappschuss über die Lücke beginnt an ihr.
    let after = r.snapshot(0, 6 * SECOND);
    assert!(after.video[0].gap_before);
    assert_eq!(after.t0_us, resumed);
    assert!(after.truncated);
    // Ton aus der Lücke fehlt ebenfalls: kein Tonpaket vor dem Neubeginn.
    assert!(after.audio[0].1[0].pts_us + AUDIO_US > resumed);

    // Der gehaltene Schnappschuss ist unversehrt.
    for p in &held.video {
        assert_eq!(pts_of(p), p.pts_us);
    }
    drop(held);
    let results = feed(&r, 6 * SECOND, 14 * SECOND, 2_000);
    assert!(results.iter().all(|(_, o)| *o == PushOutcome::Stored));
}

#[test]
fn packets_cross_blocks_and_oversize_gets_own_allocation() {
    let r = ring(8, 1_000, 60);
    // 700 Byte: jedes Paket beginnt in einem neuen Block.
    for i in 0..5 {
        let pts = frame_time(i);
        assert_eq!(
            r.push(Stream::Video, pts, pts, i == 0, &payload(pts, 700)),
            PushOutcome::Stored
        );
    }
    let big = frame_time(5);
    assert_eq!(
        r.push(Stream::Video, big, big, false, &payload(big, 5_000)),
        PushOutcome::Stored
    );
    let stats = r.stats();
    assert_eq!(stats.oversize_packets, 1);
    assert_eq!(stats.pool_blocks - stats.free_blocks, 5);
    let s = r.snapshot(0, i64::MAX);
    assert_eq!(s.video.len(), 6);
    for p in &s.video {
        assert_eq!(pts_of(p), p.pts_us);
    }
    assert_eq!(s.video[5].data().len(), 5_000);
}

#[test]
fn snapshot_uses_only_newest_section() {
    let r = ring(64, 64 * 1024, 60);
    feed(&r, 0, 3 * SECOND, 200);
    r.new_section();
    feed(&r, 5 * SECOND, 7 * SECOND, 200);
    let s = r.snapshot(0, 7 * SECOND);
    assert_eq!(s.t0_us, 5 * SECOND);
    assert!(s.truncated);
    assert!(
        s.audio[0]
            .1
            .iter()
            .all(|p| p.pts_us + AUDIO_US > 5 * SECOND)
    );
}

#[test]
fn wait_for_returns_when_packet_arrives() {
    let r = Arc::new(ring(8, 64 * 1024, 60));
    assert!(!r.wait_for(SECOND, Duration::from_millis(20)));
    let writer = {
        let r = r.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            r.push(Stream::Video, SECOND, SECOND, true, &payload(SECOND, 10));
        })
    };
    assert!(r.wait_for(SECOND, Duration::from_secs(2)));
    writer.join().unwrap();
}
