// SPDX-License-Identifier: GPL-3.0-or-later
//! Tray-Symbol und Menü (Design, Datei 51). M0-Umfang: Zustandsfarbe, Tooltip, Menü mit
//! „Clip speichern“, „Letzten Clip kopieren“ (G1a, CF_HDROP), „Aufnahme pausieren/fortsetzen“,
//! „Mikrofon pausieren“, „Beenden“. Die sechs gestalteten Symbole folgen mit Datei 71; bis dahin
//! ein farbiger Kreis je Zustand.

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    SaveClip,
    CopyLastClip,
    /// Letzten Clip auf die Zielgröße exportieren und kopieren (G1a).
    CopyLastClipSmall,
    TogglePause,
    ToggleMic,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Starting,
    Buffering,
    /// Läuft mit Einschränkung (CPU-Encoder, Hotkey inaktiv, HDR).
    Restricted,
    Saving,
    Paused,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayView {
    pub state: TrayState,
    pub tooltip: String,
    pub paused: bool,
    pub mic_paused: bool,
    /// Zielgröße in MB für den Menüeintrag Letzten Clip unter N MB kopieren.
    pub target_mb: u64,
}

pub struct Tray {
    icon: TrayIcon,
    save: MenuItem,
    copy: MenuItem,
    copy_small: MenuItem,
    pause: MenuItem,
    mic: CheckMenuItem,
    quit: MenuItem,
    last: TrayView,
}

fn color(state: TrayState) -> [u8; 3] {
    match state {
        TrayState::Starting => [120, 120, 120],
        TrayState::Buffering => [46, 160, 67],
        TrayState::Restricted => [210, 153, 34],
        TrayState::Saving => [56, 139, 253],
        TrayState::Paused => [150, 150, 150],
        TrayState::Failed => [218, 54, 51],
    }
}

/// Gefüllter Kreis 32 × 32 mit weichem Rand.
fn icon(state: TrayState) -> Icon {
    const N: u32 = 32;
    let [r, g, b] = color(state);
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    let c = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            let a = (c - d + 0.5).clamp(0.0, 1.0);
            rgba.extend_from_slice(&[r, g, b, (a * 255.0) as u8]);
        }
    }
    Icon::from_rgba(rgba, N, N).expect("gültiges Symbol")
}

impl Tray {
    pub fn new(view: TrayView) -> Result<Self, String> {
        let save = MenuItem::new("Clip speichern", true, None);
        let copy = MenuItem::new("Letzten Clip kopieren", true, None);
        let copy_small = MenuItem::new(
            format!("Letzten Clip unter {} MB kopieren", view.target_mb),
            true,
            None,
        );
        let pause = MenuItem::new("Aufnahme pausieren", true, None);
        let mic = CheckMenuItem::new("Mikrofon pausieren", true, view.mic_paused, None);
        let quit = MenuItem::new("Beenden", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &save,
            &copy,
            &copy_small,
            &PredefinedMenuItem::separator(),
            &pause,
            &mic,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(&view.tooltip)
            .with_icon(icon(view.state))
            .build()
            .map_err(|e| e.to_string())?;
        let mut tray = Tray {
            icon,
            save,
            copy,
            copy_small,
            pause,
            mic,
            quit,
            last: view.clone(),
        };
        tray.update(&view);
        Ok(tray)
    }

    pub fn update(&mut self, view: &TrayView) {
        if view.state != self.last.state {
            let _ = self.icon.set_icon(Some(icon(view.state)));
        }
        if view.tooltip != self.last.tooltip {
            let _ = self.icon.set_tooltip(Some(&view.tooltip));
        }
        self.pause.set_text(if view.paused {
            "Aufnahme fortsetzen"
        } else {
            "Aufnahme pausieren"
        });
        self.mic.set_checked(view.mic_paused);
        self.last = view.clone();
    }

    /// Menüklicks seit dem letzten Aufruf.
    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut out = Vec::new();
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let id = ev.id();
            let cmd = if id == self.save.id() {
                TrayCommand::SaveClip
            } else if id == self.copy.id() {
                TrayCommand::CopyLastClip
            } else if id == self.copy_small.id() {
                TrayCommand::CopyLastClipSmall
            } else if id == self.pause.id() {
                TrayCommand::TogglePause
            } else if id == self.mic.id() {
                TrayCommand::ToggleMic
            } else if id == self.quit.id() {
                TrayCommand::Quit
            } else {
                continue;
            };
            out.push(cmd);
        }
        out
    }
}
