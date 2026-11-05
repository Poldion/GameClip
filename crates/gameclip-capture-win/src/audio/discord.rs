// SPDX-License-Identifier: GPL-3.0-or-later
//! Discord finden (Design, Datei 36 und „Audio-Pipeline“): Prozessliste per
//! `CreateToolhelp32Snapshot`, Wurzel ist ein `Discord.exe`, `DiscordPTB.exe` oder
//! `DiscordCanary.exe`, dessen Elternprozess nicht denselben Namen hat. Es wird kein Handle auf
//! Discord geöffnet.

use std::time::Duration;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

/// Abfrageabstand (`DISCORD_POLL_S`).
pub const POLL: Duration = Duration::from_secs(2);

const NAMES: [&str; 3] = ["discord.exe", "discordptb.exe", "discordcanary.exe"];

/// Ein Eintrag der Prozessliste.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub parent: u32,
    pub name: String,
}

/// Wurzelprozess von Discord in einer Prozessliste. Laufen mehrere Varianten (z. B. Discord und
/// PTB), gewinnt die mit der kleinsten PID, damit die Wahl zwischen zwei Abfragen stabil bleibt.
pub fn root_of(procs: &[Proc]) -> Option<u32> {
    procs
        .iter()
        .filter(|p| NAMES.contains(&p.name.to_ascii_lowercase().as_str()))
        .filter(|p| {
            !procs
                .iter()
                .any(|q| q.pid == p.parent && q.name.eq_ignore_ascii_case(&p.name))
        })
        .map(|p| p.pid)
        .min()
}

/// Aktuelle Prozessliste.
pub fn snapshot() -> Vec<Proc> {
    let mut out = Vec::new();
    let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return out;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut ok = unsafe { Process32FirstW(snap, &mut entry) }.is_ok();
    while ok {
        let end = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        out.push(Proc {
            pid: entry.th32ProcessID,
            parent: entry.th32ParentProcessID,
            name: String::from_utf16_lossy(&entry.szExeFile[..end]),
        });
        ok = unsafe { Process32NextW(snap, &mut entry) }.is_ok();
    }
    unsafe {
        let _ = CloseHandle(snap);
    }
    out
}

/// Wurzelprozess des laufenden Discord oder `None`.
pub fn find_root() -> Option<u32> {
    root_of(&snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, parent: u32, name: &str) -> Proc {
        Proc {
            pid,
            parent,
            name: name.into(),
        }
    }

    #[test]
    fn root_is_the_discord_whose_parent_is_not_discord() {
        let procs = [
            p(4, 0, "System"),
            p(900, 4, "explorer.exe"),
            p(1000, 900, "Discord.exe"),
            p(1010, 1000, "Discord.exe"),
            p(1020, 1000, "Discord.exe"),
            p(1030, 1010, "Discord.exe"),
        ];
        assert_eq!(root_of(&procs), Some(1000));
    }

    #[test]
    fn variants_and_absence() {
        assert_eq!(root_of(&[p(1, 0, "explorer.exe")]), None);
        assert_eq!(
            root_of(&[
                p(50, 1, "DiscordCanary.exe"),
                p(51, 50, "DiscordCanary.exe")
            ]),
            Some(50)
        );
        // PTB, dessen Eltern ein normales Discord sind, ist eine eigene Wurzel; die kleinere PID gewinnt.
        assert_eq!(
            root_of(&[p(70, 1, "Discord.exe"), p(60, 70, "DiscordPTB.exe")]),
            Some(60)
        );
    }

    #[test]
    fn snapshot_contains_this_process() {
        let me = std::process::id();
        assert!(snapshot().iter().any(|p| p.pid == me));
    }
}
