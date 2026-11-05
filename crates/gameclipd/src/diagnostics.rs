// SPDX-License-Identifier: GPL-3.0-or-later
//! Diagnose zum Kopieren (Design, Datei 54): Windows-Build, GPUs mit Treiberversion, Encoder,
//! Audiogeräte, Einstellungen, Puffer-Zähler und die letzten 200 Logzeilen. `%USERPROFILE%` wird
//! durch `<USER>` ersetzt, damit kein Benutzername im Text steht.

use std::fmt::Write as _;
use std::path::Path;

use gameclip_core::settings::Settings;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIDevice, IDXGIFactory1};
use windows_core::Interface;

pub struct DiagInput<'a> {
    pub settings: &'a Settings,
    pub state: String,
    pub encoder: Option<String>,
    pub ring: Option<gameclip_capture_win::ring::RingStats>,
    pub buffered_us: u64,
    pub log_dir: &'a Path,
}

fn windows_build() -> String {
    let read = |name: &str| {
        std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                "/v",
                name,
            ])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .find(|l| l.contains(name))
                    .and_then(|l| l.split_whitespace().last().map(str::to_string))
            })
            .unwrap_or_default()
    };
    format!(
        "{} (Build {}.{})",
        read("DisplayVersion"),
        read("CurrentBuild"),
        read("UBR").trim_start_matches("0x")
    )
}

fn gpus() -> Vec<String> {
    let mut out = Vec::new();
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return out;
    };
    let mut i = 0;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        if let Ok(d) = unsafe { adapter.GetDesc1() } {
            let end = d
                .Description
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(d.Description.len());
            let name = String::from_utf16_lossy(&d.Description[..end]);
            // Treiberversion (UMD) als vier 16-Bit-Teile.
            let version = unsafe { adapter.CheckInterfaceSupport(&IDXGIDevice::IID) }
                .map(|v| {
                    let v = v as u64;
                    format!(
                        "{}.{}.{}.{}",
                        v >> 48,
                        (v >> 32) & 0xFFFF,
                        (v >> 16) & 0xFFFF,
                        v & 0xFFFF
                    )
                })
                .unwrap_or_else(|_| "?".into());
            out.push(format!(
                "{name} (Treiber {version}, Hersteller 0x{:04X})",
                d.VendorId
            ));
        }
        i += 1;
    }
    out
}

fn last_log_lines(dir: &Path, n: usize) -> Vec<String> {
    let newest = std::fs::read_dir(dir)
        .ok()
        .and_then(|e| {
            e.flatten()
                .filter(|f| f.file_name().to_string_lossy().starts_with("gameclipd"))
                .max_by_key(|f| f.metadata().and_then(|m| m.modified()).ok())
        })
        .map(|f| f.path());
    let Some(path) = newest else {
        return Vec::new();
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Ersetzt den Benutzerordner durch `<USER>`.
pub fn anonymize(text: &str) -> String {
    match std::env::var("USERPROFILE") {
        Ok(home) if !home.is_empty() => text.replace(&home, "<USER>"),
        _ => text.to_string(),
    }
}

pub fn report(d: &DiagInput<'_>) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "GameClip {} – Diagnose", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(s, "Windows: {}", windows_build());
    for g in gpus() {
        let _ = writeln!(s, "GPU: {g}");
    }
    let _ = writeln!(s, "Zustand: {}", d.state);
    let _ = writeln!(s, "Encoder: {}", d.encoder.as_deref().unwrap_or("–"));
    let _ = writeln!(s, "Gepuffert: {:.1} s", d.buffered_us as f64 / 1e6);
    if let Some(r) = d.ring {
        let _ = writeln!(
            s,
            "Ringpuffer: {} Pakete, {}/{} Blöcke frei, {} übergroß, {} verworfen, {} Lücken",
            r.packets, r.free_blocks, r.pool_blocks, r.oversize_packets, r.dropped_packets, r.gaps
        );
    }
    match gameclip_capture_win::audio::mic::list_devices() {
        Ok(mics) => {
            for m in mics {
                let _ = writeln!(s, "Mikrofon: {}", m.name);
            }
        }
        Err(e) => {
            let _ = writeln!(s, "Mikrofone: {e}");
        }
    }
    let c = &d.settings.capture;
    let _ = writeln!(
        s,
        "Einstellungen: {} s, {:?} {} fps {:?}, Mikrofon {}, Hotkey {:?}",
        c.buffer_seconds,
        c.resolution,
        c.fps,
        c.quality,
        d.settings.audio.mic_enabled,
        d.settings.hotkey
    );
    let _ = writeln!(s, "\nLetzte Logzeilen:");
    for line in last_log_lines(d.log_dir, 200) {
        let _ = writeln!(s, "{line}");
    }
    anonymize(&s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_folder_is_replaced() {
        let home = std::env::var("USERPROFILE").unwrap();
        let text = format!("Clip in {home}\\Videos\\GameClip");
        assert_eq!(super::anonymize(&text), "Clip in <USER>\\Videos\\GameClip");
    }
}
