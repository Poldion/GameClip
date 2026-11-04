// SPDX-License-Identifier: GPL-3.0-or-later
//! Eine Tonquelle auf der gemeinsamen Zeitachse (Design, Datei 37).
//!
//! Die Zeitachse zählt Sample-Frames zu 48 kHz ab dem Start der Audio-Pipeline (QPC). Jeder
//! WASAPI-Block wird anhand seines QPC-Zeitstempels eingeordnet: Lücken (WASAPI liefert bei
//! Stille oft nichts) werden mit Stille gefüllt, Überlappungen abgeschnitten. Die Drift zwischen
//! Geräteuhr und QPC gleicht ein `Slip`-Resampler aus, dessen Verhältnis ein Regler über den
//! mittleren Füllstand (gleitendes Mittel über 10 s) um höchstens ±0,1 % verstellt.

use std::collections::VecDeque;

use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{FixedAsync, Resampler, Slip};

pub const RATE: i64 = 48_000;
pub const CHANNELS: usize = 2;
/// Frames je Block (10 ms).
pub const BLOCK: usize = 480;
/// Ab dieser Abweichung zwischen Zeitstempel und Pufferende gilt ein Block als Lücke bzw.
/// Überlappung (20 ms).
const GAP_FRAMES: i64 = 960;
/// Größte Korrektur des Verhältnisses.
const MAX_CORRECTION: f64 = 0.001;
/// Füllstandsabweichung, bei der die volle Korrektur greift (10 ms).
const FULL_CORRECTION_AT: f64 = 480.0;
/// Zeitkonstante des gleitenden Mittels in Blöcken (10 s).
const AVERAGE_BLOCKS: f64 = 1000.0;
/// Blöcke, nach denen der Bezugsfüllstand festgelegt wird (2 s).
const WARMUP_BLOCKS: u64 = 200;

/// QPC-Zeit in µs → Frame auf der Zeitachse ab `start_us`.
pub fn frames_since(start_us: i64, qpc_us: i64) -> i64 {
    ((i128::from(qpc_us - start_us) * i128::from(RATE)) / 1_000_000) as i64
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct TrackStats {
    /// Mit Stille gefüllte Frames wegen Lücken zwischen Blöcken.
    pub gap_frames: u64,
    /// Abgeschnittene Frames wegen Überlappung.
    pub overlap_frames: u64,
    /// Fehlende Frames beim Abholen (Quelle zu spät).
    pub underrun_frames: u64,
    /// Aktuelles Verhältnis des Resamplers (Ausgabe/Eingabe).
    pub ratio: f64,
}

pub struct SourceTrack {
    start_us: i64,
    buf: VecDeque<f32>,
    /// Zeitachsen-Frame von `buf[0]`; `None`, bis der erste Block da ist.
    head: Option<i64>,
    slip: Slip<f32>,
    fill_avg: f64,
    reference: Option<f64>,
    blocks: u64,
    scratch_in: Vec<f32>,
    stats: TrackStats,
}

impl SourceTrack {
    pub fn new(start_us: i64) -> Self {
        SourceTrack {
            start_us,
            buf: VecDeque::new(),
            head: None,
            slip: Slip::new(BLOCK, CHANNELS, FixedAsync::Output).expect("Slip mit 480 Frames"),
            fill_avg: 0.0,
            reference: None,
            blocks: 0,
            scratch_in: Vec::new(),
            stats: TrackStats {
                ratio: 1.0,
                ..TrackStats::default()
            },
        }
    }

    pub fn stats(&self) -> TrackStats {
        self.stats
    }

    fn end(&self) -> Option<i64> {
        self.head.map(|h| h + (self.buf.len() / CHANNELS) as i64)
    }

    /// Nimmt einen WASAPI-Block (interleaved Stereo) mit QPC-Zeit des ersten Frames an.
    pub fn push(&mut self, qpc_us: i64, samples: &[f32], silent: bool) {
        let pos = frames_since(self.start_us, qpc_us);
        let mut samples = samples;
        match self.end() {
            None => self.head = Some(pos),
            Some(end) if pos > end + GAP_FRAMES => {
                let gap = (pos - end) as usize;
                self.buf.extend(std::iter::repeat_n(0.0, gap * CHANNELS));
                self.stats.gap_frames += gap as u64;
            }
            Some(end) if pos < end - GAP_FRAMES => {
                let overlap = ((end - pos) as usize).min(samples.len() / CHANNELS);
                samples = &samples[overlap * CHANNELS..];
                self.stats.overlap_frames += overlap as u64;
            }
            Some(_) => {}
        }
        if silent {
            self.buf.extend(std::iter::repeat_n(
                0.0,
                samples.len() / CHANNELS * CHANNELS,
            ));
        } else {
            self.buf
                .extend(&samples[..samples.len() / CHANNELS * CHANNELS]);
        }
    }

    /// Liefert Block `k` (Frames `k·480 … k·480+479` der Zeitachse) nach `out`.
    pub fn pull(&mut self, k: i64, out: &mut [f32; BLOCK * CHANNELS]) {
        out.fill(0.0);
        let block_start = k * BLOCK as i64;
        let Some(head) = self.head else {
            return;
        };
        if head >= block_start + BLOCK as i64 {
            // Quelle beginnt erst später.
            return;
        }
        if head < block_start {
            // Daten liegen vor dem Block (Quelle eingestiegen, als der Mixer schon weiter war).
            let drop = ((block_start - head) as usize).min(self.buf.len() / CHANNELS);
            self.buf.drain(..drop * CHANNELS);
            self.head = Some(head + drop as i64);
        }
        let head = self.head.expect("gesetzt");
        if head > block_start {
            // Teilweise vor dem Beginn der Quelle: vorne Stille, Rest ungeresampelt.
            let offset = (head - block_start) as usize;
            let take = (BLOCK - offset).min(self.buf.len() / CHANNELS);
            for (i, s) in self.buf.drain(..take * CHANNELS).enumerate() {
                out[offset * CHANNELS + i] = s;
            }
            self.head = Some(block_start + BLOCK as i64);
            return;
        }

        let need = self.slip.input_frames_next();
        let have = self.buf.len() / CHANNELS;
        self.scratch_in.clear();
        self.scratch_in
            .extend(self.buf.iter().take(need.min(have) * CHANNELS));
        if have < need {
            self.scratch_in.resize(need * CHANNELS, 0.0);
            self.stats.underrun_frames += (need - have) as u64;
        }
        let input = InterleavedSlice::new(&self.scratch_in, CHANNELS, need).expect("Eingabe");
        let mut output = InterleavedSlice::new_mut(&mut out[..], CHANNELS, BLOCK).expect("Ausgabe");
        let (used, _) = self
            .slip
            .process_into_buffer(&input, &mut output, None)
            .unwrap_or((need, BLOCK));
        let consumed = used.min(have);
        self.buf.drain(..consumed * CHANNELS);
        self.head = Some(block_start + BLOCK as i64);
        self.regulate();
    }

    /// Hält den mittleren Füllstand beim Bezugswert: Puffer wächst → mehr Eingabe verbrauchen
    /// (Verhältnis < 1), Puffer schrumpft → weniger (Verhältnis > 1).
    fn regulate(&mut self) {
        let fill = (self.buf.len() / CHANNELS) as f64;
        self.blocks += 1;
        if self.blocks == 1 {
            self.fill_avg = fill;
        } else {
            self.fill_avg += (fill - self.fill_avg) / AVERAGE_BLOCKS.min(self.blocks as f64);
        }
        if self.blocks == WARMUP_BLOCKS {
            self.reference = Some(self.fill_avg);
        }
        let Some(reference) = self.reference else {
            return;
        };
        let error = (self.fill_avg - reference) / FULL_CORRECTION_AT;
        let ratio = 1.0 - (error * MAX_CORRECTION).clamp(-MAX_CORRECTION, MAX_CORRECTION);
        if let Some(adj) = self.slip.as_adjustable() {
            let _ = adj.set_resample_ratio(ratio, true);
        }
        self.stats.ratio = ratio;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_of(value: f32, frames: usize) -> Vec<f32> {
        vec![value; frames * CHANNELS]
    }

    #[test]
    fn source_starting_late_gets_leading_silence() {
        let mut t = SourceTrack::new(0);
        // Erster Block erst bei 15 ms (Frame 720) → Block 1 (480–959) beginnt mit 240 Frames Stille.
        t.push(15_000, &block_of(0.5, 480), false);
        let mut out = [0.0; BLOCK * CHANNELS];
        t.pull(0, &mut out);
        assert!(out.iter().all(|&s| s == 0.0));
        t.pull(1, &mut out);
        assert!(out[..240 * CHANNELS].iter().all(|&s| s == 0.0));
        assert!(out[240 * CHANNELS..].iter().all(|&s| s == 0.5));
    }

    #[test]
    fn gap_is_filled_with_silence_at_the_right_place() {
        let mut t = SourceTrack::new(0);
        t.push(0, &block_of(0.5, 480), false);
        // Nächster Block erst bei 50 ms statt 10 ms → 40 ms Lücke.
        t.push(50_000, &block_of(0.25, 480), false);
        assert_eq!(t.stats().gap_frames, 1920);
        let mut out = [0.0; BLOCK * CHANNELS];
        let mut blocks = Vec::new();
        for k in 0..6 {
            t.pull(k, &mut out);
            blocks.push(out[100 * CHANNELS]);
        }
        // Slip verzögert nicht; Werte erscheinen in den Blöcken ihrer Zeit.
        assert_eq!(blocks, [0.5, 0.0, 0.0, 0.0, 0.0, 0.25]);
    }

    #[test]
    fn overlap_is_trimmed() {
        let mut t = SourceTrack::new(0);
        t.push(0, &block_of(0.5, 2400), false);
        // Zeitstempel 25 ms (Frame 1200) statt 50 ms (Frame 2400) → 1200 Frames abgeschnitten.
        t.push(25_000, &block_of(0.25, 1440), false);
        assert_eq!(t.stats().overlap_frames, 1200);
        assert_eq!(t.end(), Some(2400 + 240));
    }

    #[test]
    fn regulator_corrects_a_fast_device_clock() {
        // Gerät liefert 0,05 % mehr Samples als die Zeitachse verbraucht.
        let mut t = SourceTrack::new(0);
        let mut out = [0.0; BLOCK * CHANNELS];
        let mut produced = 0.0f64;
        let mut fed = 0usize;
        for k in 0..6_000i64 {
            produced += 480.0 * 1.0005;
            let n = produced as usize - fed;
            t.push(k * 10_000, &block_of(0.1, n), false);
            fed += n;
            // Mixer liegt 10 Blöcke zurück.
            if k >= 10 {
                t.pull(k - 10, &mut out);
            }
        }
        let s = t.stats();
        println!(
            "Verhältnis {:.5}, Unterläufe {}",
            s.ratio, s.underrun_frames
        );
        // Der Regler verbraucht mehr Eingabe: Verhältnis knapp unter 1.
        assert!(
            s.ratio < 0.99985 && s.ratio >= 0.999,
            "Verhältnis {}",
            s.ratio
        );
        // Der Füllstand läuft nicht davon (ohne Regelung wären es nach 60 s +1440 Frames).
        let fill = t.buf.len() / CHANNELS;
        assert!(fill < 4800 + 900, "Füllstand {fill}");
    }
}
