// SPDX-License-Identifier: GPL-3.0-or-later
//! GameClip-Oberfläche (Design, Dateien 59/60). Spricht nur über die Named Pipe mit dem Dienst
//! und lädt kein FFmpeg.
//!
//! Wird die App über das Protokoll `gameclip:reveal?id=<clip_id>` gestartet (Klick auf die
//! Benachrichtigung), zeigt sie die Datei im Explorer und beendet sich ohne Fenster.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clip_protocol;
mod commands;
mod ipc;

use std::time::Duration;

use gameclip_core::ids::PROTOCOL_SCHEME;
use gameclip_protocol::messages::{ClipEntry, Command, GetSettingsReply};
use gameclip_ui_platform::pipe_client::{Client, ServerCheck};
use gameclip_ui_platform::reveal;
use tauri::{Emitter, Manager};

/// `gameclip:reveal?id=12` → `Some(12)`.
fn reveal_arg(arg: &str) -> Option<u32> {
    let rest = arg.strip_prefix(PROTOCOL_SCHEME)?.strip_prefix(':')?;
    let query = rest.trim_end_matches('/').strip_prefix("reveal?")?;
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("id="))
        .and_then(|v| v.parse().ok())
}

/// Datei zeigen; geht das nicht, den Clip-Ordner öffnen.
fn reveal_and_exit(clip_id: u32) {
    let timeout = Duration::from_secs(5);
    let client = Client::connect("gameclip-ui-reveal", &ServerCheck::None)
        .ok()
        .map(|(c, _)| c);
    if let Some(c) = &client {
        if let Ok(r) = c.call(Command::GetClip { clip_id }, timeout)
            && let Some(entry) = r
                .body
                .and_then(|b| serde_json::from_value::<ClipEntry>(b).ok())
            && reveal::reveal(std::path::Path::new(&entry.path)).is_ok()
        {
            return;
        }
        if let Ok(r) = c.call(Command::GetSettings, timeout)
            && let Some(s) = r
                .body
                .and_then(|b| serde_json::from_value::<GetSettingsReply>(b).ok())
            && let Some(dir) = s.settings.storage.clip_folder
        {
            let _ = reveal::open_folder(&dir);
            return;
        }
    }
    // Standardordner (siehe gameclipd::paths): Videos\GameClip.
    if let Some(home) = std::env::var_os("USERPROFILE") {
        let _ = reveal::open_folder(&std::path::Path::new(&home).join("Videos").join("GameClip"));
    }
}

/// `--view=settings` → `"settings"`; vom Tray des Dienstes (`gameclipd::ui_launch`).
fn view_arg<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter()
        .find_map(|a| a.as_ref().strip_prefix("--view=").map(str::to_string))
        .filter(|v| v == "gallery" || v == "settings")
}

/// Ansicht, mit der das Fenster öffnet; das Frontend holt sie einmal ab.
#[derive(Default)]
struct InitialView(std::sync::Mutex<Option<String>>);

#[tauri::command]
fn initial_view(state: tauri::State<'_, InitialView>) -> Option<String> {
    state.0.lock().ok().and_then(|mut v| v.take())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(id) = args.iter().find_map(|a| reveal_arg(a)) {
        reveal_and_exit(id);
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();

    let ipc = ipc::Ipc::default();
    tauri::Builder::default()
        // Zweiter Start (Tray, Startmenü, Benachrichtigung): vorhandenes Fenster nach vorn holen.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(id) = argv.iter().find_map(|a| reveal_arg(a)) {
                std::thread::spawn(move || reveal_and_exit(id));
                return;
            }
            if let Some(view) = view_arg(&argv) {
                let _ = app.emit("open-view", view);
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(ipc.clone())
        .manage(InitialView(std::sync::Mutex::new(view_arg(&args))))
        .register_asynchronous_uri_scheme_protocol("clip", clip_protocol::protocol)
        .invoke_handler(tauri::generate_handler![
            commands::call,
            commands::connection,
            commands::copy_file,
            commands::reveal,
            commands::pick_folder,
            commands::has_media_foundation,
            commands::start_drag,
            commands::share,
            initial_view,
        ])
        .setup(move |app| {
            ipc.start(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri startet");
}

#[cfg(test)]
mod tests {
    #[test]
    fn reveal_argument() {
        assert_eq!(super::reveal_arg("gameclip:reveal?id=12"), Some(12));
        assert_eq!(super::reveal_arg("gameclip:reveal?id=7/"), Some(7));
        assert_eq!(super::reveal_arg("gameclip:open?id=7"), None);
        assert_eq!(super::reveal_arg("--flag"), None);
    }

    #[test]
    fn view_argument() {
        assert_eq!(
            super::view_arg(&["--view=settings"]),
            Some("settings".into())
        );
        assert_eq!(
            super::view_arg(&["x", "--view=gallery"]),
            Some("gallery".into())
        );
        assert_eq!(super::view_arg(&["--view=editor"]), None);
        assert_eq!(super::view_arg::<&str>(&[]), None);
    }
}
