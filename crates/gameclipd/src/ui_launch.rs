// SPDX-License-Identifier: GPL-3.0-or-later
//! Oberfläche aus dem Tray öffnen („Galerie öffnen“, „Einstellungen“). Im Paket über die AUMID
//! `<PFN>!GameClip`, damit sie im Paketkontext läuft; ohne Paket `gameclip.exe` neben dem Dienst.
//! Läuft die Oberfläche schon, holt sie ihr Fenster nach vorn und wechselt die Ansicht.

use windows::Win32::System::Com::{
    CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Shell::{
    AO_NONE, ApplicationActivationManager, IApplicationActivationManager,
};
use windows_core::HSTRING;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Gallery,
    Settings,
}

impl View {
    fn arg(self) -> &'static str {
        match self {
            View::Gallery => "--view=gallery",
            View::Settings => "--view=settings",
        }
    }
}

/// Startet die Oberfläche auf einem eigenen Thread (COM, kann kurz blockieren).
pub fn open(view: View) {
    std::thread::spawn(move || {
        if let Err(e) = launch(view) {
            tracing::warn!("Oberfläche startet nicht: {e}");
        }
    });
}

fn launch(view: View) -> Result<(), String> {
    match crate::toast::package_family_name() {
        Some(pfn) => unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let manager: IApplicationActivationManager =
                CoCreateInstance(&ApplicationActivationManager, None, CLSCTX_LOCAL_SERVER)
                    .map_err(|e| e.to_string())?;
            manager
                .ActivateApplication(
                    &HSTRING::from(format!("{pfn}!GameClip")),
                    &HSTRING::from(view.arg()),
                    AO_NONE,
                )
                .map(drop)
                .map_err(|e| e.to_string())
        },
        None => {
            let exe = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .with_file_name("gameclip.exe");
            std::process::Command::new(exe)
                .arg(view.arg())
                .spawn()
                .map(drop)
                .map_err(|e| e.to_string())
        }
    }
}
