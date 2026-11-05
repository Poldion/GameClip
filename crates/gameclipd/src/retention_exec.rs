// SPDX-License-Identifier: GPL-3.0-or-later
//! Abgleich des Index mit den Ordnern und Speicherlimit (Design, Dateien 45 und 47).
//!
//! **Abgleich** (`reconcile`): bekannte Zeilen auf Existenz prüfen (zweimal fehlend → Zeile und
//! ihre `.m4a` weg), neue `.mp4` im Ordner und in den Spiele-Ordnern direkt darunter einordnen: Platzhalter ohne Lesen, sonst Tag lesen –
//! bekannter Pfad, umbenannt, Kopie (`foreign`) oder unbekannter GameClip-Clip (übernommen mit
//! Stern). Dateien ohne `gameclip_id` werden als `foreign` mit leerem Tag geführt.
//!
//! **Speicherlimit** (`enforce`): je Gruppe aus `retention_groups` einmal `plan_deletions`.
//! Vor jedem Löschen: Platzhalter überspringen, Tag erneut lesen; weicht er ab, nicht löschen und
//! die Zeile auf `foreign` setzen.

use std::path::{Path, PathBuf};

use gameclip_core::retention::{
    ClipInfo, ClipStatus, FolderKind, Warning, plan_deletions, retention_groups,
};
use gameclip_core::settings::StorageSettings;
use time::OffsetDateTime;
use tracing::{info, warn};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
};
use windows_core::HSTRING;

use crate::index::{ClipRow, Index, Result};
use crate::paths::Paths;

/// Erkennt Cloud-Platzhalter, ohne die Datei zu öffnen (Öffnen würde den Download starten).
pub trait FileProbe {
    fn is_placeholder(&self, path: &Path) -> bool;
}

/// Windows-Umsetzung über `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`.
pub struct WinProbe;

impl FileProbe for WinProbe {
    fn is_placeholder(&self, path: &Path) -> bool {
        let attrs = unsafe { GetFileAttributesW(&HSTRING::from(path.as_os_str())) };
        attrs != INVALID_FILE_ATTRIBUTES && attrs & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0 != 0
    }
}

/// `gameclip_id` der Datei, falls lesbar.
pub fn read_tag(path: &Path) -> Option<String> {
    gameclip_media::demux::read_tags(path)
        .ok()?
        .get("gameclip_id")
        .cloned()
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileStats {
    pub removed: Vec<u32>,
    pub adopted: Vec<u32>,
    pub foreign: Vec<u32>,
    pub renamed: Vec<u32>,
}

pub fn reconcile(
    index: &Index,
    paths: &Paths,
    folder_path: &Path,
    folder: FolderKind,
    probe: &dyn FileProbe,
) -> Result<ReconcileStats> {
    let mut stats = ReconcileStats::default();
    // 1. Bestehende Zeilen (alle Ordner, auch frühere): Existenz prüfen.
    for row in index.list_all()? {
        if row.status == ClipStatus::Pending {
            continue;
        }
        if row.path.exists() {
            if row.missing_count != 0 {
                index.set_missing(row.row_id, 0)?;
            }
        } else if row.missing_count + 1 >= 2 {
            index.remove(row.row_id)?;
            if row.status == ClipStatus::Ok && !row.tag_id.is_empty() {
                let _ = std::fs::remove_file(paths.track_file(&row.tag_id));
            }
            stats.removed.push(row.row_id);
        } else {
            index.set_missing(row.row_id, row.missing_count + 1)?;
        }
    }

    // 2. Neue Dateien in diesem Ordner und in den Spiele-Ordnern direkt darunter.
    let Ok(entries) = std::fs::read_dir(folder_path) else {
        return Ok(stats);
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            if let Ok(sub) = std::fs::read_dir(entry.path()) {
                files.extend(
                    sub.flatten()
                        .filter(|e| e.file_type().is_ok_and(|t| t.is_file())),
                );
            }
        } else {
            files.push(entry);
        }
    }
    for entry in files {
        let path = entry.path();
        if path
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case("mp4"))
        {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let known = index.find_by_path(&path)?;
        if probe.is_placeholder(&path) {
            if known.is_none() {
                let id = index.insert_row(&new_row(
                    &path,
                    folder,
                    size,
                    "",
                    ClipStatus::Foreign,
                    true,
                ))?;
                stats.foreign.push(id);
            }
            continue;
        }
        if let Some(row) = known {
            if row.size_bytes != size {
                index.update_file(row.row_id, &path, size)?;
            }
            continue;
        }
        let Some(tag) = read_tag(&path) else {
            let id = index.insert_row(&new_row(
                &path,
                folder,
                size,
                "",
                ClipStatus::Foreign,
                false,
            ))?;
            stats.foreign.push(id);
            continue;
        };
        match index.find_ok_by_tag(&tag)? {
            Some(owner) if !owner.path.exists() => {
                index.update_file(owner.row_id, &path, size)?;
                stats.renamed.push(owner.row_id);
            }
            Some(_) => {
                let id = index.insert_row(&new_row(
                    &path,
                    folder,
                    size,
                    &tag,
                    ClipStatus::Foreign,
                    false,
                ))?;
                stats.foreign.push(id);
            }
            None => {
                let mut row = new_row(&path, folder, size, &tag, ClipStatus::Ok, false);
                row.starred = true;
                row.has_tracks = paths.track_file(&tag).is_file();
                let id = index.insert_row(&row)?;
                stats.adopted.push(id);
            }
        }
    }
    Ok(stats)
}

fn new_row(
    path: &Path,
    folder: FolderKind,
    size: u64,
    tag: &str,
    status: ClipStatus,
    placeholder: bool,
) -> ClipRow {
    let created = std::fs::metadata(path)
        .and_then(|m| m.created())
        .map(OffsetDateTime::from)
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let duration_us = if placeholder {
        0
    } else {
        gameclip_media::demux::video_info(path)
            .map(|i| i.duration_us)
            .unwrap_or(0)
    };
    ClipRow {
        row_id: 0,
        tag_id: tag.to_string(),
        folder,
        path: path.to_path_buf(),
        size_bytes: size,
        created_at: created,
        duration_us,
        starred: false,
        has_tracks: false,
        source_row_id: None,
        placeholder,
        missing_count: 0,
        status,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deleted {
    pub row_id: u32,
    pub folder: FolderKind,
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Enforced {
    pub deleted: Vec<Deleted>,
    pub warnings: Vec<(Vec<FolderKind>, Warning)>,
    /// Zeilen, deren Datei einen anderen Tag trug und deshalb nicht gelöscht wurde.
    pub foreign: Vec<u32>,
}

fn folder_dir(paths: &Paths, f: FolderKind) -> &Path {
    match f {
        FolderKind::Clips => &paths.clips,
        FolderKind::Exports => &paths.exports,
    }
}

/// Speicherlimit durchsetzen; `protect` ist der gerade geschriebene Clip oder Export.
pub fn enforce(
    index: &Index,
    paths: &Paths,
    storage: &StorageSettings,
    protect: Option<u32>,
    probe: &dyn FileProbe,
) -> Result<Enforced> {
    let mut out = Enforced::default();
    if index.is_read_only() {
        return Ok(out);
    }
    let same = paths.clips == paths.exports;
    let rows = index.list_all()?;
    for (folders, limit) in
        retention_groups(storage.clip_limit_bytes, storage.export_limit_bytes, same)
    {
        let group: Vec<&ClipRow> = rows
            .iter()
            .filter(|r| folders.contains(&r.folder))
            .collect();
        let infos: Vec<ClipInfo> = group
            .iter()
            .map(|r| ClipInfo {
                row_id: r.row_id,
                folder: r.folder,
                size_bytes: r.size_bytes,
                created_at: r.created_at,
                starred: r.starred,
                status: r.status,
                placeholder: r.placeholder,
                in_current_folder: crate::paths::in_folder(&r.path, folder_dir(paths, r.folder)),
            })
            .collect();
        let plan = plan_deletions(&infos, limit, protect);
        if let Some(w) = plan.warning {
            warn!("Speicherlimit {limit} Bytes für {folders:?}: {w:?}");
            out.warnings.push((folders.clone(), w));
        }
        for id in plan.delete {
            let Some(row) = group.iter().find(|r| r.row_id == id) else {
                continue;
            };
            if probe.is_placeholder(&row.path) {
                continue;
            }
            if read_tag(&row.path).as_deref() != Some(row.tag_id.as_str()) {
                warn!("Tag weicht ab, nicht gelöscht: {}", row.path.display());
                index.set_status(row.row_id, ClipStatus::Foreign)?;
                out.foreign.push(row.row_id);
                continue;
            }
            if let Err(e) = std::fs::remove_file(&row.path) {
                warn!("Löschen fehlgeschlagen: {}: {e}", row.path.display());
                continue;
            }
            crate::paths::remove_empty_game_folder(&row.path, folder_dir(paths, row.folder));
            if row.folder == FolderKind::Clips {
                let _ = std::fs::remove_file(paths.track_file(&row.tag_id));
            }
            index.remove(row.row_id)?;
            info!(
                "Gelöscht (Speicherlimit): {} ({} Bytes)",
                row.path.display(),
                row.size_bytes
            );
            out.deleted.push(Deleted {
                row_id: row.row_id,
                folder: row.folder,
                path: row.path.clone(),
                bytes: row.size_bytes,
            });
        }
    }
    Ok(out)
}
