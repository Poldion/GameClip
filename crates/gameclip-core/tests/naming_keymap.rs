// SPDX-License-Identifier: GPL-3.0-or-later
use gameclip_core::keymap::{KeyError, MOD_CONTROL, MOD_NOREPEAT, to_vk};
use gameclip_core::naming::{ExportSuffix, clip_file_name, dedupe, export_file_name};
use gameclip_core::settings::Hotkey;
use time::macros::datetime;

fn key(code: &str, ctrl: bool) -> Hotkey {
    Hotkey {
        code: code.into(),
        ctrl,
        ..Hotkey::default()
    }
}

#[test]
fn clip_names() {
    let t = datetime!(2026-11-04 21:05:09 +1);
    assert_eq!(
        clip_file_name(Some("VALORANT-Win64-Shipping.exe"), t),
        "VALORANT-Win64-Shipping_2026-11-04_21-05-09.mp4"
    );
    assert_eq!(clip_file_name(None, t), "Clip_2026-11-04_21-05-09.mp4");
    assert_eq!(
        clip_file_name(Some("a<b>c:d"), t),
        "a_b_c_d_2026-11-04_21-05-09.mp4"
    );
    assert_eq!(
        clip_file_name(Some("con.exe"), t),
        "con__2026-11-04_21-05-09.mp4"
    );
    assert_eq!(
        clip_file_name(Some("game. "), t),
        "game_2026-11-04_21-05-09.mp4"
    );
    let long = "x".repeat(100);
    assert_eq!(
        clip_file_name(Some(&long), t).len(),
        64 + "_2026-11-04_21-05-09.mp4".len()
    );
}

#[test]
fn export_names() {
    assert_eq!(
        export_file_name("Clip_1", ExportSuffix::Trim),
        "Clip_1_trim.mp4"
    );
    assert_eq!(
        export_file_name("Clip_1", ExportSuffix::Tracks),
        "Clip_1_tracks.mp4"
    );
    assert_eq!(
        export_file_name("Clip_1", ExportSuffix::TargetSize { bytes: 10_000_000 }),
        "Clip_1_10MB.mp4"
    );
    assert_eq!(
        export_file_name("Clip_1", ExportSuffix::TargetSize { bytes: 8_500_000 }),
        "Clip_1_9MB.mp4"
    );
}

#[test]
fn dedupe_appends_counter() {
    let taken = ["a.mp4", "a_2.mp4"];
    assert_eq!(dedupe("a.mp4", |n| taken.contains(&n)), "a_3.mp4");
    assert_eq!(dedupe("b.mp4", |n| taken.contains(&n)), "b.mp4");
}

#[test]
fn hotkeys() {
    assert_eq!(to_vk(&key("F8", false)), Ok((MOD_NOREPEAT, 0x77)));
    assert_eq!(to_vk(&key("F13", false)), Ok((MOD_NOREPEAT, 0x7C)));
    assert_eq!(to_vk(&key("F24", false)), Ok((MOD_NOREPEAT, 0x87)));
    assert_eq!(to_vk(&key("F12", false)), Err(KeyError::F12Reserved));
    assert_eq!(to_vk(&key("KeyA", false)), Err(KeyError::NeedsModifier));
    assert_eq!(
        to_vk(&key("KeyA", true)),
        Ok((MOD_NOREPEAT | MOD_CONTROL, 0x41))
    );
    assert_eq!(
        to_vk(&key("Digit5", true)),
        Ok((MOD_NOREPEAT | MOD_CONTROL, 0x35))
    );
    assert!(matches!(
        to_vk(&key("Space", true)),
        Err(KeyError::Unknown(_))
    ));
    assert!(matches!(
        to_vk(&key("F25", false)),
        Err(KeyError::Unknown(_))
    ));
}

#[test]
fn program_from_file_name_reads_the_prefix() {
    use gameclip_core::naming::program_from_file_name as p;
    assert_eq!(
        p("VALORANT-Win64-Shipping_2026-11-05_21-14-03.mp4").as_deref(),
        Some("VALORANT")
    );
    assert_eq!(p("cs2_2026-11-05_21-14-03_2.mp4").as_deref(), Some("cs2"));
    assert_eq!(
        p("Rocket League_2026-11-05_21-14-03_10MB.mp4").as_deref(),
        Some("Rocket League")
    );
    assert_eq!(
        p("a_b_2026-11-05_21-14-03_trim.mp4").as_deref(),
        Some("a_b")
    );
    assert_eq!(p("Clip_2026-11-05_21-14-03.mp4"), None);
    assert_eq!(p("Urlaub.mp4"), None);
    assert_eq!(p("_2026-11-05_21-14-03.mp4"), None);
    assert_eq!(p(""), None);
}
