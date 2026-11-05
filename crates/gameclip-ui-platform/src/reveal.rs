// SPDX-License-Identifier: GPL-3.0-or-later
//! „Im Ordner anzeigen“: Explorer öffnen und die Datei markieren.

use std::path::Path;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{SHOpenFolderAndSelectItems, SHParseDisplayName};
use windows_core::HSTRING;

/// Öffnet den Ordner der Datei im Explorer und markiert sie.
pub fn reveal(path: &Path) -> windows::core::Result<()> {
    unsafe {
        // Darf schon initialisiert sein (S_FALSE oder anderer Modus); beides ist hier gleich.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(&HSTRING::from(path.as_os_str()), None, &mut pidl, 0, None)?;
        let result = SHOpenFolderAndSelectItems(pidl, None, 0);
        CoTaskMemFree(Some(pidl as *const _));
        result
    }
}

/// Öffnet einen Ordner im Explorer (Rückfall, wenn die Datei unbekannt ist).
pub fn open_folder(dir: &Path) -> std::io::Result<()> {
    std::process::Command::new("explorer.exe")
        .arg(dir)
        .spawn()
        .map(drop)
}
