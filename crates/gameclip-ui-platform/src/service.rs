// SPDX-License-Identifier: GPL-3.0-or-later
//! Dienst starten (Design, Datei 58). Den Autostart schaltet der Dienst selbst
//! (`GetAutostart`/`SetAutostart`), weil nur die App „Service“ ihre `StartupTask` sieht.
//!
//! Im Paket startet die Oberfläche den Dienst über seine AUMID (`<PFN>!Service`), damit er im
//! Paketkontext läuft (Fähigkeiten, umgeleitetes `LOCALAPPDATA`). Ohne Paket (Entwicklung) startet
//! sie `gameclipd.exe` neben der eigenen Exe.

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
