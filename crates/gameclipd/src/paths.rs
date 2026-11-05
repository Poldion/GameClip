// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordner und Dateien des Dienstes (Design, „Speicherorte“).

use std::path::{Path, PathBuf};

use gameclip_core::settings::Settings;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Videos, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub clips: PathBuf,
    pub exports: PathBuf,
    /// `%LOCALAPPDATA%\GameClip\tracks` (Einzelspuren `<tag_id>.m4a`).
    pub tracks: PathBuf,
    pub index: PathBuf,
    pub settings: PathBuf,
    pub logs: PathBuf,
}

impl Paths {
    /// Ordner aus den Einstellungen bzw. die Standardordner.
    pub fn resolve(settings: &Settings) -> Self {
        let local = env_dir("LOCALAPPDATA").join("GameClip");
        let roaming = env_dir("APPDATA").join("GameClip");
        let (clips, exports) = default_folders();
        Paths {
            clips: settings.storage.clip_folder.clone().unwrap_or(clips),
            exports: settings.storage.export_folder.clone().unwrap_or(exports),
            tracks: local.join("tracks"),
            index: local.join("index.sqlite"),
            settings: roaming.join("settings.toml"),
            logs: local.join("logs"),
        }
    }

    /// Alle Ordner anlegen (Clip- und Export-Ordner nur, wenn es die Standardordner sind).
    pub fn create_all(&self) -> std::io::Result<()> {
        for dir in [&self.clips, &self.exports, &self.tracks, &self.logs] {
            std::fs::create_dir_all(dir)?;
        }
        if let Some(p) = self.index.parent() {
            std::fs::create_dir_all(p)?;
        }
        Ok(())
    }

    pub fn track_file(&self, tag_id: &str) -> PathBuf {
        self.tracks.join(format!("{tag_id}.m4a"))
    }
}

/// Liegt `path` in `dir` oder in einem Spiele-Ordner direkt darunter?
pub fn in_folder(path: &Path, dir: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    parent == dir || parent.parent() == Some(dir)
}

/// Entfernt den Spiele-Ordner von `deleted`, wenn er nach dem Löschen leer ist. Der Clip- oder
/// Export-Ordner selbst (`root`) bleibt immer stehen; volle Ordner lässt `remove_dir` stehen.
pub fn remove_empty_game_folder(deleted: &Path, root: &Path) {
    if let Some(parent) = deleted.parent()
        && parent != root
        && parent.parent() == Some(root)
    {
        let _ = std::fs::remove_dir(parent);
    }
}

fn env_dir(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `<Videos>\GameClip` und `<Videos>\GameClip Export`; liegt `<Videos>` in OneDrive,
/// `%USERPROFILE%\GameClip` bzw. `%USERPROFILE%\GameClip Export`.
pub fn default_folders() -> (PathBuf, PathBuf) {
    let base = match videos_dir() {
        Some(v) if !is_under_onedrive(&v) => v,
        _ => env_dir("USERPROFILE"),
    };
    (base.join("GameClip"), base.join("GameClip Export"))
}

fn videos_dir() -> Option<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Videos, KF_FLAG_DEFAULT, None).ok()?;
        let path = p.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(p.0 as *const _));
        path
    }
}

pub fn is_under_onedrive(path: &Path) -> bool {
    std::env::var_os("OneDrive")
        .map(PathBuf::from)
        .is_some_and(|od| !od.as_os_str().is_empty() && path.starts_with(od))
}
