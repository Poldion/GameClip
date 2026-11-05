// SPDX-License-Identifier: GPL-3.0-or-later
//! „Teilen“: öffnet den Teilen-Dialog von Windows mit der Clip-Datei (Discord, Mail, Nearby
//! Share, Phone Link …).
//!
//! Desktop-Apps erreichen ihn über `IDataTransferManagerInterop` mit dem Fensterhandle. Windows
//! fragt die Daten per `DataRequested` auf dem Thread des Fensters ab; [`show`] muss deshalb dort
//! laufen. [`open_file`] blockiert und gehört auf einen Hintergrund-Thread.

use std::cell::RefCell;
use std::path::Path;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
use windows::Foundation::TypedEventHandler;
use windows::Storage::{IStorageItem, StorageFile};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::IDataTransferManagerInterop;
use windows_collections::IIterable;
use windows_core::{AgileReference, HSTRING, Interface};

/// Geöffnete Datei, über Threads hinweg verschickbar.
pub type SharedFile = AgileReference<StorageFile>;

/// Öffnet die Datei als `StorageFile` (blockiert, nicht auf dem Fenster-Thread aufrufen).
pub fn open_file(path: &Path) -> windows::core::Result<SharedFile> {
    let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.join()?;
    AgileReference::new(&file)
}

struct Pending {
    title: String,
    file: StorageFile,
}

thread_local! {
    /// Fenster, für die `DataRequested` schon angemeldet ist (Handle als `isize`).
    static REGISTERED: RefCell<Vec<(isize, DataTransferManager, i64)>> = const { RefCell::new(Vec::new()) };
    /// Was beim nächsten `DataRequested` geteilt wird.
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

fn on_requested(args: &DataRequestedEventArgs) -> windows::core::Result<()> {
    let Some(p) = PENDING.with(|p| p.borrow_mut().take()) else {
        return Ok(());
    };
    let data = args.Request()?.Data()?;
    data.Properties()?.SetTitle(&HSTRING::from(p.title))?;
    let items: IIterable<IStorageItem> = vec![Some(p.file.cast::<IStorageItem>()?)].into();
    data.SetStorageItems(&items, true)
}

fn hwnd_of<W: HasWindowHandle>(window: &W) -> Option<HWND> {
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut _)),
        _ => None,
    }
}

/// Zeigt den Teilen-Dialog für `file` über `window`. Auf dem Fenster-Thread aufrufen.
pub fn show<W: HasWindowHandle>(
    window: &W,
    file: &SharedFile,
    title: String,
) -> windows::core::Result<()> {
    let hwnd = hwnd_of(window).ok_or_else(|| {
        windows::core::Error::new(windows::Win32::Foundation::E_HANDLE, "kein Fenster")
    })?;
    let file = file.resolve()?;
    let interop = windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()?;
    let key = hwnd.0 as isize;
    let known = REGISTERED.with(|r| r.borrow().iter().any(|(h, _, _)| *h == key));
    if !known {
        let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };
        let token = manager.DataRequested(&TypedEventHandler::new(
            |_, args: windows_core::Ref<DataRequestedEventArgs>| match args.as_ref() {
                Some(a) => on_requested(a),
                None => Ok(()),
            },
        ))?;
        REGISTERED.with(|r| r.borrow_mut().push((key, manager, token)));
    }
    PENDING.with(|p| *p.borrow_mut() = Some(Pending { title, file }));
    unsafe { interop.ShowShareUIForWindow(hwnd) }
}
