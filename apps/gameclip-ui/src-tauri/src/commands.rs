// SPDX-License-Identifier: GPL-3.0-or-later
//! Tauri-Befehle (Design, Datei 60). Statt je Protokollbefehl einen eigenen Tauri-Befehl zu
//! schreiben, gibt es einen allgemeinen `call`, der einen `Command` durchreicht; die Typisierung
//! je Befehl liegt in `ipc.ts`. Dazu die Aufrufe, die nur die Oberfläche hat.

use std::path::PathBuf;

use gameclip_protocol::messages::{ClipEntry, Command, Response};
use gameclip_ui_platform::{clipboard, media, reveal as reveal_mod, update};
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::ipc::{Connection, Ipc};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn call(ipc: State<'_, Ipc>, cmd: Command) -> Result<Response, String> {
    let ipc = ipc.inner().clone();
    blocking(move || ipc.call(cmd)).await?
}

#[tauri::command]
pub fn connection(ipc: State<'_, Ipc>) -> Connection {
    ipc.connection()
}

/// Pfad eines Clips, nur über den Dienst (die Oberfläche kennt keine Pfade aus dem Frontend).
pub fn clip_path(ipc: &Ipc, clip_id: u32) -> Result<PathBuf, String> {
    let r = ipc.call(Command::GetClip { clip_id })?;
    match (r.ok, r.body, r.error) {
        (true, Some(body), _) => serde_json::from_value::<ClipEntry>(body)
            .map(|c| PathBuf::from(c.path))
            .map_err(|e| e.to_string()),
        (_, _, Some(e)) => Err(e.message),
        _ => Err("leere Antwort".into()),
    }
}

#[tauri::command]
pub async fn copy_file(ipc: State<'_, Ipc>, clip_id: u32) -> Result<(), String> {
    let ipc = ipc.inner().clone();
    blocking(move || {
        let path = clip_path(&ipc, clip_id)?;
        clipboard::copy_file(&path).map_err(|e| e.to_string())
    })
    .await?
}

#[tauri::command]
pub async fn reveal(ipc: State<'_, Ipc>, clip_id: u32) -> Result<(), String> {
    let ipc = ipc.inner().clone();
    blocking(move || {
        let path = clip_path(&ipc, clip_id)?;
        reveal_mod::reveal(&path).map_err(|e| e.to_string())
    })
    .await?
}

#[tauri::command]
pub async fn pick_folder(
    app: tauri::AppHandle,
    current: Option<String>,
) -> Result<Option<String>, String> {
    blocking(move || {
        let mut dialog = app.dialog().file().set_title("Ordner wählen");
        if let Some(dir) = current.filter(|d| std::path::Path::new(d).is_dir()) {
            dialog = dialog.set_directory(dir);
        }
        dialog
            .blocking_pick_folder()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.display().to_string())
    })
    .await
}

/// Bild am Mauszeiger, wenn die Shell kein Vorschaubild liefert.
const DRAG_FALLBACK_PNG: &[u8] = include_bytes!("../icons/128x128.png");

/// Datei aus der Galerie ziehen. `DoDragDrop` braucht den Hauptthread mit seiner
/// Nachrichtenschleife; der Befehl kehrt zurück, sobald das Ziehen begonnen hat.
#[tauri::command]
pub async fn start_drag(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    ipc: State<'_, Ipc>,
    clip_id: u32,
) -> Result<(), String> {
    let ipc = ipc.inner().clone();
    let (path, preview) = blocking(move || {
        let path = clip_path(&ipc, clip_id)?;
        let preview = gameclip_ui_platform::thumbnail::png(&path, 96)
            .unwrap_or_else(|_| DRAG_FALLBACK_PNG.to_vec());
        Ok::<_, String>((path, preview))
    })
    .await??;
    app.run_on_main_thread(move || {
        if let Err(e) = gameclip_ui_platform::drag::start(&window, &path, preview) {
            tracing::warn!("Ziehen nicht möglich: {e}");
        }
    })
    .map_err(|e| e.to_string())
}

/// Teilen-Dialog von Windows mit der Clip-Datei (Design, Datei 64: „Teilen“ in der Galerie).
#[tauri::command]
pub async fn share(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    ipc: State<'_, Ipc>,
    clip_id: u32,
) -> Result<(), String> {
    use gameclip_ui_platform::share;
    let ipc = ipc.inner().clone();
    let (file, title) = blocking(move || {
        let path = clip_path(&ipc, clip_id)?;
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Clip".into());
        let file = share::open_file(&path).map_err(|e| e.to_string())?;
        Ok::<_, String>((file, title))
    })
    .await??;
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(share::show(&window, &file, title).map_err(|e| e.to_string()));
    })
    .map_err(|e| e.to_string())?;
    blocking(move || rx.recv().map_err(|e| e.to_string())).await???;
    Ok(())
}

/// Ergebnis von „Nach Updates suchen“ für die Einstellungen.
#[derive(serde::Serialize)]
pub struct UpdateCheck {
    /// Installierte Version; `None` ohne Paket (Entwicklung).
    current: Option<String>,
    latest: Option<String>,
    newer: bool,
    page_url: Option<String>,
    size_bytes: Option<u64>,
}

fn check_update_blocking() -> Result<(UpdateCheck, Option<update::Available>), String> {
    let current = update::current_version();
    let latest =
        update::check().map_err(|e| format!("GitHub nicht erreichbar: {}", e.message()))?;
    let newer = match (&latest, current) {
        (Some(a), Some(c)) => a.version > c,
        _ => false,
    };
    Ok((
        UpdateCheck {
            current: current.map(update::version_string),
            latest: latest.as_ref().map(|a| update::version_string(a.version)),
            newer,
            page_url: latest.as_ref().map(|a| a.page_url.clone()),
            size_bytes: latest.as_ref().map(|a| a.size_bytes),
        },
        latest.filter(|_| newer),
    ))
}

/// Fragt GitHub nach einer neueren Version. Nur auf Klick des Benutzers (kein Hintergrund-Check).
#[tauri::command]
pub async fn check_update() -> Result<UpdateCheck, String> {
    blocking(|| check_update_blocking().map(|(c, _)| c)).await?
}

/// Lädt die neueste Version und öffnet sie im App Installer, der GameClip beendet und
/// aktualisiert. Prüft selbst noch einmal; das Frontend gibt keine Adresse vor.
#[tauri::command]
pub async fn install_update() -> Result<(), String> {
    blocking(|| {
        let (_, available) = check_update_blocking()?;
        let available = available.ok_or("Keine neuere Version vorhanden.")?;
        let path = update::download(&available)
            .map_err(|e| format!("Download fehlgeschlagen: {}", e.message()))?;
        tracing::info!(
            "Update {} geladen nach {}",
            update::version_string(available.version),
            path.display()
        );
        update::open_installer(&path).map_err(|e| e.message().to_string())
    })
    .await?
}

#[tauri::command]
pub fn has_media_foundation() -> bool {
    media::has_media_foundation()
}
