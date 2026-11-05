// SPDX-License-Identifier: GPL-3.0-or-later
//! Datei in die Zwischenablage legen (`CF_HDROP`), zum Einfügen in Discord oder den Explorer.
//! Dieselbe Logik hat der Dienst für die Tray-Aktion (`gameclipd::clipboard`); beide sind klein
//! genug, dass sich ein gemeinsames Crate nicht lohnt.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_HDROP;
use windows::Win32::UI::Shell::DROPFILES;

/// Legt `path` als Dateiliste mit einem Eintrag in die Zwischenablage.
pub fn copy_file(path: &Path) -> windows::core::Result<()> {
    // DROPFILES, danach der Pfad als UTF-16 mit doppeltem Nullzeichen.
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.extend([0, 0]);
    let header = std::mem::size_of::<DROPFILES>();
    let bytes = header + wide.len() * 2;
    unsafe {
        let mem: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes)?;
        let ptr = GlobalLock(mem) as *mut u8;
        let drop = DROPFILES {
            pFiles: header as u32,
            fWide: true.into(),
            ..Default::default()
        };
        std::ptr::write_unaligned(ptr as *mut DROPFILES, drop);
        std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, ptr.add(header), wide.len() * 2);
        let _ = GlobalUnlock(mem);

        OpenClipboard(None)?;
        let result = EmptyClipboard()
            .and_then(|_| SetClipboardData(u32::from(CF_HDROP.0), Some(HANDLE(mem.0))).map(|_| ()));
        let _ = CloseClipboard();
        result
    }
}
