// SPDX-License-Identifier: GPL-3.0-or-later
use gameclip_core::forecast::{BLOCK_BYTES, Level, forecast};
use gameclip_core::settings::{Quality, Resolution, Settings};

const GIB: u64 = 1024 * 1024 * 1024;

#[test]
fn default_1080p60_medium_60s_two_tracks() {
    let s = Settings::default();
    assert_eq!(s.separate_track_count(), 2);
    let f = forecast(&s, 16 * GIB);
    assert_eq!(f.buffer_bytes, 56 * BLOCK_BYTES);
    assert_eq!(f.buffer_bytes, 117_440_512);
    assert_eq!(f.reserve_bytes, 54_525_952);
    assert_eq!(f.total_bytes, 251_966_464);
    assert_eq!(f.level, Level::Ok);
}

#[test]
fn p1440_high_300s_three_tracks() {
    let mut s = Settings::default();
    s.capture.resolution = Resolution::P1440;
    s.capture.quality = Quality::High;
    s.capture.buffer_seconds = 300;
    s.audio.discord_enabled = true;
    let f = forecast(&s, 16 * GIB);
    assert_eq!(f.buffer_bytes, 1_287_651_328);
    assert_eq!(f.reserve_bytes, 123_731_968);
    assert_eq!(f.total_bytes, 1_491_383_296);
    assert_eq!(f.level, Level::Ok);
    assert_eq!(forecast(&s, 8 * GIB).level, Level::Warning);
    assert_eq!(forecast(&s, 4 * GIB).level, Level::Rejected);
}

#[test]
fn thirty_fps_uses_sixty_percent() {
    use gameclip_core::forecast::video_bps;
    assert_eq!(video_bps(Resolution::P1080, 30, Quality::Medium), 8_400_000);
    assert_eq!(video_bps(Resolution::P720, 30, Quality::Medium), 5_040_000);
}

#[test]
fn mic_off_reduces_track_count() {
    let mut s = Settings::default();
    s.audio.mic_enabled = false;
    assert_eq!(s.separate_track_count(), 1);
    assert!(
        forecast(&s, 16 * GIB).buffer_bytes < forecast(&Settings::default(), 16 * GIB).buffer_bytes
    );
}
