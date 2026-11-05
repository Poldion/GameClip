// SPDX-License-Identifier: GPL-3.0-or-later
//! Paketfamilie und Programmordner. Ohne Paket (Entwicklung) gibt es keine Paketfamilie; dann
//! liegt `gameclipd.exe` neben der eigenen Exe.

use std::path::PathBuf;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;

/// `<Name>_<PublisherHash>` oder `None` außerhalb eines Pakets.
pub fn family_name() -> Option<String> {
    let mut len = 0u32;
    unsafe {
        let _ = GetCurrentPackageFamilyName(&mut len, None);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize];
        if GetCurrentPackageFamilyName(&mut len, Some(windows_core::PWSTR(buf.as_mut_ptr())))
            != ERROR_SUCCESS
        {
            return None;
        }
        Some(String::from_utf16_lossy(
            &buf[..len.saturating_sub(1) as usize],
        ))
    }
}

/// Erwarteter Pfad des Dienstes: im Paket und in der Entwicklung neben der eigenen Exe.
pub fn service_exe() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|d| d.join("gameclipd.exe"))
}
