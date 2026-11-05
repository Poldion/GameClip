// SPDX-License-Identifier: GPL-3.0-or-later
//! SQLite-Index der Clips und Exporte (Design, Datei 45).
//!
//! WAL-Modus. Ist `schema_version` > 1 (neuere App-Version), wird nur gelesen und nichts
//! gelöscht. Lässt sich die Datei nicht als Datenbank lesen, wird sie nach
//! `index.sqlite.broken` umbenannt und neu angelegt; der Abgleich (`reconcile`, folgt) stellt die
//! Einträge aus den Datei-Tags wieder her.
//!
//! Noch offen in M0: `reconcile` und `adopt`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gameclip_core::retention::{ClipStatus, FolderKind};
use rusqlite::{Connection, OptionalExtension, Row, params};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const SCHEMA: &str = include_str!("../sql/schema.sql");
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Index ist schreibgeschützt (Schema einer neueren Version)")]
    ReadOnly,
    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, IndexError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipRow {
    pub row_id: u32,
    pub tag_id: String,
    pub folder: FolderKind,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created_at: OffsetDateTime,
    pub duration_us: u64,
    pub starred: bool,
    pub has_tracks: bool,
    pub source_row_id: Option<u32>,
    pub placeholder: bool,
    pub missing_count: u32,
    pub status: ClipStatus,
}

/// Neuer Eintrag mit Status `pending` (vor dem ersten sichtbaren Byte der Datei).
pub struct NewClip<'a> {
    pub tag_id: &'a str,
    pub folder: FolderKind,
    pub path: &'a Path,
    pub created_at: OffsetDateTime,
    pub source_row_id: Option<u32>,
}

pub struct Index {
    conn: Mutex<Connection>,
    read_only: bool,
    /// Gesetzt, wenn beim Öffnen eine beschädigte Datei ersetzt wurde.
    pub recovered_from: Option<PathBuf>,
}

fn folder_str(f: FolderKind) -> &'static str {
    match f {
        FolderKind::Clips => "clips",
        FolderKind::Exports => "exports",
    }
}

fn status_str(s: ClipStatus) -> &'static str {
    match s {
        ClipStatus::Pending => "pending",
        ClipStatus::Ok => "ok",
        ClipStatus::Foreign => "foreign",
    }
}

fn row_to_clip(r: &Row<'_>) -> rusqlite::Result<ClipRow> {
    let folder: String = r.get("folder")?;
    let status: String = r.get("status")?;
    let created: String = r.get("created_at")?;
    Ok(ClipRow {
        row_id: r.get("row_id")?,
        tag_id: r.get("tag_id")?,
        folder: if folder == "exports" {
            FolderKind::Exports
        } else {
            FolderKind::Clips
        },
        path: PathBuf::from(r.get::<_, String>("path")?),
        size_bytes: r.get::<_, i64>("size_bytes")? as u64,
        created_at: OffsetDateTime::parse(&created, &Rfc3339).unwrap_or(OffsetDateTime::UNIX_EPOCH),
        duration_us: r.get::<_, i64>("duration_us")? as u64,
        starred: r.get("starred")?,
        has_tracks: r.get("has_tracks")?,
        source_row_id: r.get("source_row_id")?,
        placeholder: r.get("placeholder")?,
        missing_count: r.get("missing_count")?,
        status: match status.as_str() {
            "ok" => ClipStatus::Ok,
            "foreign" => ClipStatus::Foreign,
            _ => ClipStatus::Pending,
        },
    })
}

impl Index {
    /// Öffnet oder legt den Index an; ersetzt eine beschädigte Datei.
    pub fn open(path: &Path) -> Result<Self> {
        match Self::try_open(path) {
            Ok((conn, read_only)) => Ok(Index {
                conn: Mutex::new(conn),
                read_only,
                recovered_from: None,
            }),
            Err(_) if path.exists() => {
                let broken = path.with_extension("sqlite.broken");
                let _ = std::fs::remove_file(&broken);
                std::fs::rename(path, &broken)?;
                for suffix in ["-wal", "-shm"] {
                    let mut side = path.as_os_str().to_owned();
                    side.push(suffix);
                    let _ = std::fs::remove_file(PathBuf::from(side));
                }
                let (conn, read_only) = Self::try_open(path)?;
                Ok(Index {
                    conn: Mutex::new(conn),
                    read_only,
                    recovered_from: Some(broken),
                })
            }
            Err(e) => Err(e),
        }
    }

    fn try_open(path: &Path) -> Result<(Connection, bool)> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let has_meta: bool = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n > 0)?;
        if !has_meta {
            conn.execute_batch(SCHEMA)?;
        }
        let version: u32 = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'schema_version'",
                [],
                |r| r.get::<_, String>(0),
            )?
            .parse()
            .unwrap_or(u32::MAX);
        Ok((conn, version > SCHEMA_VERSION))
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn write<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        if self.read_only {
            return Err(IndexError::ReadOnly);
        }
        let conn = self.conn.lock().expect("Index-Mutex");
        Ok(f(&conn)?)
    }

    fn read<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        let conn = self.conn.lock().expect("Index-Mutex");
        Ok(f(&conn)?)
    }

    pub fn insert_pending(&self, c: &NewClip<'_>) -> Result<u32> {
        let created = c
            .created_at
            .to_offset(time::UtcOffset::UTC)
            .format(&Rfc3339)
            .expect("RFC 3339");
        self.write(|db| {
            db.execute(
                "INSERT INTO clips (tag_id, folder, path, size_bytes, created_at, duration_us,
                                    source_row_id, status)
                 VALUES (?1, ?2, ?3, 0, ?4, 0, ?5, 'pending')",
                params![
                    c.tag_id,
                    folder_str(c.folder),
                    c.path.to_string_lossy(),
                    created,
                    c.source_row_id
                ],
            )?;
            Ok(db.last_insert_rowid() as u32)
        })
    }

    /// Nach dem Umbenennen: Eintrag gültig, Pfad ist der endgültige.
    pub fn mark_ok(
        &self,
        row_id: u32,
        path: &Path,
        size_bytes: u64,
        duration_us: u64,
        has_tracks: bool,
    ) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET status = 'ok', path = ?2, size_bytes = ?3, duration_us = ?4,
                                  has_tracks = ?5
                 WHERE row_id = ?1",
                params![
                    row_id,
                    path.to_string_lossy(),
                    size_bytes as i64,
                    duration_us as i64,
                    has_tracks
                ],
            )?;
            Ok(())
        })
    }

    pub fn set_status(&self, row_id: u32, status: ClipStatus) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET status = ?2 WHERE row_id = ?1",
                params![row_id, status_str(status)],
            )?;
            Ok(())
        })
    }

    pub fn remove(&self, row_id: u32) -> Result<()> {
        self.write(|db| {
            db.execute("DELETE FROM clips WHERE row_id = ?1", params![row_id])?;
            Ok(())
        })
    }

    pub fn set_star(&self, row_id: u32, starred: bool) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET starred = ?2 WHERE row_id = ?1",
                params![row_id, starred],
            )?;
            Ok(())
        })
    }

    pub fn get(&self, row_id: u32) -> Result<Option<ClipRow>> {
        self.read(|db| {
            db.query_row(
                "SELECT * FROM clips WHERE row_id = ?1",
                params![row_id],
                row_to_clip,
            )
            .optional()
        })
    }

    /// Alle Einträge eines Ordners, neueste zuerst.
    pub fn list(&self, folder: FolderKind) -> Result<Vec<ClipRow>> {
        self.read(|db| {
            let mut stmt = db.prepare(
                "SELECT * FROM clips WHERE folder = ?1 ORDER BY created_at DESC, row_id DESC",
            )?;
            stmt.query_map(params![folder_str(folder)], row_to_clip)?
                .collect()
        })
    }

    /// Alle Einträge aller Ordner.
    pub fn list_all(&self) -> Result<Vec<ClipRow>> {
        let mut all = self.list(FolderKind::Clips)?;
        all.extend(self.list(FolderKind::Exports)?);
        Ok(all)
    }

    pub fn find_by_path(&self, path: &Path) -> Result<Option<ClipRow>> {
        self.read(|db| {
            db.query_row(
                "SELECT * FROM clips WHERE path = ?1",
                params![path.to_string_lossy()],
                row_to_clip,
            )
            .optional()
        })
    }

    /// Die `ok`-Zeile mit diesem Tag (höchstens eine, Design „Eigentum“).
    pub fn find_ok_by_tag(&self, tag_id: &str) -> Result<Option<ClipRow>> {
        self.read(|db| {
            db.query_row(
                "SELECT * FROM clips WHERE tag_id = ?1 AND status = 'ok' LIMIT 1",
                params![tag_id],
                row_to_clip,
            )
            .optional()
        })
    }

    /// Fertige Zeile direkt anlegen (Abgleich: übernommene oder fremde Dateien).
    pub fn insert_row(&self, c: &ClipRow) -> Result<u32> {
        let created = c
            .created_at
            .to_offset(time::UtcOffset::UTC)
            .format(&Rfc3339)
            .expect("RFC 3339");
        self.write(|db| {
            db.execute(
                "INSERT INTO clips (tag_id, folder, path, size_bytes, created_at, duration_us,
                                    starred, has_tracks, source_row_id, placeholder, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    c.tag_id,
                    folder_str(c.folder),
                    c.path.to_string_lossy(),
                    c.size_bytes as i64,
                    created,
                    c.duration_us as i64,
                    c.starred,
                    c.has_tracks,
                    c.source_row_id,
                    c.placeholder,
                    status_str(c.status)
                ],
            )?;
            Ok(db.last_insert_rowid() as u32)
        })
    }

    pub fn update_file(&self, row_id: u32, path: &Path, size_bytes: u64) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET path = ?2, size_bytes = ?3, missing_count = 0 WHERE row_id = ?1",
                params![row_id, path.to_string_lossy(), size_bytes as i64],
            )?;
            Ok(())
        })
    }

    /// Übernommene Kopie: neuer Tag, `ok`, mit Stern, ohne Einzelspuren (Design, Datei 45).
    pub fn adopt(&self, row_id: u32, new_tag: &str, size_bytes: u64) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET tag_id = ?2, status = 'ok', starred = 1, has_tracks = 0,
                                  size_bytes = ?3
                 WHERE row_id = ?1",
                params![row_id, new_tag, size_bytes as i64],
            )?;
            Ok(())
        })
    }

    pub fn set_missing(&self, row_id: u32, missing_count: u32) -> Result<()> {
        self.write(|db| {
            db.execute(
                "UPDATE clips SET missing_count = ?2 WHERE row_id = ?1",
                params![row_id, missing_count],
            )?;
            Ok(())
        })
    }

    /// Neuester gültiger Clip (Tray-Aktion „Letzten Clip kopieren“).
    pub fn latest_ok(&self, folder: FolderKind) -> Result<Option<ClipRow>> {
        Ok(self
            .list(folder)?
            .into_iter()
            .find(|c| c.status == ClipStatus::Ok))
    }
}
