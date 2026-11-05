// SPDX-License-Identifier: GPL-3.0-or-later
//! Autostart über die `StartupTask` der App „Service“ (Design, Datei 58).
//!
//! `StartupTask::GetAsync` findet nur die Aufgaben der aufrufenden App. Die Aufgabe hängt an
//! „Service“, also schaltet sie der Dienst; die Oberfläche (App „GameClip“) fragt über die Pipe
//! (`GetAutostart`, `SetAutostart`). Geprüft 2026-11-05 mit `Invoke-CommandInDesktopPackage`:
//! aus „GameClip“ liefert `GetForCurrentPackageAsync` eine leere Liste, aus „Service“ die Aufgabe.

use gameclip_core::ids::STARTUP_TASK_ID;
use gameclip_protocol::messages::AutostartState;
use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows_core::HSTRING;

fn task() -> windows::core::Result<StartupTask> {
    StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.join()
}

fn map(state: StartupTaskState) -> AutostartState {
    match state {
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => AutostartState::Enabled,
        StartupTaskState::DisabledByUser => AutostartState::DisabledByUser,
        StartupTaskState::DisabledByPolicy => AutostartState::DisabledByPolicy,
        _ => AutostartState::Disabled,
    }
}

fn log_err<T>(what: &str, r: windows::core::Result<T>) -> Option<T> {
    r.map_err(|e| tracing::warn!("Autostart: {what} fehlgeschlagen: {e}"))
        .ok()
}

pub fn state() -> AutostartState {
    if crate::toast::package_family_name().is_none() {
        return AutostartState::Unavailable;
    }
    log_err("Abfrage", task().and_then(|t| t.State()))
        .map(map)
        .unwrap_or(AutostartState::Unavailable)
}

/// Schaltet den Autostart. Ist er im Task-Manager abgeschaltet, bleibt er aus und das Ergebnis
/// sagt das (`DisabledByUser`).
pub fn set(on: bool) -> AutostartState {
    if crate::toast::package_family_name().is_none() {
        return AutostartState::Unavailable;
    }
    let Some(task) = log_err("Abfrage", task()) else {
        return AutostartState::Unavailable;
    };
    if on {
        log_err(
            "Einschalten",
            task.RequestEnableAsync().and_then(|op| op.join()),
        )
        .map(map)
        .unwrap_or(AutostartState::Unavailable)
    } else {
        let _ = log_err("Ausschalten", task.Disable());
        log_err("Abfrage", task.State())
            .map(map)
            .unwrap_or(AutostartState::Unavailable)
    }
}
