// SPDX-License-Identifier: GPL-3.0-or-later
//! Schema von `settings.toml` (Schema-Version 1), Standardwerte und Prüfregeln.
//!
//! Unbekannte Schlüssel gehen beim Speichern nicht verloren: [`merge_into`] schreibt die
//! bekannten Werte in die bestehende TOML-Tabelle, statt die Datei neu aufzubauen.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::keymap;

pub const SCHEMA_VERSION: u32 = 1;

pub const BUFFER_SECONDS_MIN: u32 = 15;
pub const BUFFER_SECONDS_MAX: u32 = 300;
pub const GAIN_DB_MIN: f32 = -30.0;
pub const GAIN_DB_MAX: f32 = 12.0;
pub const LIMIT_BYTES_MIN: u64 = 1_000_000_000;
pub const LIMIT_BYTES_MAX: u64 = 2_000_000_000_000;
pub const TARGET_BYTES_MIN: u64 = 1_000_000;
pub const TARGET_BYTES_MAX: u64 = 2_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Resolution {
    #[serde(rename = "720p")]
    P720,
    #[serde(rename = "1080p")]
    P1080,
    #[serde(rename = "1440p")]
    P1440,
    #[serde(rename = "2160p")]
    P2160,
}

impl Resolution {
    pub fn size(self) -> (u32, u32) {
        match self {
            Resolution::P720 => (1280, 720),
            Resolution::P1080 => (1920, 1080),
            Resolution::P1440 => (2560, 1440),
            Resolution::P2160 => (3840, 2160),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Quality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Hotkey {
    /// Wert von `KeyboardEvent.code`, z. B. `"F8"` oder `"KeyC"`.
    pub code: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
}

impl Default for Hotkey {
    fn default() -> Self {
        Hotkey {
            code: "F8".into(),
            ctrl: false,
            alt: false,
            shift: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CaptureSettings {
    pub buffer_seconds: u32,
    pub resolution: Resolution,
    pub fps: u32,
    pub quality: Quality,
    /// Gerätename des Monitors; `None` = Hauptmonitor.
    pub monitor: Option<String>,
    pub cursor: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        CaptureSettings {
            buffer_seconds: 60,
            resolution: Resolution::P1080,
            fps: 60,
            quality: Quality::Medium,
            monitor: None,
            cursor: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GainDb {
    pub system: f32,
    pub mic: f32,
    pub discord: f32,
}

impl Default for GainDb {
    fn default() -> Self {
        GainDb {
            system: 0.0,
            mic: 0.0,
            discord: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct AudioSettings {
    pub mic_enabled: bool,
    /// WASAPI-Endpunkt-ID; `None` = Windows-Standardgerät.
    pub mic_device: Option<String>,
    pub discord_enabled: bool,
    pub gain_db: GainDb,
}

impl Default for AudioSettings {
    fn default() -> Self {
        AudioSettings {
            mic_enabled: true,
            mic_device: None,
            discord_enabled: false,
            gain_db: GainDb::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct StorageSettings {
    /// `None` = Standardordner, zur Laufzeit ermittelt.
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub clip_folder: Option<PathBuf>,
    #[cfg_attr(feature = "ts", ts(type = "string | null"))]
    pub export_folder: Option<PathBuf>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub clip_limit_bytes: u64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub export_limit_bytes: u64,
}

impl Default for StorageSettings {
    fn default() -> Self {
        StorageSettings {
            clip_folder: None,
            export_folder: None,
            clip_limit_bytes: 10_000_000_000,
            export_limit_bytes: 10_000_000_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ExportSettings {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub target_bytes: u64,
}

impl Default for ExportSettings {
    fn default() -> Self {
        ExportSettings {
            target_bytes: 10_000_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct SoundSettings {
    pub enabled: bool,
    pub volume: f32,
}

impl Default for SoundSettings {
    fn default() -> Self {
        SoundSettings {
            enabled: true,
            volume: 0.7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Settings {
    pub schema_version: u32,
    pub first_run_done: bool,
    pub capture: CaptureSettings,
    pub audio: AudioSettings,
    pub hotkey: Hotkey,
    pub storage: StorageSettings,
    pub export: ExportSettings,
    pub sound: SoundSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: SCHEMA_VERSION,
            first_run_done: false,
            capture: CaptureSettings::default(),
            audio: AudioSettings::default(),
            hotkey: Hotkey::default(),
            storage: StorageSettings::default(),
            export: ExportSettings::default(),
            sound: SoundSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct FieldError {
    pub key: String,
    pub reason: String,
}

impl FieldError {
    fn new(key: &str, reason: impl Into<String>) -> Self {
        FieldError {
            key: key.into(),
            reason: reason.into(),
        }
    }
}

impl Settings {
    /// Prüft alle Werte gegen die erlaubten Bereiche aus dem Design.
    /// Ordnerpfade prüft der Dienst selbst (Existenz, Schreibrecht).
    pub fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut errors = Vec::new();
        let c = &self.capture;
        if !(BUFFER_SECONDS_MIN..=BUFFER_SECONDS_MAX).contains(&c.buffer_seconds) {
            errors.push(FieldError::new(
                "capture.buffer_seconds",
                "erlaubt sind 15 bis 300 Sekunden",
            ));
        }
        if c.fps != 30 && c.fps != 60 {
            errors.push(FieldError::new("capture.fps", "erlaubt sind 30 und 60"));
        }
        let gains = [
            ("audio.gain_db.system", self.audio.gain_db.system),
            ("audio.gain_db.mic", self.audio.gain_db.mic),
            ("audio.gain_db.discord", self.audio.gain_db.discord),
        ];
        for (key, value) in gains {
            if !(GAIN_DB_MIN..=GAIN_DB_MAX).contains(&value) {
                errors.push(FieldError::new(key, "erlaubt sind −30 bis +12 dB"));
            }
        }
        if let Err(e) = keymap::to_vk(&self.hotkey) {
            errors.push(FieldError::new("hotkey", e.to_string()));
        }
        for (key, value) in [
            ("storage.clip_limit_bytes", self.storage.clip_limit_bytes),
            (
                "storage.export_limit_bytes",
                self.storage.export_limit_bytes,
            ),
        ] {
            if !(LIMIT_BYTES_MIN..=LIMIT_BYTES_MAX).contains(&value) {
                errors.push(FieldError::new(key, "erlaubt sind 1 GB bis 2000 GB"));
            }
        }
        if !(TARGET_BYTES_MIN..=TARGET_BYTES_MAX).contains(&self.export.target_bytes) {
            errors.push(FieldError::new(
                "export.target_bytes",
                "erlaubt sind 1 MB bis 2000 MB",
            ));
        }
        if !(0.0..=1.0).contains(&self.sound.volume) {
            errors.push(FieldError::new("sound.volume", "erlaubt sind 0,0 bis 1,0"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// `true`, wenn der Wechsel von `old` zu `new` den Puffer neu anlegen muss.
    pub fn pipeline_changed(old: &Settings, new: &Settings) -> bool {
        old.capture != new.capture
            || old.audio.mic_enabled != new.audio.mic_enabled
            || old.audio.mic_device != new.audio.mic_device
            || old.audio.discord_enabled != new.audio.discord_enabled
    }

    /// Anzahl der Einzelspuren nach der Spuren-Regel: System immer, Mikrofon und Discord,
    /// wenn eingeschaltet.
    pub fn separate_track_count(&self) -> u32 {
        1 + u32::from(self.audio.mic_enabled) + u32::from(self.audio.discord_enabled)
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("settings.toml ist kein gültiges TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("settings.toml konnte nicht geschrieben werden: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Ergebnis beim Lesen der Einstellungen.
#[derive(Debug, Clone, PartialEq)]
pub enum Loaded {
    /// Normale Datei; darf überschrieben werden.
    Writable(Settings),
    /// Datei stammt von einer neueren Version; wird nur gelesen, nie überschrieben.
    ReadOnly(Settings),
}

impl Loaded {
    pub fn settings(&self) -> &Settings {
        match self {
            Loaded::Writable(s) | Loaded::ReadOnly(s) => s,
        }
    }
}

/// Liest `settings.toml`. Fehlende Schlüssel bekommen Standardwerte.
pub fn parse(text: &str) -> Result<Loaded, SettingsError> {
    let table: toml::Table = text.parse()?;
    let version = table
        .get("schema_version")
        .and_then(toml::Value::as_integer)
        .unwrap_or(i64::from(SCHEMA_VERSION));
    let settings: Settings = toml::Value::Table(table).try_into()?;
    if version > i64::from(SCHEMA_VERSION) {
        Ok(Loaded::ReadOnly(settings))
    } else {
        Ok(Loaded::Writable(settings))
    }
}

/// Schreibt `settings` in die bestehende Tabelle `existing`. Bekannte Schlüssel werden
/// ersetzt, unbekannte bleiben erhalten. Liefert den neuen Dateiinhalt.
pub fn merge_into(existing: &str, settings: &Settings) -> Result<String, SettingsError> {
    let mut table: toml::Table = if existing.trim().is_empty() {
        toml::Table::new()
    } else {
        existing.parse()?
    };
    let new = toml::Table::try_from(settings)?;
    merge_tables(&mut table, new);
    // TOML kennt kein `null`: ein auf `None` gesetzter Wert fehlt in `new` und muss
    // deshalb aus der alten Tabelle entfernt werden.
    let unset = [
        ("capture", "monitor", settings.capture.monitor.is_none()),
        ("audio", "mic_device", settings.audio.mic_device.is_none()),
        (
            "storage",
            "clip_folder",
            settings.storage.clip_folder.is_none(),
        ),
        (
            "storage",
            "export_folder",
            settings.storage.export_folder.is_none(),
        ),
    ];
    for (section, key, is_none) in unset {
        if is_none && let Some(toml::Value::Table(t)) = table.get_mut(section) {
            t.remove(key);
        }
    }
    Ok(toml::to_string_pretty(&table)?)
}

fn merge_tables(target: &mut toml::Table, source: toml::Table) {
    for (key, value) in source {
        match (target.get_mut(&key), value) {
            (Some(toml::Value::Table(t)), toml::Value::Table(s)) => merge_tables(t, s),
            (_, value) => {
                target.insert(key, value);
            }
        }
    }
}
