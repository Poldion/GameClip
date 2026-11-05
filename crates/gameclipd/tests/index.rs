// SPDX-License-Identifier: GPL-3.0-or-later
//! Index und Abgleich mit dem Ordner (Design, Dateien 45 und 55).
#![cfg(windows)]

mod common;

use common::{FakeProbe, paths, tagged_mp4};
use gameclip_core::retention::{ClipStatus, FolderKind};
use gameclipd::index::{Index, NewClip};
use gameclipd::retention_exec::reconcile;
use time::OffsetDateTime;

fn own_clip(index: &Index, path: &std::path::Path, tag: &str) -> u32 {
    let id = index
        .insert_pending(&NewClip {
            tag_id: tag,
            folder: FolderKind::Clips,
            path,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: None,
        })
        .unwrap();
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    index.mark_ok(id, path, size, 1_000_000, true).unwrap();
    id
}

#[test]
fn reconcile_handles_unknown_copied_renamed_untagged_and_placeholder_files() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let probe = FakeProbe::default();

    // Eigener Clip, danach umbenannt.
    let original = p.clips.join("eigen.mp4");
    tagged_mp4(&original, "tag-eigen", 0);
    let own = own_clip(&index, &original, "tag-eigen");
    let renamed = p.clips.join("eigen_umbenannt.mp4");
    std::fs::rename(&original, &renamed).unwrap();

    // Zweiter eigener Clip und eine Kopie davon.
    let second = p.clips.join("zweiter.mp4");
    tagged_mp4(&second, "tag-zwei", 0);
    own_clip(&index, &second, "tag-zwei");
    std::fs::copy(&second, p.clips.join("zweiter - Kopie.mp4")).unwrap();

    // Unbekannter GameClip-Clip (z. B. nach Neuinstallation), fremde Datei ohne Tag, Platzhalter.
    tagged_mp4(&p.clips.join("unbekannt.mp4"), "tag-neu", 0);
    tagged_mp4(&p.clips.join("fremd.mp4"), "", 0);
    let cloud = p.clips.join("cloud.mp4");
    std::fs::write(&cloud, b"kein mp4, darf nicht gelesen werden").unwrap();
    probe.mark(&cloud);
    // .part wird übersprungen, ein pending-Eintrag bleibt.
    std::fs::write(p.clips.join("laufend.mp4.part"), b"x").unwrap();
    let pending = index
        .insert_pending(&NewClip {
            tag_id: "tag-laufend",
            folder: FolderKind::Clips,
            path: &p.clips.join("laufend.mp4"),
            created_at: OffsetDateTime::now_utc(),
            source_row_id: None,
        })
        .unwrap();

    let stats = reconcile(&index, &p, &p.clips, FolderKind::Clips, &probe).unwrap();
    println!("{stats:?}");
    assert_eq!(stats.renamed, [own]);
    assert_eq!(index.get(own).unwrap().unwrap().path, renamed);

    let rows = index.list(FolderKind::Clips).unwrap();
    let by_name = |n: &str| {
        rows.iter()
            .find(|r| r.path.file_name().unwrap() == n)
            .unwrap()
    };
    let copy = by_name("zweiter - Kopie.mp4");
    assert_eq!(
        (copy.status, copy.tag_id.as_str()),
        (ClipStatus::Foreign, "tag-zwei")
    );
    assert_eq!(by_name("zweiter.mp4").status, ClipStatus::Ok);
    let adopted = by_name("unbekannt.mp4");
    assert_eq!(adopted.status, ClipStatus::Ok);
    assert!(adopted.starred, "übernommene Clips sind geschützt");
    assert!(adopted.duration_us > 900_000);
    let foreign = by_name("fremd.mp4");
    assert_eq!(
        (foreign.status, foreign.tag_id.as_str()),
        (ClipStatus::Foreign, "")
    );
    let ph = by_name("cloud.mp4");
    assert!(ph.placeholder);
    assert_eq!(ph.duration_us, 0, "Platzhalter wird nicht gelesen");
    assert_eq!(
        index.get(pending).unwrap().unwrap().status,
        ClipStatus::Pending
    );
    assert!(!rows.iter().any(|r| r.path.extension().unwrap() == "part"));

    // Zweiter Lauf ändert nichts mehr.
    let again = reconcile(&index, &p, &p.clips, FolderKind::Clips, &probe).unwrap();
    assert_eq!(again, Default::default());
}

#[test]
fn missing_twice_removes_row_and_tracks() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let probe = FakeProbe::default();
    let clip = p.clips.join("weg.mp4");
    tagged_mp4(&clip, "tag-weg", 0);
    let id = own_clip(&index, &clip, "tag-weg");
    std::fs::write(p.track_file("tag-weg"), b"spuren").unwrap();
    std::fs::remove_file(&clip).unwrap();

    reconcile(&index, &p, &p.clips, FolderKind::Clips, &probe).unwrap();
    assert_eq!(
        index.get(id).unwrap().unwrap().missing_count,
        1,
        "einmal fehlend: bleibt"
    );
    let stats = reconcile(&index, &p, &p.clips, FolderKind::Clips, &probe).unwrap();
    assert_eq!(stats.removed, [id]);
    assert!(index.get(id).unwrap().is_none());
    assert!(!p.track_file("tag-weg").exists());
}

#[test]
fn clips_in_an_earlier_folder_stay_while_their_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let old_dir = dir.path().join("alter_ordner");
    std::fs::create_dir_all(&old_dir).unwrap();
    let old = old_dir.join("alt.mp4");
    tagged_mp4(&old, "tag-alt", 0);
    let id = own_clip(&index, &old, "tag-alt");
    for _ in 0..3 {
        reconcile(
            &index,
            &p,
            &p.clips,
            FolderKind::Clips,
            &FakeProbe::default(),
        )
        .unwrap();
    }
    assert_eq!(index.get(id).unwrap().unwrap().missing_count, 0);
}

#[test]
fn broken_database_is_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    std::fs::write(&p.index, b"das ist keine Datenbank, sondern Text").unwrap();
    let index = Index::open(&p.index).unwrap();
    let broken = index.recovered_from.clone().expect("ersetzt");
    assert!(broken.exists());
    assert!(index.list(FolderKind::Clips).unwrap().is_empty());
}

#[test]
fn reconcile_reads_game_folders_one_level_deep() {
    let dir = tempfile::tempdir().unwrap();
    let p = paths(dir.path());
    let index = Index::open(&p.index).unwrap();
    let probe = FakeProbe::default();
    let game = p.clips.join("VALORANT");
    std::fs::create_dir_all(game.join("tiefer")).unwrap();
    tagged_mp4(&game.join("a.mp4"), "tag-a", 0);
    tagged_mp4(&game.join("tiefer").join("b.mp4"), "tag-b", 0);

    let stats = reconcile(&index, &p, &p.clips, FolderKind::Clips, &probe).unwrap();
    assert_eq!(
        stats.adopted.len(),
        1,
        "nur der Spiele-Ordner, nicht tiefer"
    );
    let row = index.get(stats.adopted[0]).unwrap().unwrap();
    assert_eq!(row.path, game.join("a.mp4"));
}
