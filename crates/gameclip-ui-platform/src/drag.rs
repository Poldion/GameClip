// SPDX-License-Identifier: GPL-3.0-or-later
//! Datei aus der Galerie ziehen (Design, Datei 58): echtes Windows-Drag (`DoDragDrop`) über das
//! Crate `drag`, damit Discord, der Explorer und andere Programme die Datei selbst bekommen.
//! Muss auf dem Thread mit der Nachrichtenschleife des Fensters laufen (Tauri: Hauptthread).

use std::path::Path;

use raw_window_handle::HasWindowHandle;

/// Startet das Ziehen von `path`. `preview_png` ist das Bild am Mauszeiger.
pub fn start<W: HasWindowHandle>(
    window: &W,
    path: &Path,
    preview_png: Vec<u8>,
) -> Result<(), String> {
    let path = dunce_canonical(path)?;
    drag::start_drag(
        window,
        drag::DragItem::Files(vec![path]),
        drag::Image::Raw(preview_png),
        |result, _| tracing::debug!("Ziehen beendet: {result:?}"),
        drag::Options::default(),
    )
    .map_err(|e| e.to_string())
}

/// Absoluter Pfad ohne `\\?\`-Präfix (das manche Ziele nicht verstehen).
fn dunce_canonical(path: &Path) -> Result<std::path::PathBuf, String> {
    let p = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let s = p.to_string_lossy();
    Ok(match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => std::path::PathBuf::from(rest),
        _ => p,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn canonical_path_has_no_verbatim_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.mp4");
        std::fs::write(&f, b"x").unwrap();
        let p = super::dunce_canonical(&f).unwrap();
        assert!(!p.to_string_lossy().starts_with(r"\\?\"), "{}", p.display());
        assert!(p.is_absolute() && p.exists());
    }
}
