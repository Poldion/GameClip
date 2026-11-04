// SPDX-License-Identifier: GPL-3.0-or-later
//! Planung des Größen-Exports: Auflösung, Bildrate und Videobitrate für eine Zielgröße.
//!
//! Oberfläche (Vorab-Warnung) und Worker rechnen mit derselben Funktion.

use serde::{Deserialize, Serialize};

use crate::forecast::AUDIO_EXPORT_BPS;

/// Mindestens so viele Bit pro Pixel und Bild, bevor eine Stufe gewählt wird (vorläufig, S11).
pub const BPP_MIN: f64 = 0.05;
/// Darunter warnt der Export-Dialog vor sehr niedriger Qualität (vorläufig, S11).
pub const BPP_WARN: f64 = 0.03;
/// Unter dieser Videobitrate ist ein Export nicht sinnvoll.
pub const MIN_VIDEO_BPS: f64 = 50_000.0;
/// Reserve für den Container in Byte.
pub const CONTAINER_RESERVE_BYTES: f64 = 64_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ExportPlan {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub video_bps: u64,
    pub warn_low_quality: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum PlanError {
    /// Die Zielgröße reicht für diese Dauer nicht; `min_target_bytes` würde reichen.
    TooSmall {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        min_target_bytes: u64,
    },
}

/// Stufen in absteigender Reihenfolge: (Breite, Höhe, fps).
const TIERS: [(u32, u32, u32); 4] = [
    (1920, 1080, 60),
    (1280, 720, 60),
    (1280, 720, 30),
    (854, 480, 30),
];

/// Plant einen Größen-Export.
///
/// `source` ist die Auflösung und Bildrate des Original-Clips; es wird nie hochskaliert.
pub fn plan(
    duration_us: u64,
    target_bytes: u64,
    source: (u32, u32, u32),
) -> Result<ExportPlan, PlanError> {
    let seconds = (duration_us as f64 / 1_000_000.0).max(0.001);
    let video_bps = (target_bytes as f64 * 8.0 * 0.92
        - AUDIO_EXPORT_BPS as f64 * seconds
        - CONTAINER_RESERVE_BYTES * 8.0)
        / seconds;

    if video_bps < MIN_VIDEO_BPS {
        let min_target = ((MIN_VIDEO_BPS + AUDIO_EXPORT_BPS as f64) * seconds
            + CONTAINER_RESERVE_BYTES * 8.0)
            / (8.0 * 0.92);
        return Err(PlanError::TooSmall {
            min_target_bytes: min_target.ceil() as u64,
        });
    }

    let (src_w, src_h, src_fps) = source;
    let candidates: Vec<_> = TIERS
        .iter()
        .copied()
        .filter(|&(_, h, fps)| h <= src_h.max(480) && fps <= src_fps.max(30))
        .collect();
    let bpp =
        |(w, h, fps): (u32, u32, u32)| video_bps / (f64::from(w) * f64::from(h) * f64::from(fps));

    let chosen = candidates
        .iter()
        .copied()
        .find(|&tier| bpp(tier) >= BPP_MIN)
        .unwrap_or(*candidates.last().unwrap_or(&TIERS[3]));
    // Quellen kleiner als 480p behalten ihre Größe.
    let (w, h, fps) = if src_h < 480 {
        (src_w, src_h, chosen.2)
    } else {
        chosen
    };

    Ok(ExportPlan {
        width: w,
        height: h,
        fps,
        video_bps: video_bps as u64,
        warn_low_quality: bpp((w, h, fps)) < BPP_WARN,
    })
}

/// Neue Videobitrate für den einen Korrekturversuch nach einem zu großen Ergebnis.
pub fn correct(video_bps: u64, target_bytes: u64, actual_bytes: u64) -> u64 {
    if actual_bytes == 0 {
        return video_bps;
    }
    (video_bps as f64 * target_bytes as f64 / actual_bytes as f64 * 0.97) as u64
}
