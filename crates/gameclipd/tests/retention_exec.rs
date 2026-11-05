// SPDX-License-Identifier: GPL-3.0-or-later
//! Speicherlimit: löscht nur eigene Dateien (Design, Dateien 47 und 55).
#![cfg(windows)]

mod common;

use std::path::{Path, PathBuf};

use common::{FakeProbe, paths, tagged_mp4};
use gameclip_core::retention::{ClipStatus, FolderKind, Warning};
use gameclip_core::settings::StorageSettings;
use gameclipd::index::{Index, NewClip};
use gameclipd::paths::Paths;
use gameclipd::retention_exec::enforce;
use time::{Duration, OffsetDateTime};

/// Legt drei eigene Clips an (alt → neu), je etwa gleich groß; liefert (row_id, Pfad).
fn three_clips(index: &Index, p: &Paths, folder: FolderKind, dir: &Path) -> Vec<(u32, PathBuf)> {
    let mut out = Vec::new();
    for (i, name) in ["a.mp4", "b.mp4", "c.mp4"].iter().enumerate() {
        let path = dir.join(name);
        let tag = format!("tag-{name}");
        tagged_mp4(&path, &tag, 0);
        let id = index
            .insert_pending(&NewClip {
                tag_id: &tag,
                folder,
                path: &path,
                created_at: OffsetDateTime::now_utc() - Duration::minutes(10 - i as i64),
                source_row_id: None,
            })
            .unwrap();
        let size = std::fs::metadata(&path).unwrap().len();
        index.mark_ok(id, &path, size, 1_000_000, true).unwrap();
        std::fs::write(p.track_file(&tag), b"spuren").unwrap();
        out.push((id, path));
    }
    out
}

fn limit(bytes: u64) -> StorageSettings {
    StorageSettings {
        clip_folder: None,
        export_folder: None,
        clip_limit_bytes: bytes,
        export_limit_bytes: bytes,
    }
}

fn size(p: &Path) -> u64 {
    std::fs::metadata(p).unwrap().len()
}

#[test]
fn deletes_oldest_own_clips_until_under_limit() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let clips = three_clips(&index, &p, FolderKind::Clips, &p.clips);
    // Platz für zwei Clips: der älteste muss weg; der neueste ist geschützt.
    let two = size(&clips[1].1) + size(&clips[2].1) + 10;
    let done = enforce(
        &index,
        &p,
        &limit(two),
        Some(clips[2].0),
        &FakeProbe::default(),
    )
    .unwrap();
    assert_eq!(
        done.deleted.iter().map(|d| d.row_id).collect::<Vec<_>>(),
        [clips[0].0]
    );
    assert!(!clips[0].1.exists());
    assert!(
        !p.track_file("tag-a.mp4").exists(),
        "Einzelspuren mit gelöscht"
    );
    assert!(clips[1].1.exists() && clips[2].1.exists());
}

#[test]
fn skips_placeholders_and_foreign_tags() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let clips = three_clips(&index, &p, FolderKind::Clips, &p.clips);
    let probe = FakeProbe::default();
    // a ist Platzhalter (zählt 0 Bytes, nie gelöscht), b trägt inzwischen einen fremden Tag.
    probe.mark(&clips[0].1);
    index.set_star(clips[2].0, false).unwrap();
    tagged_mp4(&clips[1].1, "jemand-anderes", 0);
    let done = enforce(&index, &p, &limit(10), None, &probe).unwrap();
    println!("{done:?}");
    assert!(clips[0].1.exists(), "Platzhalter bleibt");
    assert!(clips[1].1.exists(), "fremder Tag: nicht löschen");
    assert_eq!(
        index.get(clips[1].0).unwrap().unwrap().status,
        ClipStatus::Foreign
    );
    assert_eq!(done.foreign, [clips[1].0]);
    assert!(!clips[2].1.exists());
}

#[test]
fn starred_clips_produce_a_warning_instead_of_deleting() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let clips = three_clips(&index, &p, FolderKind::Clips, &p.clips);
    for (id, _) in &clips {
        index.set_star(*id, true).unwrap();
    }
    // Jeder Clip passt einzeln ins Limit, alle drei zusammen nicht.
    let one = clips.iter().map(|c| size(&c.1)).max().unwrap() + 10;
    let done = enforce(&index, &p, &limit(one), None, &FakeProbe::default()).unwrap();
    assert!(done.deleted.is_empty());
    assert_eq!(done.warnings[0].1, Warning::AllStarred);
}

#[test]
fn shared_folder_forms_one_group_with_the_smaller_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = paths(dir.path());
    p.exports = p.clips.clone();
    let index = Index::open(&p.index).unwrap();
    let clips = three_clips(&index, &p, FolderKind::Clips, &p.clips);
    // Ein Export im selben Ordner, neuer als alle Clips.
    let export = p.clips.join("export.mp4");
    tagged_mp4(&export, "tag-export", 0);
    let e = index
        .insert_pending(&NewClip {
            tag_id: "tag-export",
            folder: FolderKind::Exports,
            path: &export,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: Some(clips[0].0),
        })
        .unwrap();
    index
        .mark_ok(e, &export, size(&export), 1_000_000, false)
        .unwrap();
    let storage = StorageSettings {
        clip_folder: None,
        export_folder: None,
        clip_limit_bytes: 1_000_000_000,
        export_limit_bytes: size(&export) + size(&clips[2].1) + 10,
    };
    let done = enforce(&index, &p, &storage, Some(e), &FakeProbe::default()).unwrap();
    let deleted: Vec<u32> = done.deleted.iter().map(|d| d.row_id).collect();
    assert_eq!(
        deleted,
        [clips[0].0, clips[1].0],
        "eine Gruppe, kleineres Limit, älteste zuerst"
    );
}

#[test]
fn deleting_the_last_clip_of_a_game_removes_its_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let game = p.clips.join("cs2");
    std::fs::create_dir_all(&game).unwrap();
    let clips = three_clips(&index, &p, FolderKind::Clips, &game);
    let one = size(&clips[2].1) + 10;
    let done = enforce(
        &index,
        &p,
        &limit(one),
        Some(clips[2].0),
        &FakeProbe::default(),
    )
    .unwrap();
    assert_eq!(done.deleted.len(), 2);
    assert!(game.is_dir(), "noch ein Clip drin: Ordner bleibt");

    gameclipd::paths::remove_empty_game_folder(&clips[2].1, &p.clips);
    assert!(game.is_dir(), "remove_dir lässt volle Ordner stehen");
    std::fs::remove_file(&clips[2].1).unwrap();
    gameclipd::paths::remove_empty_game_folder(&clips[2].1, &p.clips);
    assert!(!game.exists());
    gameclipd::paths::remove_empty_game_folder(&p.clips.join("x.mp4"), &p.clips);
    assert!(p.clips.is_dir(), "der Clip-Ordner selbst bleibt");
}

#[test]
fn clips_in_game_folders_count_as_current_folder() {
    use gameclipd::paths::in_folder;
    let root = Path::new(r"C:\Videos\GameClip");
    assert!(in_folder(&root.join("a.mp4"), root));
    assert!(in_folder(&root.join("cs2").join("a.mp4"), root));
    assert!(!in_folder(&root.join("cs2").join("x").join("a.mp4"), root));
    assert!(!in_folder(Path::new(r"C:\Andere\a.mp4"), root));
}
