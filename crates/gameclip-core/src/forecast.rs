// SPDX-License-Identifier: GPL-3.0-or-later
//! RAM-Prognose für den Ringpuffer. Dieselbe Funktion nutzen Dienst und Oberfläche.

use serde::{Deserialize, Serialize};

use crate::settings::{Quality, Resolution, Settings};

/// Größe eines Blocks im Puffer-Pool (2 MiB).
pub const BLOCK_BYTES: u64 = 2 * 1024 * 1024;
/// Reserve für gehaltene Schnappschüsse: so viele Sekunden Aufnahme zusätzlich.
pub const RESERVE_SECONDS: u64 = 30;
/// Grundbedarf des Dienstes in der Anzeige; gleich dem vorläufigen Ziel G3.
pub const BASE_RAM_BYTES: u64 = 80_000_000;
pub const RAM_WARN_FRACTION: f64 = 0.10;
pub const RAM_REJECT_FRACTION: f64 = 0.25;

pub const AUDIO_TRACK_BPS: u64 = 128_000;
pub const AUDIO_MIX_BPS: u64 = 160_000;
pub const AUDIO_EXPORT_BPS: u64 = 96_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Level {
    Ok,
    Warning,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Forecast {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub buffer_bytes: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub reserve_bytes: u64,
    /// Wert, der angezeigt und mit den RAM-Schwellen verglichen wird.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub total_bytes: u64,
    pub level: Level,
}

/// Videobitrate in bit/s für Auflösung, Bildrate und Qualitätsstufe (vorläufige Werte, Q1).
pub fn video_bps(resolution: Resolution, fps: u32, quality: Quality) -> u64 {
    // Werte für 60 fps in kbit/s.
    let kbps_60: u64 = match (resolution, quality) {
        (Resolution::P720, Quality::Low) => 4_800,
        (Resolution::P720, Quality::Medium) => 8_400,
        (Resolution::P720, Quality::High) => 14_400,
        (Resolution::P1080, Quality::Low) => 8_000,
        (Resolution::P1080, Quality::Medium) => 14_000,
        (Resolution::P1080, Quality::High) => 24_000,
        (Resolution::P1440, Quality::Low) => 12_000,
        (Resolution::P1440, Quality::Medium) => 20_000,
        (Resolution::P1440, Quality::High) => 32_000,
        (Resolution::P2160, Quality::Low) => 25_000,
        (Resolution::P2160, Quality::Medium) => 40_000,
        (Resolution::P2160, Quality::High) => 60_000,
    };
    let kbps = if fps >= 60 {
        kbps_60
    } else {
        kbps_60 * 60 / 100
    };
    kbps * 1_000
}

/// Gesamtbitrate aus Video, Einzelspuren und Mischung in bit/s.
pub fn total_bps(settings: &Settings) -> u64 {
    let c = &settings.capture;
    video_bps(c.resolution, c.fps, c.quality)
        + u64::from(settings.separate_track_count()) * AUDIO_TRACK_BPS
        + AUDIO_MIX_BPS
}

pub fn forecast(settings: &Settings, physical_ram_bytes: u64) -> Forecast {
    let rate = total_bps(settings) as f64;
    let seconds = f64::from(settings.capture.buffer_seconds) + 1.0;
    let buffer_bytes = ceil_blocks(rate * seconds / 8.0 * 1.05);
    let reserve_bytes = ceil_blocks(rate * RESERVE_SECONDS as f64 / 8.0);
    let total_bytes = buffer_bytes + reserve_bytes + BASE_RAM_BYTES;

    let ram = physical_ram_bytes as f64;
    let total = total_bytes as f64;
    let level = if total >= ram * RAM_REJECT_FRACTION {
        Level::Rejected
    } else if total >= ram * RAM_WARN_FRACTION {
        Level::Warning
    } else {
        Level::Ok
    };
    Forecast {
        buffer_bytes,
        reserve_bytes,
        total_bytes,
        level,
    }
}

fn ceil_blocks(bytes: f64) -> u64 {
    let blocks = (bytes / BLOCK_BYTES as f64).ceil() as u64;
    blocks * BLOCK_BYTES
}
