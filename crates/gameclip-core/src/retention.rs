// SPDX-License-Identifier: GPL-3.0-or-later
//! Speicherlimit: welche Clips gelöscht werden, damit ein Ordner unter seinem Limit bleibt.
//!
//! Reine Funktionen ohne Dateizugriff. Das eigentliche Löschen und die doppelte
//! Eigentumsprüfung macht der Dienst.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum FolderKind {
    Clips,
    Exports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ClipStatus {
    Pending,
    Ok,
    Foreign,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipInfo {
    pub row_id: u32,
    pub folder: FolderKind,
    pub size_bytes: u64,
    pub created_at: OffsetDateTime,
    pub starred: bool,
    pub status: ClipStatus,
    pub placeholder: bool,
    pub in_current_folder: bool,
}

impl ClipInfo {
    fn counts(&self) -> bool {
        self.status == ClipStatus::Ok && self.in_current_folder
    }

    /// Belegter lokaler Platz; Cloud-Platzhalter belegen keinen.
    fn local_bytes(&self) -> u64 {
        if self.placeholder { 0 } else { self.size_bytes }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Warning {
    /// Alle verbleibenden Clips haben einen Stern.
    AllStarred,
    /// Ein einzelner Clip ist größer als das Limit.
    ClipLargerThanLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionPlan {
    pub delete: Vec<u32>,
    pub warning: Option<Warning>,
}

/// Plant das Löschen der ältesten Clips ohne Stern, bis die Summe unter `limit_bytes` liegt.
///
/// `protect` ist der gerade geschriebene Clip; er wird nie gelöscht. Reicht Löschen nicht,
/// wird gar nichts gelöscht und eine Warnung zurückgegeben.
pub fn plan_deletions(clips: &[ClipInfo], limit_bytes: u64, protect: Option<u32>) -> RetentionPlan {
    let counted: Vec<&ClipInfo> = clips.iter().filter(|c| c.counts()).collect();
    let total: u64 = counted.iter().map(|c| c.local_bytes()).sum();
    if total <= limit_bytes {
        return RetentionPlan::default();
    }

    let mut candidates: Vec<&ClipInfo> = counted
        .iter()
        .copied()
        .filter(|c| !c.starred && !c.placeholder && Some(c.row_id) != protect)
        .collect();
    candidates.sort_by_key(|c| (c.created_at, c.row_id));

    let mut remaining = total;
    let mut delete = Vec::new();
    for clip in candidates {
        if remaining <= limit_bytes {
            break;
        }
        remaining -= clip.local_bytes();
        delete.push(clip.row_id);
    }

    if remaining > limit_bytes {
        let too_large = counted.iter().any(|c| c.local_bytes() > limit_bytes);
        let warning = if too_large {
            Warning::ClipLargerThanLimit
        } else {
            Warning::AllStarred
        };
        return RetentionPlan {
            delete: Vec::new(),
            warning: Some(warning),
        };
    }
    RetentionPlan {
        delete,
        warning: None,
    }
}

/// Gruppen für das Löschen: bei getrennten Ordnern je eine mit eigenem Limit, bei gleichem
/// Ordner eine gemeinsame mit dem kleineren Limit.
pub fn retention_groups(
    clip_limit: u64,
    export_limit: u64,
    same_folder: bool,
) -> Vec<(Vec<FolderKind>, u64)> {
    if same_folder {
        vec![(
            vec![FolderKind::Clips, FolderKind::Exports],
            clip_limit.min(export_limit),
        )]
    } else {
        vec![
            (vec![FolderKind::Clips], clip_limit),
            (vec![FolderKind::Exports], export_limit),
        ]
    }
}
