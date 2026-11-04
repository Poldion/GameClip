// SPDX-License-Identifier: GPL-3.0-or-later
//! Mischung mit Lautstärke je Quelle und Limiter (Design, Datei 37).
//!
//! Der Limiter schaut 5 ms voraus. Damit sich die Zeitstempel nicht verschieben, gibt der Mixer
//! jeden Block einen Block später aus: Block k wird mit Blick auf Block k+1 begrenzt. Die
//! Einzelspuren laufen mit derselben Verzögerung, damit alle Spuren dieselben Blöcke tragen.

use super::resample::{BLOCK, CHANNELS};

pub type Block = [f32; BLOCK * CHANNELS];

/// Schwelle des Limiters: −1 dBFS *(vorläufig)*.
const THRESHOLD: f32 = 0.891_250_9;
/// Vorausschau in Frames (5 ms).
const LOOKAHEAD: usize = 240;
/// Erholung je Frame (≈ 50 ms Zeitkonstante).
const RELEASE: f32 = 1.0 / 2400.0;

pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// RMS eines Blocks in dBFS (−100 bei Stille).
pub fn rms_dbfs(block: &Block) -> f32 {
    let sum: f32 = block.iter().map(|s| s * s).sum();
    let rms = (sum / block.len() as f32).sqrt();
    if rms <= 1e-5 {
        -100.0
    } else {
        20.0 * rms.log10()
    }
}

pub struct Limiter {
    gain: f32,
    prev: Option<Block>,
}

impl Default for Limiter {
    fn default() -> Self {
        Limiter {
            gain: 1.0,
            prev: None,
        }
    }
}

impl Limiter {
    /// Nimmt Block k+1 an und liefert den begrenzten Block k (beim ersten Aufruf `None`).
    pub fn process(&mut self, next: &Block) -> Option<Block> {
        let mut cur = self.prev.replace(*next)?;
        let frames = BLOCK;
        let peak_at = |i: usize| -> f32 {
            let s = if i < frames {
                &cur[i * CHANNELS..i * CHANNELS + CHANNELS]
            } else {
                &next[(i - frames) * CHANNELS..(i - frames) * CHANNELS + CHANNELS]
            };
            s.iter().fold(0.0f32, |m, v| m.max(v.abs()))
        };
        let mut gains = [1.0f32; BLOCK];
        for (i, g) in gains.iter_mut().enumerate() {
            let peak = (i..i + LOOKAHEAD).map(peak_at).fold(0.0f32, f32::max);
            let target = if peak > THRESHOLD {
                THRESHOLD / peak
            } else {
                1.0
            };
            // Sofort absenken, langsam erholen.
            self.gain = if target < self.gain {
                target
            } else {
                self.gain + (target - self.gain) * RELEASE
            };
            *g = self.gain;
        }
        for (i, g) in gains.iter().enumerate() {
            for c in 0..CHANNELS {
                cur[i * CHANNELS + c] *= g;
            }
        }
        Some(cur)
    }
}

/// Summiert Quellen mit Lautstärke (linear).
pub fn mix(sources: &[(&Block, f32)]) -> Block {
    let mut out = [0.0f32; BLOCK * CHANNELS];
    for (block, gain) in sources {
        for (o, s) in out.iter_mut().zip(block.iter()) {
            *o += s * gain;
        }
    }
    out
}

/// Setzt die Frames eines Blocks stumm, die in `[from_us, to_us)` liegen. `block_us` ist die
/// Zeit des ersten Frames.
pub fn mute_window(block: &mut Block, block_us: i64, from_us: i64, to_us: i64) {
    let frame_us = |i: usize| block_us + (i as i64 * 1_000_000) / 48_000;
    for i in 0..BLOCK {
        let t = frame_us(i);
        if t >= from_us && t < to_us {
            block[i * CHANNELS] = 0.0;
            block[i * CHANNELS + 1] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_db_is_unity() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0) - 0.501).abs() < 0.001);
    }

    #[test]
    fn limiter_keeps_peaks_below_threshold_and_delays_one_block() {
        let mut l = Limiter::default();
        let quiet = [0.2f32; BLOCK * CHANNELS];
        let mut loud = [0.2f32; BLOCK * CHANNELS];
        // Spitze von 1,6 mitten im zweiten Block.
        for s in &mut loud[200 * CHANNELS..260 * CHANNELS] {
            *s = 1.6;
        }
        assert!(l.process(&quiet).is_none());
        let first = l.process(&loud).unwrap();
        // Die Spitze liegt bei Frame 200 von Block 1; in Block 0 sieht erst Frame 440 sie
        // (Vorausschau 240 Frames). Davor bleibt alles unverändert.
        assert!(
            first[..440 * CHANNELS]
                .iter()
                .all(|&s| (s - 0.2).abs() < 1e-6)
        );
        assert!(first[479 * CHANNELS] < 0.2);
        let second = l.process(&quiet).unwrap();
        let peak = second.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak <= THRESHOLD + 1e-4, "Spitze {peak}");
        // Nach der Spitze erholt sich die Verstärkung langsam (≈ 50 ms).
        let third = l.process(&quiet).unwrap();
        assert!(third[third.len() - 1] > third[0]);
        let mut last = third;
        for _ in 0..30 {
            last = l.process(&quiet).unwrap();
        }
        assert!(
            (last[last.len() - 1] - 0.2).abs() < 0.002,
            "nach 300 ms erholt"
        );
    }

    #[test]
    fn mute_window_silences_only_the_window() {
        let mut b = [0.5f32; BLOCK * CHANNELS];
        // Block bei 1 000 000 µs, stumm von 1 002 000 bis 1 004 000 (Frames 96–191).
        mute_window(&mut b, 1_000_000, 1_002_000, 1_004_000);
        assert_eq!(b[95 * CHANNELS], 0.5);
        assert_eq!(b[96 * CHANNELS], 0.0);
        assert_eq!(b[191 * CHANNELS], 0.0);
        assert_eq!(b[192 * CHANNELS], 0.5);
    }

    #[test]
    fn rms_of_full_scale_sine_is_minus_three() {
        let mut b = [0.0f32; BLOCK * CHANNELS];
        for i in 0..BLOCK {
            let s = (i as f32 * 1000.0 * std::f32::consts::TAU / 48_000.0).sin();
            b[i * 2] = s;
            b[i * 2 + 1] = s;
        }
        assert!((rms_dbfs(&b) + 3.01).abs() < 0.1);
    }
}
