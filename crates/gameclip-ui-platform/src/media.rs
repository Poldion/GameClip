// SPDX-License-Identifier: GPL-3.0-or-later
//! Einzige Prüfung auf das Media Feature Pack (Windows 11 N/KN). Media Foundation wird nie zur
//! Ladezeit gelinkt, damit die Oberfläche auch ohne startet und den Hinweis zeigen kann.

use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW};
use windows_core::w;

/// `true`, wenn `mfplat.dll` im Systemordner liegt und lädt.
pub fn has_media_foundation() -> bool {
    unsafe {
        match LoadLibraryExW(w!("mfplat.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) {
            Ok(h) => {
                let _ = FreeLibrary(h);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn media_foundation_is_present_on_the_dev_machine() {
        // Entwicklungs- und CI-Rechner sind keine N-Editionen.
        assert!(super::has_media_foundation());
    }
}
