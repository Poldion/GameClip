// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehle, Antworten und Ereignisse auf der Named Pipe (Design, Datei 24).
//!
//! Typ-Regeln: Zeiten `u64` in µs (`*_us`), Größen `u64` in Byte (`*_bytes`),
//! Fortschritt `percent: f32` von 0,0 bis 100,0, `clip_id` = Zeilennummer im Index.
//! Alle `u64` werden in TypeScript als `number` ausgegeben.

use gameclip_core::export_plan::ExportPlan;
use gameclip_core::forecast::Forecast;
use gameclip_core::retention::{ClipStatus, FolderKind, Warning};
use gameclip_core::settings::{Hotkey, Settings};
use gameclip_core::state::ServiceState;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::ErrorCode;

/// Anfrage des Clients. `id` zählt pro Verbindung hoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Request {
    pub id: u32,
    #[serde(flatten)]
    pub cmd: Command,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "body")]
#[ts(export)]
pub enum Command {
    Hello {
        protocol: u32,
        client: String,
    },
    GetStatus,
    GetSettings,
    SetSettings {
        /// JSON-Merge-Patch auf [`Settings`].
        partial: serde_json::Value,
    },
    GetForecast {
        settings: Settings,
    },
    ListDevices,
    GetCapabilities,
    StartBuffer,
    StopBuffer,
    Restart,
    SaveClip,
    PauseMic {
        on: bool,
    },
    SuspendHotkey {
        on: bool,
    },
    TestRegister {
        hotkey: Hotkey,
    },
    SubscribeMicLevel {
        on: bool,
    },
    WatchFolders {
        on: bool,
    },
    ListClips {
        folder: FolderKind,
    },
    GetClip {
        clip_id: u32,
    },
    SetStar {
        clip_id: u32,
        on: bool,
    },
    AdoptClip {
        clip_id: u32,
    },
    DeleteClip {
        clip_id: u32,
    },
    GetFrameTimes {
        clip_id: u32,
    },
    GetExportEstimate {
        clip_id: u32,
        #[ts(type = "number")]
        in_us: u64,
        #[ts(type = "number")]
        out_us: u64,
        #[ts(type = "number")]
        target_bytes: u64,
    },
    StartExportJob {
        clip_id: u32,
        #[ts(type = "number")]
        in_us: u64,
        #[ts(type = "number")]
        out_us: u64,
        profile: ExportProfile,
    },
    CancelJob {
        job_id: u32,
    },
    ListJobs,
    CopyDiagnostics,
    /// Autostart des Dienstes (`StartupTask` der App „Service“). Nur der Dienst selbst sieht die
    /// Aufgabe; die Oberfläche fragt deshalb über die Pipe.
    GetAutostart,
    SetAutostart {
        on: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind")]
#[ts(export)]
pub enum ExportProfile {
    Mix,
    SeparateTracks,
    TargetSize {
        #[ts(type = "number")]
        bytes: u64,
    },
}

/// Antwort des Dienstes. `body` enthält den passenden `…Reply`-Typ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Response {
    pub id: u32,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

impl Response {
    pub fn ok<T: Serialize>(id: u32, body: &T) -> Self {
        Response {
            id,
            ok: true,
            body: Some(serde_json::to_value(body).expect("serialisierbar")),
            error: None,
        }
    }

    pub fn err(id: u32, code: ErrorCode, message: impl Into<String>) -> Self {
        Response {
            id,
            ok: false,
            body: None,
            error: Some(ErrorBody {
                code,
                message: message.into(),
                details: None,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

// --- Antworten ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HelloReply {
    pub protocol: u32,
    pub service_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ServiceStatus {
    pub state: ServiceState,
    pub restrictions: Vec<String>,
    pub mic_paused: bool,
    pub saves_in_flight: u8,
    pub encoder: Option<String>,
    pub capture_resolution: Option<(u32, u32)>,
    #[ts(type = "number")]
    pub buffered_us: u64,
    #[ts(type = "number")]
    pub forecast_total_bytes: u64,
}

pub type GetStatusReply = ServiceStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSettingsReply {
    pub settings: Settings,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SetSettingsReply {
    pub settings: Settings,
}

pub type GetForecastReply = Forecast;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub primary: bool,
    pub width: u32,
    pub height: u32,
    pub hdr: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MicInfo {
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListDevicesReply {
    pub monitors: Vec<MonitorInfo>,
    pub mics: Vec<MicInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetCapabilitiesReply {
    pub separate_tracks: bool,
    pub max_buffer_s: u32,
    pub forecast: bool,
    pub encoders: Vec<String>,
    pub hdr_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TestRegisterReply {
    pub free: bool,
    pub error: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ClipEntry {
    pub clip_id: u32,
    pub tag_id: String,
    pub path: String,
    #[ts(type = "number")]
    pub size_bytes: u64,
    /// RFC 3339, UTC.
    pub created_at: String,
    #[ts(type = "number")]
    pub duration_us: u64,
    pub starred: bool,
    pub has_tracks: bool,
    pub status: ClipStatus,
    pub placeholder: bool,
    pub in_current_folder: bool,
    /// Spiel bzw. Programm aus dem Dateinamen (`VALORANT`), `None` bei „Clip_…“ oder fremden Namen.
    pub program: Option<String>,
}

pub type GetClipReply = ClipEntry;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListClipsReply {
    pub clips: Vec<ClipEntry>,
    #[ts(type = "number")]
    pub used_bytes: u64,
    #[ts(type = "number")]
    pub limit_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetFrameTimesReply {
    #[ts(type = "Array<number>")]
    pub frames_us: Vec<u64>,
    #[ts(type = "number")]
    pub duration_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TooSmall {
    #[ts(type = "number")]
    pub min_target_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetExportEstimateReply {
    pub plan: Option<ExportPlan>,
    pub too_small: Option<TooSmall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StartExportJobReply {
    pub job_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum JobState {
    Running,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JobInfo {
    pub job_id: u32,
    pub clip_id: u32,
    pub profile: ExportProfile,
    pub state: JobState,
    pub percent: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListJobsReply {
    pub jobs: Vec<JobInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CopyDiagnosticsReply {
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AutostartState {
    Enabled,
    Disabled,
    /// Im Task-Manager abgeschaltet; nur dort wieder einschaltbar.
    DisabledByUser,
    DisabledByPolicy,
    /// Kein Paket (Entwicklung) oder Aufgabe nicht gefunden.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AutostartReply {
    pub state: AutostartState,
}

/// Leere Antwort für Befehle ohne Rückgabewert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Empty {}

// --- Ereignisse ---

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "event", content = "body")]
#[ts(export)]
pub enum ServiceEvent {
    StateChanged(ServiceStatus),
    SaveCompleted {
        clip_id: u32,
        path: String,
        #[ts(type = "number")]
        duration_us: u64,
        #[ts(type = "number")]
        size_bytes: u64,
        has_tracks: bool,
        shortened: bool,
    },
    SaveFailed {
        code: ErrorCode,
        message: String,
    },
    NewClip {
        clip_id: u32,
        folder: FolderKind,
    },
    ClipDeleted {
        clip_id: u32,
        folder: FolderKind,
    },
    ClipsChanged {
        folder: FolderKind,
    },
    FormatChanged {
        width: u32,
        height: u32,
        hdr: bool,
    },
    StorageWarning {
        folder: FolderKind,
        #[ts(type = "number")]
        used_bytes: u64,
        #[ts(type = "number")]
        limit_bytes: u64,
        reason: Warning,
    },
    JobProgress {
        job_id: u32,
        percent: f32,
        encoder: String,
    },
    JobCompleted {
        job_id: u32,
        clip_id: u32,
    },
    JobFailed {
        job_id: u32,
        code: ErrorCode,
        message: String,
    },
    MicLevel {
        dbfs: f32,
    },
}
