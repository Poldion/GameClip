// SPDX-License-Identifier: GPL-3.0-or-later
use gameclip_core::export_plan::{PlanError, correct, plan};

const SRC_1080P60: (u32, u32, u32) = (1920, 1080, 60);
const S: u64 = 1_000_000;

#[test]
fn ten_mb_thirty_seconds_is_720p30() {
    let p = plan(30 * S, 10_000_000, SRC_1080P60).unwrap();
    assert_eq!((p.width, p.height, p.fps), (1280, 720, 30));
    assert!(
        (2_330_000..2_350_000).contains(&p.video_bps),
        "{}",
        p.video_bps
    );
    assert!(!p.warn_low_quality);
}

#[test]
fn ten_mb_three_hundred_seconds_is_480p30_with_warning() {
    let p = plan(300 * S, 10_000_000, SRC_1080P60).unwrap();
    assert_eq!((p.width, p.height, p.fps), (854, 480, 30));
    assert!(p.warn_low_quality);
}

#[test]
fn short_clip_keeps_1080p60() {
    let p = plan(3 * S, 10_000_000, SRC_1080P60).unwrap();
    assert_eq!((p.width, p.height, p.fps), (1920, 1080, 60));
}

#[test]
fn p1440_source_starts_at_1080p60() {
    let p = plan(3 * S, 10_000_000, (2560, 1440, 60)).unwrap();
    assert_eq!((p.width, p.height, p.fps), (1920, 1080, 60));
}

#[test]
fn never_upscales() {
    let p = plan(3 * S, 50_000_000, (1280, 720, 30)).unwrap();
    assert_eq!((p.width, p.height, p.fps), (1280, 720, 30));
}

#[test]
fn one_mb_three_hundred_seconds_is_too_small() {
    let err = plan(300 * S, 1_000_000, SRC_1080P60).unwrap_err();
    let PlanError::TooSmall { min_target_bytes } = err;
    assert!(min_target_bytes > 1_000_000);
    // Die genannte Mindestgröße muss tatsächlich reichen.
    assert!(plan(300 * S, min_target_bytes, SRC_1080P60).is_ok());
}

#[test]
fn correction_scales_down() {
    assert_eq!(correct(1_000_000, 10_000_000, 11_000_000), 881_818);
    assert_eq!(correct(1_000_000, 10_000_000, 0), 1_000_000);
}
