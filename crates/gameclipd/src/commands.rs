// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehle der Dienst-Schnittstelle (Design, Datei 24 und 53), ausgeführt in der Hauptschleife.
//!
//! Untermodul von `service`, damit es direkt auf den Zustand des Dienstes zugreift. Jeder Befehl
//! liefert den passenden `…Reply` als JSON oder einen Fehler mit Code.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gameclip_core::export_plan::{PlanError, plan};
use gameclip_core::forecast::forecast;
use gameclip_core::retention::{ClipStatus, FolderKind};
use gameclip_core::settings::{FieldError, Settings, merge_into};
use gameclip_core::state::{Restrictions, StateEvent};
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::messages::{
    ClipEntry, Command, CopyDiagnosticsReply, Empty, GetCapabilitiesReply, GetExportEstimateReply,
    GetFrameTimesReply, GetSettingsReply, ListClipsReply, ListDevicesReply, ListJobsReply, MicInfo,
    MonitorInfo, ServiceStatus, SetSettingsReply, StartExportJobReply, TooSmall,
};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;

use super::Service;
use crate::index::ClipRow;
use crate::paths::Paths;

pub type CmdResult = Result<Value, CmdError>;

/// Fehler eines Befehls; `details` z. B. die Feldfehler von `SetSettings`.
#[derive(Debug)]
pub struct CmdError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

impl CmdError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        CmdError {
            code,
            message: message.into(),
            details: None,
        }
    }
}

impl From<(ErrorCode, String)> for CmdError {
    fn from((code, message): (ErrorCode, String)) -> Self {
        CmdError::new(code, message)
    }
}

fn ok<T: serde::Serialize>(v: &T) -> CmdResult {
    Ok(serde_json::to_value(v).expect("serialisierbar"))
}

fn empty() -> CmdResult {
    ok(&Empty {})
}

fn internal(e: impl std::fmt::Display) -> (ErrorCode, String) {
    (ErrorCode::Internal, e.to_string())
}

/// Anforderungen je Verbindung; die Wirkung gilt, solange mindestens eine sie stellt.
#[derive(Debug, Default)]
pub struct Conns {
    pub suspend_hotkey: HashSet<u32>,
    pub watch_folders: HashSet<u32>,
    pub mic_level: HashSet<u32>,
}

impl Service {
    pub(super) fn handle(&mut self, conn: u32, cmd: Command) -> CmdResult {
        match cmd {
            Command::Hello { .. } => Err(CmdError::new(
                ErrorCode::InvalidArgument,
                "Hello nur einmal",
            )),
            Command::GetStatus => ok(&self.status()),
            Command::GetSettings => ok(&GetSettingsReply {
                settings: self.settings.clone(),
                read_only: self.settings_read_only,
            }),
            Command::SetSettings { partial } => self.set_settings(partial),
            Command::GetForecast { settings } => ok(&forecast(&settings, super::physical_ram())),
            Command::ListDevices => self.list_devices(),
            Command::GetCapabilities => ok(&GetCapabilitiesReply {
                separate_tracks: true,
                max_buffer_s: gameclip_core::settings::BUFFER_SECONDS_MAX,
                forecast: true,
                encoders: self
                    .recorder
                    .as_ref()
                    .map(|r| vec![r.encoder().0.to_string()])
                    .unwrap_or_default(),
                hdr_supported: false,
            }),
            Command::StartBuffer => {
                self.feed(StateEvent::StartBuffer);
                empty()
            }
            Command::StopBuffer => {
                self.feed(StateEvent::StopBuffer);
                empty()
            }
            Command::Restart => {
                self.feed(StateEvent::Restart);
                empty()
            }
            Command::SaveClip => {
                self.press(gameclip_capture_win::pacer::qpc_us());
                empty()
            }
            Command::PauseMic { on } => {
                self.feed(StateEvent::MicPause(on));
                empty()
            }
            Command::SuspendHotkey { on } => self.suspend_hotkey(conn, on),
            Command::TestRegister { hotkey } => ok(&self.msg.test_register(&hotkey)),
            Command::SubscribeMicLevel { on } => {
                set_flag(&mut self.conns.mic_level, conn, on);
                empty()
            }
            Command::WatchFolders { on } => self.watch_folders(conn, on),
            Command::ListClips { folder } => self.list_clips(folder),
            Command::GetClip { clip_id } => {
                let row = self.row(clip_id)?;
                ok(&self.clip_entry(&row))
            }
            Command::SetStar { clip_id, on } => {
                self.row(clip_id)?;
                self.index.set_star(clip_id, on).map_err(internal)?;
                empty()
            }
            Command::AdoptClip { clip_id } => self.adopt_clip(clip_id),
            Command::DeleteClip { clip_id } => self.delete_clip(clip_id),
            Command::GetFrameTimes { clip_id } => self.frame_times(clip_id),
            Command::GetExportEstimate {
                clip_id,
                in_us,
                out_us,
                target_bytes,
            } => {
                let row = self.row(clip_id)?;
                let info = gameclip_media::demux::video_info(&row.path)
                    .map_err(|e| (ErrorCode::NotFound, e.to_string()))?;
                let reply = match plan(
                    out_us.saturating_sub(in_us),
                    target_bytes,
                    (info.width, info.height, info.fps),
                ) {
                    Ok(p) => GetExportEstimateReply {
                        plan: Some(p),
                        too_small: None,
                    },
                    Err(PlanError::TooSmall { min_target_bytes }) => GetExportEstimateReply {
                        plan: None,
                        too_small: Some(TooSmall { min_target_bytes }),
                    },
                };
                ok(&reply)
            }
            Command::StartExportJob {
                clip_id,
                in_us,
                out_us,
                profile,
            } => {
                let row = self.row(clip_id)?;
                if out_us <= in_us {
                    return Err(CmdError::new(ErrorCode::InvalidArgument, "out_us ≤ in_us"));
                }
                ok(&StartExportJobReply {
                    job_id: self.jobs.start(row, in_us, out_us, profile),
                })
            }
            Command::CancelJob { job_id } => {
                self.jobs.cancel(job_id);
                empty()
            }
            Command::ListJobs => ok(&ListJobsReply {
                jobs: self.jobs.list(),
            }),
            Command::CopyDiagnostics => self.copy_diagnostics(),
        }
    }

    /// Verbindung getrennt: ihre Anforderungen zurücknehmen.
    pub(super) fn disconnected(&mut self, conn: u32) {
        let suspended = self.conns.suspend_hotkey.remove(&conn);
        if suspended && self.conns.suspend_hotkey.is_empty() {
            self.feed(StateEvent::UiDisconnectedWhileSuspended);
        }
        if self.conns.watch_folders.remove(&conn) && self.conns.watch_folders.is_empty() {
            self.set_watching(false);
        }
        self.conns.mic_level.remove(&conn);
    }

    pub(super) fn status(&self) -> ServiceStatus {
        let r = self.machine.restrictions;
        let mut restrictions = Vec::new();
        for (flag, name) in [
            (Restrictions::HOTKEY_INACTIVE, "hotkey_inactive"),
            (Restrictions::CPU_ENCODER, "cpu_encoder"),
            (Restrictions::HDR_ACTIVE, "hdr_active"),
        ] {
            if r.contains(flag) {
                restrictions.push(name.to_string());
            }
        }
        ServiceStatus {
            state: self.machine.state,
            restrictions,
            mic_paused: self.machine.mic_paused,
            saves_in_flight: self.machine.saves_in_flight,
            encoder: self.recorder.as_ref().map(|r| r.encoder().0.to_string()),
            capture_resolution: self
                .recorder
                .as_ref()
                .map(|_| self.settings.capture.resolution.size()),
            buffered_us: self.ring.buffered_us(),
            forecast_total_bytes: forecast(&self.settings, super::physical_ram()).total_bytes,
        }
    }

    fn row(&self, clip_id: u32) -> Result<ClipRow, (ErrorCode, String)> {
        self.index
            .get(clip_id)
            .map_err(internal)?
            .ok_or_else(|| (ErrorCode::NotFound, format!("Clip {clip_id} unbekannt")))
    }

    fn folder_dir(&self, folder: FolderKind) -> &Path {
        match folder {
            FolderKind::Clips => &self.paths.clips,
            FolderKind::Exports => &self.paths.exports,
        }
    }

    pub(super) fn clip_entry(&self, row: &ClipRow) -> ClipEntry {
        ClipEntry {
            clip_id: row.row_id,
            tag_id: row.tag_id.clone(),
            path: row.path.to_string_lossy().into_owned(),
            size_bytes: row.size_bytes,
            created_at: row.created_at.format(&Rfc3339).unwrap_or_default(),
            duration_us: row.duration_us,
            starred: row.starred,
            has_tracks: row.has_tracks,
            status: row.status,
            placeholder: row.placeholder,
            in_current_folder: row.path.parent() == Some(self.folder_dir(row.folder)),
        }
    }

    fn list_clips(&mut self, folder: FolderKind) -> CmdResult {
        // Abgleich bei Bedarf (T6): vor jeder Liste.
        self.reconcile_folder(folder);
        let rows = self.index.list(folder).map_err(internal)?;
        let entries: Vec<ClipEntry> = rows.iter().map(|r| self.clip_entry(r)).collect();
        let used_bytes = entries
            .iter()
            .filter(|e| e.status == ClipStatus::Ok && e.in_current_folder && !e.placeholder)
            .map(|e| e.size_bytes)
            .sum();
        let limit_bytes = match folder {
            FolderKind::Clips => self.settings.storage.clip_limit_bytes,
            FolderKind::Exports => self.settings.storage.export_limit_bytes,
        };
        ok(&ListClipsReply {
            clips: entries,
            used_bytes,
            limit_bytes,
        })
    }

    fn delete_clip(&mut self, clip_id: u32) -> CmdResult {
        let row = self.row(clip_id)?;
        if self.index.is_read_only() {
            return Err(CmdError::new(
                ErrorCode::NotReady,
                "Index ist schreibgeschützt",
            ));
        }
        if row.path.exists() {
            std::fs::remove_file(&row.path).map_err(internal)?;
        }
        if row.folder == FolderKind::Clips && row.status == ClipStatus::Ok && !row.tag_id.is_empty()
        {
            let _ = std::fs::remove_file(self.paths.track_file(&row.tag_id));
        }
        self.index.remove(clip_id).map_err(internal)?;
        tracing::info!("Gelöscht (Nutzer): {}", row.path.display());
        self.events
            .send(gameclip_protocol::messages::ServiceEvent::ClipDeleted {
                clip_id,
                folder: row.folder,
            });
        empty()
    }

    fn list_devices(&self) -> CmdResult {
        let monitors = gameclip_gpu::list_monitors()
            .map_err(internal)?
            .into_iter()
            .enumerate()
            .map(|(i, m)| MonitorInfo {
                name: format!("Monitor {} ({}×{})", i + 1, m.width, m.height),
                id: m.id,
                primary: m.primary,
                width: m.width,
                height: m.height,
                hdr: m.hdr,
            })
            .collect();
        let default = gameclip_capture_win::audio::mic::default_device_id().ok();
        let mics = gameclip_capture_win::audio::mic::list_devices()
            .map_err(internal)?
            .into_iter()
            .map(|d| MicInfo {
                default: default.as_deref() == Some(d.id.as_str()),
                id: d.id,
                name: d.name,
            })
            .collect();
        ok(&ListDevicesReply { monitors, mics })
    }

    fn suspend_hotkey(&mut self, conn: u32, on: bool) -> CmdResult {
        let was = !self.conns.suspend_hotkey.is_empty();
        set_flag(&mut self.conns.suspend_hotkey, conn, on);
        let now = !self.conns.suspend_hotkey.is_empty();
        if now && !was {
            self.msg.unregister();
        } else if was && !now {
            self.feed(StateEvent::UiDisconnectedWhileSuspended);
        }
        empty()
    }

    fn watch_folders(&mut self, conn: u32, on: bool) -> CmdResult {
        let was = !self.conns.watch_folders.is_empty();
        set_flag(&mut self.conns.watch_folders, conn, on);
        let now = !self.conns.watch_folders.is_empty();
        if now != was {
            self.set_watching(now);
        }
        empty()
    }
}

impl Service {
    fn set_settings(&mut self, partial: Value) -> CmdResult {
        if self.settings_read_only {
            return Err(CmdError::new(
                ErrorCode::NotReady,
                "Einstellungen stammen von einer neueren Version und werden nur gelesen",
            ));
        }
        let mut merged = serde_json::to_value(&self.settings).map_err(internal)?;
        merge_patch(&mut merged, &partial);
        let new: Settings = serde_json::from_value(merged)
            .map_err(|e| CmdError::new(ErrorCode::InvalidArgument, e.to_string()))?;
        let mut errors = new.validate().err().unwrap_or_default();
        for (key, dir) in [
            ("storage.clip_folder", &new.storage.clip_folder),
            ("storage.export_folder", &new.storage.export_folder),
        ] {
            if let Some(d) = dir
                && let Err(reason) = check_folder(d)
            {
                errors.push(FieldError {
                    key: key.into(),
                    reason,
                });
            }
        }
        if !errors.is_empty() {
            return Err(CmdError {
                code: ErrorCode::InvalidArgument,
                message: "Ungültige Einstellungen".into(),
                details: serde_json::to_value(&errors).ok(),
            });
        }
        // Speichern: unbekannte Schlüssel der Datei bleiben (merge_into), atomar über .tmp.
        let existing = std::fs::read_to_string(&self.paths.settings).unwrap_or_default();
        let text = merge_into(&existing, &new).map_err(internal)?;
        if let Some(dir) = self.paths.settings.parent() {
            std::fs::create_dir_all(dir).map_err(internal)?;
        }
        let tmp = self.paths.settings.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(internal)?;
        std::fs::rename(&tmp, &self.paths.settings).map_err(internal)?;

        let old = std::mem::replace(&mut self.settings, new);
        self.apply_settings(&old);
        ok(&SetSettingsReply {
            settings: self.settings.clone(),
        })
    }

    /// Wirkung geänderter Einstellungen.
    fn apply_settings(&mut self, old: &Settings) {
        if old.hotkey != self.settings.hotkey && self.conns.suspend_hotkey.is_empty() {
            self.apply(&gameclip_core::state::Effect::ReRegisterHotkey);
        }
        if old.storage.clip_folder != self.settings.storage.clip_folder
            || old.storage.export_folder != self.settings.storage.export_folder
        {
            let settings_path = self.paths.settings.clone();
            self.paths = Paths::resolve(&self.settings);
            self.paths.settings = settings_path;
            let _ = self.paths.create_all();
            self.jobs.set_paths(self.paths.clone());
            if self.conns.watch_folders.is_empty() {
                self.set_watching(false);
            } else {
                self.set_watching(false);
                self.set_watching(true);
            }
            self.reconcile_folder(FolderKind::Clips);
            self.reconcile_folder(FolderKind::Exports);
        }
        if old.storage != self.settings.storage {
            self.enforce_limit(None);
        }
        if Settings::pipeline_changed(old, &self.settings) {
            self.feed(StateEvent::SettingsChanged { pipeline: true });
        }
    }

    fn adopt_clip(&mut self, clip_id: u32) -> CmdResult {
        let row = self.row(clip_id)?;
        if row.status != ClipStatus::Foreign {
            return Err(CmdError::new(
                ErrorCode::InvalidArgument,
                "nur fremde Clips",
            ));
        }
        let tag = uuid::Uuid::new_v4().to_string();
        let mut part = row.path.as_os_str().to_owned();
        part.push(".part");
        let part = std::path::PathBuf::from(part);
        let result =
            gameclip_media::mux::remux_with_tags(&row.path, &part, &[("gameclip_id", &tag)])
                .map_err(|e| CmdError::new(ErrorCode::EncoderFailed, e.to_string()))
                .and_then(|()| {
                    std::fs::rename(&part, &row.path)
                        .map_err(|e| CmdError::new(ErrorCode::DiskFull, e.to_string()))
                });
        if let Err(e) = result {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        let size = std::fs::metadata(&row.path)
            .map(|m| m.len())
            .unwrap_or(row.size_bytes);
        self.index.adopt(clip_id, &tag, size).map_err(internal)?;
        self.frames.forget(clip_id);
        let row = self.row(clip_id)?;
        ok(&self.clip_entry(&row))
    }

    fn frame_times(&mut self, clip_id: u32) -> CmdResult {
        let row = self.row(clip_id)?;
        let frames_us = self
            .frames
            .get(clip_id, &row.path)
            .map_err(|e| CmdError::new(ErrorCode::NotFound, e.to_string()))?;
        ok(&GetFrameTimesReply {
            duration_us: row.duration_us,
            frames_us,
        })
    }

    fn copy_diagnostics(&self) -> CmdResult {
        let text = crate::diagnostics::report(&crate::diagnostics::DiagInput {
            settings: &self.settings,
            state: format!("{:?}", self.machine.state),
            encoder: self.recorder.as_ref().map(|r| r.encoder().0.to_string()),
            ring: Some(self.ring.stats()),
            buffered_us: self.ring.buffered_us(),
            log_dir: &self.paths.logs,
        });
        ok(&CopyDiagnosticsReply { text })
    }

    /// Ordnerüberwachung an oder aus (nur solange eine Verbindung sie anfordert).
    pub(super) fn set_watching(&mut self, on: bool) {
        self.watcher = None;
        if on {
            self.watcher = Some(crate::watcher::Watcher::start(
                &[
                    (self.paths.clips.clone(), FolderKind::Clips),
                    (self.paths.exports.clone(), FolderKind::Exports),
                ],
                self.watch_tx.clone(),
            ));
        }
    }

    /// Abgleich eines Ordners; bei Änderungen `ClipsChanged`.
    pub(super) fn reconcile_folder(&mut self, folder: FolderKind) {
        let dir = self.folder_dir(folder).to_path_buf();
        match crate::retention_exec::reconcile(
            &self.index,
            &self.paths,
            &dir,
            folder,
            &crate::retention_exec::WinProbe,
        ) {
            Ok(stats) => {
                if stats != Default::default() {
                    tracing::info!("Abgleich {folder:?}: {stats:?}");
                    for id in &stats.removed {
                        self.frames.forget(*id);
                    }
                    self.events
                        .send(gameclip_protocol::messages::ServiceEvent::ClipsChanged { folder });
                }
            }
            Err(e) => tracing::warn!("Abgleich {folder:?}: {e}"),
        }
    }
}

/// RFC 7386: Objekte werden zusammengeführt, `null` entfernt, alles andere ersetzt.
pub fn merge_patch(target: &mut Value, patch: &Value) {
    if let Value::Object(p) = patch {
        if !target.is_object() {
            *target = Value::Object(Default::default());
        }
        let t = target.as_object_mut().expect("Objekt");
        for (k, v) in p {
            if v.is_null() {
                t.remove(k);
            } else {
                merge_patch(t.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
    } else {
        *target = patch.clone();
    }
}

/// Ordnerregel (Design, Datei 53): existiert, beschreibbar, nicht unter `%WINDIR%` oder
/// `%ProgramFiles%`, nicht der Laufwerksstamm.
pub fn check_folder(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err("Ordner existiert nicht".into());
    }
    if dir.parent().is_none()
        || dir.components().count() <= 2 && dir.to_string_lossy().ends_with('\\')
    {
        return Err("Laufwerksstamm ist nicht erlaubt".into());
    }
    // Pfadweise vergleichen (nicht als Text), ohne Groß-/Kleinschreibung.
    let lower = PathBuf::from(dir.to_string_lossy().to_lowercase());
    for var in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
            && lower.starts_with(v.to_lowercase())
        {
            return Err(format!("Ordner unter %{var}% ist nicht erlaubt"));
        }
    }
    let probe = dir.join(format!(".gameclip-write-test-{}", std::process::id()));
    std::fs::write(&probe, b"").map_err(|e| format!("nicht beschreibbar: {e}"))?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}
fn set_flag(set: &mut HashSet<u32>, conn: u32, on: bool) {
    if on {
        set.insert(conn);
    } else {
        set.remove(&conn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_patch_follows_rfc7386() {
        let mut t = json!({"a": {"b": 1, "c": 2}, "d": 3});
        merge_patch(&mut t, &json!({"a": {"b": 5, "c": null}, "e": [1]}));
        assert_eq!(t, json!({"a": {"b": 5}, "d": 3, "e": [1]}));
        merge_patch(&mut t, &json!({"a": 7}));
        assert_eq!(t["a"], json!(7));
    }

    #[test]
    fn folder_rule() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_folder(dir.path()).is_ok());
        assert!(check_folder(&dir.path().join("fehlt")).is_err());
        assert!(check_folder(Path::new("C:\\")).is_err());
        if let Ok(win) = std::env::var("WINDIR") {
            assert!(check_folder(Path::new(&win)).is_err());
        }
    }

    #[test]
    fn flags_per_connection() {
        let mut s = HashSet::new();
        set_flag(&mut s, 1, true);
        set_flag(&mut s, 2, true);
        set_flag(&mut s, 1, false);
        assert_eq!(s.len(), 1);
        assert!(s.contains(&2));
    }
}
