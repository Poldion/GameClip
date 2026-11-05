// SPDX-License-Identifier: GPL-3.0-or-later
//! Ordnerüberwachung, solange die Galerie offen ist (Design, Datei 46).
//!
//! `ReadDirectoryChangesW` je Ordner (nicht rekursiv) auf einem eigenen Thread. Jede Meldung, auch
//! `ERROR_NOTIFY_ENUM_DIR` (Überlauf), geht als `FolderKind` an die Hauptschleife; die entprellt
//! (1 s) und gleicht ab. Keine eigene Logik über Dateien.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::Sender;
use gameclip_core::retention::FolderKind;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::CancelSynchronousIo;
use windows_core::HSTRING;

pub struct Watcher {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Watcher {
    /// Überwacht die Ordner (gleiche Ordner nur einmal).
    pub fn start(dirs: &[(PathBuf, FolderKind)], tx: Sender<FolderKind>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        let mut seen: Vec<&Path> = Vec::new();
        for (dir, folder) in dirs {
            if seen.contains(&dir.as_path()) {
                continue;
            }
            seen.push(dir);
            let (dir, folder, tx, stop) = (dir.clone(), *folder, tx.clone(), stop.clone());
            if let Ok(t) = std::thread::Builder::new()
                .name("gameclip-watch".into())
                .spawn(move || watch(&dir, folder, &tx, &stop))
            {
                threads.push(t);
            }
        }
        Watcher { stop, threads }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            // Blockierendes ReadDirectoryChangesW abbrechen.
            use std::os::windows::io::AsRawHandle;
            unsafe {
                let _ = CancelSynchronousIo(HANDLE(t.as_raw_handle()));
            }
            let _ = t.join();
        }
    }
}

fn watch(dir: &Path, folder: FolderKind, tx: &Sender<FolderKind>, stop: &AtomicBool) {
    let Ok(handle) = (unsafe {
        CreateFileW(
            &HSTRING::from(dir.as_os_str()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }) else {
        return;
    };
    let mut buf = vec![0u8; 16 * 1024];
    while !stop.load(Ordering::Relaxed) {
        let mut returned = 0u32;
        let r = unsafe {
            ReadDirectoryChangesW(
                handle,
                buf.as_mut_ptr().cast(),
                buf.len() as u32,
                false,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_LAST_WRITE,
                Some(&mut returned),
                None,
                None,
            )
        };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        // Erfolg, Überlauf (returned == 0 / ERROR_NOTIFY_ENUM_DIR): jedes Mal abgleichen.
        if tx.send(folder).is_err() {
            break;
        }
        if r.is_err() && returned == 0 {
            // Abgebrochen oder Ordner weg: kurz warten statt im Kreis zu laufen.
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
    unsafe {
        let _ = CloseHandle(handle);
    }
}
