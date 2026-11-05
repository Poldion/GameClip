// SPDX-License-Identifier: GPL-3.0-or-later
//! Dienst starten und Autostart schalten (Design, Datei 58).
//!
//! Im Paket startet die Oberfläche den Dienst über seine AUMID (`<PFN>!Service`), damit er im
//! Paketkontext läuft (Fähigkeiten, umgeleitetes `LOCALAPPDATA`). Ohne Paket (Entwicklung) startet
//! sie `gameclipd.exe` neben der eigenen Exe. Autostart gibt es nur im Paket (`StartupTask`).

use gameclip_core::ids::STARTUP_TASK_ID;
use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Shell::{
    AO_NONE, ApplicationActivationManager, IApplicationActivationManager,
};
use windows_core::HSTRING;

use crate::package;

/// Application-Id des Dienstes im Manifest.
const SERVICE_APP_ID: &str = "Service";

/// Startet den Dienst. Kehrt zurück, sobald der Prozess läuft (nicht, wenn die Pipe bereit ist).
pub fn start() -> windows::core::Result<()> {
    match package::family_name() {
        Some(pfn) => unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)?;
            manager.ActivateApplication(
                &HSTRING::from(format!("{pfn}!{SERVICE_APP_ID}")),
                None,
                AO_NONE,
            )?;
            Ok(())
        },
        None => {
            let exe = package::service_exe().ok_or_else(windows::core::Error::from_thread)?;
            std::process::Command::new(exe)
                .spawn()
                .map(drop)
                .map_err(|e| {
                    windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
                })
        }
    }
}

/// Zustand des Autostarts, wie die Einstellungen ihn zeigen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autostart {
    Enabled,
    Disabled,
    /// Vom Benutzer im Task-Manager abgeschaltet; nur dort wieder einschaltbar.
    DisabledByUser,
    /// Durch Richtlinie gesperrt.
    DisabledByPolicy,
    /// Kein Paket (Entwicklung).
    Unavailable,
}

fn task() -> windows::core::Result<StartupTask> {
    StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.join()
}

fn map(state: StartupTaskState) -> Autostart {
    match state {
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => Autostart::Enabled,
        StartupTaskState::DisabledByUser => Autostart::DisabledByUser,
        StartupTaskState::DisabledByPolicy => Autostart::DisabledByPolicy,
        _ => Autostart::Disabled,
    }
}

pub fn autostart() -> Autostart {
    if package::family_name().is_none() {
        return Autostart::Unavailable;
    }
    task()
        .and_then(|t| t.State())
        .map(map)
        .unwrap_or(Autostart::Unavailable)
}

/// Schaltet den Autostart. Einschalten fragt Windows; ist er vom Benutzer abgeschaltet, bleibt er
/// aus und das Ergebnis sagt das.
pub fn set_autostart(on: bool) -> Autostart {
    if package::family_name().is_none() {
        return Autostart::Unavailable;
    }
    let Ok(task) = task() else {
        return Autostart::Unavailable;
    };
    if on {
        task.RequestEnableAsync()
            .and_then(|op| op.join())
            .map(map)
            .unwrap_or(Autostart::Unavailable)
    } else {
        let _ = task.Disable();
        task.State().map(map).unwrap_or(Autostart::Unavailable)
    }
}
