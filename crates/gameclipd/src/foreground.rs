// SPDX-License-Identifier: GPL-3.0-or-later
//! Name des Programms im Vordergrund für den Dateinamen (Design, Datei 44).
//!
//! `GetForegroundWindow` → `GetWindowThreadProcessId` → Eintrag mit dieser PID in einer
//! Prozessliste (`CreateToolhelp32Snapshot`). Es wird **kein Handle** auf den Prozess geöffnet
//! (Vanguard-Verträglichkeit, D-Entscheidung „kein Handle auf den Spielprozess“).

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Programmname ohne `.exe`, z. B. `VALORANT-Win64-Shipping`; `None` ohne Vordergrundfenster.
pub fn foreground_program() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    process_name(pid)
}

/// Name des Prozesses `pid` aus der Prozessliste.
pub fn process_name(pid: u32) -> Option<String> {
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()? };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut found = None;
    let mut ok = unsafe { Process32FirstW(snap, &mut entry) }.is_ok();
    while ok {
        if entry.th32ProcessID == pid {
            let end = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
            found = Some(match name.rsplit_once('.') {
                Some((stem, ext)) if ext.eq_ignore_ascii_case("exe") => stem.to_string(),
                _ => name,
            });
            break;
        }
        ok = unsafe { Process32NextW(snap, &mut entry) }.is_ok();
    }
    unsafe {
        let _ = CloseHandle(snap);
    }
    found
}

#[cfg(test)]
mod tests {
    #[test]
    fn own_process_is_found_without_extension() {
        let name = super::process_name(std::process::id()).unwrap();
        assert!(!name.to_lowercase().ends_with(".exe"));
        assert!(name.starts_with("gameclipd"), "{name}");
    }
}
