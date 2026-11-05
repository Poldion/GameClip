// SPDX-License-Identifier: GPL-3.0-or-later
//! Vorschaubild eines Clips von der Windows-Shell (`IShellItemImageFactory`), als PNG über WIC.
//! Die Shell nutzt ihren eigenen Bild-Cache; ein Video wird dafür nicht dekodiert.

use std::path::{Path, PathBuf};

use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{DeleteObject, HGDIOBJ};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA,
    IWICImagingFactory, WICBitmapEncoderNoCache, WICBitmapUseAlpha,
};
use windows::Win32::System::Com::StructuredStorage::GetHGlobalFromStream;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_RESIZETOFIT,
};
use windows_core::HSTRING;

/// PNG-Bytes eines Vorschaubilds mit höchstens `size` Pixeln Kantenlänge. Läuft auf einem eigenen
/// Thread, weil die Shell ein STA verlangt und der Aufrufer (z. B. ein Tokio-Thread) keines hat.
pub fn png(path: &Path, size: u32) -> windows::core::Result<Vec<u8>> {
    let path: PathBuf = path.to_path_buf();
    std::thread::spawn(move || unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let result = png_sta(&path, size);
        if init.is_ok() {
            CoUninitialize();
        }
        result
    })
    .join()
    .unwrap_or_else(|_| {
        Err(windows::core::Error::from_hresult(
            windows::Win32::Foundation::E_FAIL,
        ))
    })
}

unsafe fn png_sta(path: &Path, size: u32) -> windows::core::Result<Vec<u8>> {
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None)?;
        let side = size as i32;
        let hbm = factory.GetImage(
            SIZE { cx: side, cy: side },
            SIIGBF_RESIZETOFIT | SIIGBF_BIGGERSIZEOK,
        )?;
        let result = encode(hbm);
        let _ = DeleteObject(HGDIOBJ(hbm.0));
        result
    }
}

unsafe fn encode(hbm: windows::Win32::Graphics::Gdi::HBITMAP) -> windows::core::Result<Vec<u8>> {
    unsafe {
        let wic: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let bitmap = wic.CreateBitmapFromHBITMAP(hbm, Default::default(), WICBitmapUseAlpha)?;
        let (mut w, mut h) = (0u32, 0u32);
        bitmap.GetSize(&mut w, &mut h)?;

        let stream = windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal(
            Default::default(),
            true,
        )?;
        let encoder = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        encoder.CreateNewFrame(&mut frame, std::ptr::null_mut())?;
        let frame = frame.ok_or_else(|| {
            windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL)
        })?;
        frame.Initialize(None)?;
        frame.SetSize(w, h)?;
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format)?;
        frame.WriteSource(&bitmap, std::ptr::null())?;
        frame.Commit()?;
        encoder.Commit()?;

        let mem = GetHGlobalFromStream(&stream)?;
        let len = GlobalSize(mem);
        let ptr = GlobalLock(mem) as *const u8;
        if ptr.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        let bytes = std::slice::from_raw_parts(ptr, len).to_vec();
        let _ = GlobalUnlock(mem);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn png_of_a_shell_item() {
        // Jede Datei hat in der Shell mindestens ein Symbol; dafür reicht eine leere Textdatei.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("probe.txt");
        std::fs::write(&file, b"x").unwrap();
        let png = super::png(&file, 64).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }
}
