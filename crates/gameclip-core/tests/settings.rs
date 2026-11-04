// SPDX-License-Identifier: GPL-3.0-or-later
use gameclip_core::settings::{Loaded, Settings, merge_into, parse};

#[test]
fn empty_file_gives_defaults() {
    assert_eq!(parse("").unwrap(), Loaded::Writable(Settings::default()));
}

#[test]
fn hotkey_without_modifier_fields_is_read() {
    let loaded = parse("[hotkey]\ncode = \"F13\"\n").unwrap();
    let s = loaded.settings();
    assert_eq!(s.hotkey.code, "F13");
    assert!(!s.hotkey.ctrl && !s.hotkey.alt && !s.hotkey.shift);
}

#[test]
fn newer_schema_is_read_only() {
    let loaded = parse("schema_version = 2\nfirst_run_done = true\n").unwrap();
    assert!(matches!(loaded, Loaded::ReadOnly(ref s) if s.first_run_done));
}

#[test]
fn unknown_keys_survive_saving() {
    let existing = "future_key = 7\n[capture]\nbuffer_seconds = 90\nnew_option = \"x\"\n";
    let mut s = parse(existing).unwrap().settings().clone();
    s.capture.buffer_seconds = 120;
    let out = merge_into(existing, &s).unwrap();
    assert!(out.contains("future_key = 7"));
    assert!(out.contains("new_option = \"x\""));
    let back = parse(&out).unwrap();
    assert_eq!(back.settings().capture.buffer_seconds, 120);
}

#[test]
fn option_set_to_none_is_removed() {
    let existing = "[storage]\nclip_folder = 'D:\\Clips'\n";
    let mut s = parse(existing).unwrap().settings().clone();
    assert!(s.storage.clip_folder.is_some());
    s.storage.clip_folder = None;
    let out = merge_into(existing, &s).unwrap();
    assert!(
        parse(&out)
            .unwrap()
            .settings()
            .storage
            .clip_folder
            .is_none()
    );
}

#[test]
fn validation_reports_fields() {
    let mut s = Settings::default();
    assert!(s.validate().is_ok());
    s.capture.buffer_seconds = 5;
    s.capture.fps = 144;
    s.hotkey.code = "F12".into();
    s.export.target_bytes = 10;
    let keys: Vec<_> = s
        .validate()
        .unwrap_err()
        .into_iter()
        .map(|e| e.key)
        .collect();
    assert_eq!(
        keys,
        [
            "capture.buffer_seconds",
            "capture.fps",
            "hotkey",
            "export.target_bytes"
        ]
    );
}

#[test]
fn pipeline_change_detection() {
    let a = Settings::default();
    let mut b = a.clone();
    b.storage.clip_limit_bytes = 5_000_000_000;
    b.audio.gain_db.mic = 3.0;
    assert!(!Settings::pipeline_changed(&a, &b));
    b.capture.fps = 30;
    assert!(Settings::pipeline_changed(&a, &b));
}
