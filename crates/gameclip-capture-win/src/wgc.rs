// SPDX-License-Identifier: GPL-3.0-or-later
//! Monitoraufnahme mit Windows.Graphics.Capture (Design, Datei 32).
//!
//! WGC verwendet seine Texturen wieder; `FrameArrived` kopiert deshalb jedes Bild auf der GPU
//! in einen Ring aus 3 eigenen Texturen. Der Taktgeber holt sich mit `acquire` das neueste
//! Bild; solange er es hält, überschreibt der Handler es nicht.

use std::sync::{Arc, Mutex};

use gameclip_gpu::GpuDevice;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind,
    GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Security::Authorization::AppCapabilityAccess::AppCapabilityAccessStatus;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::IInspectable;
use windows_core::Interface;

use crate::Result;

const SLOTS: usize = 3;
const POOL_BUFFERS: i32 = 2;

/// Zähler der Aufnahme.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CaptureCounters {
    /// Von WGC gelieferte Bilder.
    pub arrived: u64,
    /// Bilder, die überschrieben wurden, bevor der Taktgeber sie geholt hat.
    pub overwritten: u64,
}

struct Slots {
    textures: Vec<ID3D11Texture2D>,
    /// Größe der eigenen Texturen (= Inhaltsgröße der Quelle).
    size: (u32, u32),
    newest: Option<usize>,
    in_use: Option<usize>,
    /// Zeit des neuesten Bildes (QPC, µs).
    time_us: i64,
    /// Zählt neue Bilder; der Taktgeber erkennt daran Wiederholungen.
    seq: u64,
    /// `newest` wurde seit dem letzten `acquire` nicht abgeholt.
    unread: bool,
    counters: CaptureCounters,
}

/// Geräte-Referenzen für den Handler-Thread von WGC.
struct DeviceRefs {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    /// WinRT-Sicht desselben Geräts, für `Recreate` des Frame-Pools.
    winrt: IDirect3DDevice,
}

// SAFETY: Das Gerät ist mit `SetMultithreadProtected(true)` angelegt (gameclip-gpu); D3D11-Geräte
// sind frei verwendbar, der Immediate Context ist durch den Multithread-Schutz serialisiert.
unsafe impl Send for DeviceRefs {}
unsafe impl Sync for DeviceRefs {}
// SAFETY: Texturen gehören zu demselben geschützten Gerät; Zugriff nur unter dem Mutex.
unsafe impl Send for Slots {}

struct Shared {
    slots: Mutex<Slots>,
    dev: DeviceRefs,
}

/// Das neueste Bild, gehalten bis zum Drop.
pub struct Acquired {
    shared: Arc<Shared>,
    pub texture: ID3D11Texture2D,
    pub size: (u32, u32),
    pub time_us: i64,
    pub seq: u64,
}

impl Drop for Acquired {
    fn drop(&mut self) {
        if let Ok(mut s) = self.shared.slots.lock() {
            s.in_use = None;
        }
    }
}

pub struct WgcCapture {
    session: GraphicsCaptureSession,
    pool: Direct3D11CaptureFramePool,
    token: i64,
    shared: Arc<Shared>,
    /// `true`, wenn der gelbe Rahmen abgeschaltet werden konnte.
    pub border_removed: bool,
}

impl WgcCapture {
    /// Startet die Aufnahme des Monitors von `gpu`. `fps` begrenzt die Lieferrate.
    pub fn start(gpu: &GpuDevice, fps: u32, cursor: bool) -> Result<Self> {
        let item = item_for_monitor(gpu.monitor)?;
        let size = item.Size()?;

        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &gpu.winrt_device()?,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            POOL_BUFFERS,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        session.SetIsCursorCaptureEnabled(cursor)?;
        // Ab Build 26100 vorhanden; begrenzt die Lieferrate schon vor dem Taktgeber.
        let _ = session.SetMinUpdateInterval(windows::Foundation::TimeSpan {
            Duration: 10_000_000 / i64::from(fps),
        });
        let border_removed = remove_border(&session);

        let dims = (size.Width as u32, size.Height as u32);
        let shared = Arc::new(Shared {
            slots: Mutex::new(Slots {
                textures: make_textures(&gpu.device, dims)?,
                size: dims,
                newest: None,
                in_use: None,
                time_us: 0,
                seq: 0,
                unread: false,
                counters: CaptureCounters::default(),
            }),
            dev: DeviceRefs {
                device: gpu.device.clone(),
                context: gpu.context.clone(),
                winrt: gpu.winrt_device()?,
            },
        });

        let handler_shared = shared.clone();
        let handler =
            TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |sender, _| {
                if let Some(pool) = sender.as_ref() {
                    on_frame(pool, &handler_shared);
                }
                Ok(())
            });
        let token = pool.FrameArrived(&handler)?;
        session.StartCapture()?;
        Ok(WgcCapture {
            session,
            pool,
            token,
            shared,
            border_removed,
        })
    }

    /// Holt das neueste Bild (oder `None`, solange noch keins da ist).
    pub fn acquire(&self) -> Option<Acquired> {
        let mut s = self.shared.slots.lock().ok()?;
        let index = s.newest?;
        s.in_use = Some(index);
        s.unread = false;
        Some(Acquired {
            shared: self.shared.clone(),
            texture: s.textures[index].clone(),
            size: s.size,
            time_us: s.time_us,
            seq: s.seq,
        })
    }

    pub fn counters(&self) -> CaptureCounters {
        self.shared
            .slots
            .lock()
            .map(|s| s.counters)
            .unwrap_or_default()
    }
}

impl Drop for WgcCapture {
    fn drop(&mut self) {
        let _ = self.pool.RemoveFrameArrived(self.token);
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

fn item_for_monitor(monitor: HMONITOR) -> Result<GraphicsCaptureItem> {
    let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    Ok(unsafe { interop.CreateForMonitor(monitor)? })
}

/// Fragt die randlose Aufnahme an und schaltet den Rahmen ab. Ohne MSIX-Fähigkeit
/// `graphicsCaptureWithoutBorder` kann das scheitern (Q15); die Aufnahme läuft dann mit Rahmen.
fn remove_border(session: &GraphicsCaptureSession) -> bool {
    let granted = GraphicsCaptureAccess::RequestAccessAsync(GraphicsCaptureAccessKind::Borderless)
        .and_then(|op| op.join())
        .map(|s| s == AppCapabilityAccessStatus::Allowed)
        .unwrap_or(false);
    granted && session.SetIsBorderRequired(false).is_ok()
}

fn make_textures(device: &ID3D11Device, (w, h): (u32, u32)) -> Result<Vec<ID3D11Texture2D>> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: w,
        Height: h,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        // Der AMD-Treiber lehnt eine VideoProcessor-Eingabe ohne Render-Target-Bindung ab
        // (`CreateVideoProcessorInputView` → E_INVALIDARG, gemessen 2026-11).
        BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
        ..Default::default()
    };
    (0..SLOTS)
        .map(|_| {
            let mut tex = None;
            unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex))? };
            Ok(tex.expect("Textur erzeugt"))
        })
        .collect()
}

/// Handler für `FrameArrived` (läuft auf einem Thread von WGC).
fn on_frame(pool: &Direct3D11CaptureFramePool, shared: &Shared) {
    let Ok(frame) = pool.TryGetNextFrame() else {
        return;
    };
    let (Ok(content), Ok(time), Ok(surface)) = (
        frame.ContentSize(),
        frame.SystemRelativeTime(),
        frame.Surface(),
    ) else {
        return;
    };
    let Ok(source) = surface
        .cast::<IDirect3DDxgiInterfaceAccess>()
        .and_then(|a| unsafe { a.GetInterface::<ID3D11Texture2D>() })
    else {
        return;
    };
    let dims = (content.Width.max(1) as u32, content.Height.max(1) as u32);

    let Ok(mut s) = shared.slots.lock() else {
        return;
    };
    s.counters.arrived += 1;
    if dims != s.size {
        // Quelle hat die Größe geändert: eigene Texturen neu anlegen, sobald keine gehalten wird.
        if s.in_use.is_some() {
            return;
        }
        match make_textures(&shared.dev.device, dims) {
            Ok(t) => {
                s.textures = t;
                s.size = dims;
                s.newest = None;
            }
            Err(_) => return,
        }
        let _ = pool.Recreate(
            &shared.dev.winrt,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            POOL_BUFFERS,
            SizeInt32 {
                Width: dims.0 as i32,
                Height: dims.1 as i32,
            },
        );
    }
    let target = (0..SLOTS)
        .find(|&i| Some(i) != s.newest && Some(i) != s.in_use)
        .expect("3 Plätze, höchstens 2 belegt");
    let region = D3D11_BOX {
        left: 0,
        top: 0,
        front: 0,
        right: dims.0,
        bottom: dims.1,
        back: 1,
    };
    unsafe {
        shared.dev.context.CopySubresourceRegion(
            &s.textures[target],
            0,
            0,
            0,
            0,
            &source,
            0,
            Some(&region),
        );
    }
    if s.unread {
        s.counters.overwritten += 1;
    }
    s.newest = Some(target);
    s.unread = true;
    s.seq += 1;
    s.time_us = time.Duration / 10;
}
