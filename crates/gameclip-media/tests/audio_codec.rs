// SPDX-License-Identifier: GPL-3.0-or-later
//! AAC-Encoder: Blockbildung aus unregelmäßigen Eingaben, PTS in Samples, letzter kurzer Block.
#![cfg(windows)]

use gameclip_media::AudioEncoder;
use gameclip_media::audio_codec::SAMPLE_RATE;

/// Interleaved Stereo-Sinus, `frames` Sample-Paare ab Sample `start`.
fn sine(start: usize, frames: usize) -> Vec<f32> {
    (start..start + frames)
        .flat_map(|i| {
            let s = (i as f32 * 440.0 * std::f32::consts::TAU / SAMPLE_RATE as f32).sin() * 0.5;
            [s, s]
        })
        .collect()
}

#[test]
fn irregular_chunks_become_1024_sample_packets() {
    let mut enc = AudioEncoder::new(160_000).unwrap();
    assert_eq!(enc.frame_size(), 1024);
    assert_eq!(enc.initial_padding(), 1024);

    // 1 s in Blöcken von 480 Samples (10 ms, wie WASAPI) plus ein unrunder Rest.
    let total = SAMPLE_RATE as usize + 300;
    let mut packets = Vec::new();
    let mut pos = 0;
    while pos < total {
        let n = 480.min(total - pos);
        packets.extend(enc.push(&sine(pos, n)).unwrap());
        pos += n;
    }
    packets.extend(enc.finish().unwrap());

    let pts: Vec<i64> = packets.iter().map(|p| p.pts).collect();
    let duration: i64 = packets.iter().map(|p| p.duration).sum();
    println!(
        "{} Pakete, erste PTS {:?}, Dauer {duration}",
        packets.len(),
        &pts[..3]
    );
    // Der Encoder verschiebt um die Anlaufverzögerung: erstes Paket bei -1024.
    assert_eq!(pts[0], -1024);
    assert!(pts.windows(2).all(|w| w[1] - w[0] == 1024));
    // Genau alle Eingabe-Samples plus die Anlaufverzögerung.
    assert_eq!(duration, total as i64 + enc.initial_padding());
    assert!(packets.iter().all(|p| p.size > 0));
}

#[test]
fn bitrate_roughly_matches() {
    let mut enc = AudioEncoder::new(160_000).unwrap();
    let mut bytes = 0usize;
    for i in 0..500 {
        for p in enc.push(&sine(i * 480, 480)).unwrap() {
            bytes += p.size as usize;
        }
    }
    for p in enc.finish().unwrap() {
        bytes += p.size as usize;
    }
    // 500 × 10 ms = 5 s bei 160 kbit/s ≈ 100 000 Byte.
    let kbps = bytes as f64 * 8.0 / 5.0 / 1000.0;
    println!("{kbps:.0} kbit/s");
    assert!((100.0..220.0).contains(&kbps), "{kbps} kbit/s");
}
