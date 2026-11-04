// SPDX-License-Identifier: GPL-3.0-or-later
//! Wahl des Adapters, an dem der Monitor hängt, und Erzeugen des gemeinsamen D3D11-Geräts.

use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::{HMODULE, LUID, POINT};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Multithread, ID3D11VideoContext1,
    ID3D11VideoDevice,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIAdapter1, IDXGIDevice, IDXGIFactory1, IDXGIOutput,
};
use windows::Win32::Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows_core::Interface;

use crate::{GpuError, Result};

/// Welcher Monitor aufgenommen wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorTarget {
    Primary,
    Handle(HMONITOR),
}

/// Gemeinsames Gerät für Aufnahme, Umwandlung und Encoder.
pub struct GpuDevice {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    pub video_device: ID3D11VideoDevice,
    pub video_context: ID3D11VideoContext1,
    pub adapter: IDXGIAdapter1,
    pub output: IDXGIOutput,
    pub monitor: HMONITOR,
    /// Kennung des Adapters (LUID als u64), z. B. für den Worker.
    pub adapter_luid: u64,
}

impl GpuDevice {
    /// Erzeugt das Gerät auf dem Adapter, an dem `target` hängt.
    pub fn for_monitor(target: MonitorTarget) -> Result<Self> {
        let monitor = match target {
            MonitorTarget::Primary => unsafe {
                MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY)
            },
            MonitorTarget::Handle(h) => h,
        };
        let (adapter, output) = find_adapter(monitor)?;
        let (device, context) = create_device(&adapter)?;

        // Mehrere Threads (Aufnahme, Taktgeber, Encoder) nutzen dasselbe Gerät.
        let multithread: ID3D11Multithread = context.cast()?;
        unsafe {
            let _ = multithread.SetMultithreadProtected(true);
        }

        let video_device: ID3D11VideoDevice = device.cast()?;
        let video_context: ID3D11VideoContext1 = context.cast()?;
        let desc = unsafe { adapter.GetDesc1()? };
        Ok(GpuDevice {
            device,
            context,
            video_device,
            video_context,
            adapter,
            output,
            monitor,
            adapter_luid: luid_to_u64(desc.AdapterLuid),
        })
    }

    /// Name des Adapters, z. B. für die Statuszeile und die Diagnose.
    pub fn adapter_name(&self) -> String {
        unsafe { self.adapter.GetDesc1() }
            .map(|d| {
                let end = d
                    .Description
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(d.Description.len());
                String::from_utf16_lossy(&d.Description[..end])
            })
            .unwrap_or_default()
    }

    /// PCI-Herstellerkennung des Adapters (0x10DE NVIDIA, 0x1002 AMD, 0x8086 Intel).
    pub fn vendor_id(&self) -> u32 {
        unsafe { self.adapter.GetDesc1() }
            .map(|d| d.VendorId)
            .unwrap_or(0)
    }

    /// WinRT-Gerät für Windows.Graphics.Capture.
    pub fn winrt_device(&self) -> Result<IDirect3DDevice> {
        let dxgi: IDXGIDevice = self.device.cast()?;
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
        Ok(inspectable.cast()?)
    }

    /// `Err`, wenn das Gerät verloren ist (Treiber-Neustart, Standby).
    pub fn check_removed(&self) -> Result<()> {
        unsafe { self.device.GetDeviceRemovedReason()? };
        Ok(())
    }
}

fn luid_to_u64(l: LUID) -> u64 {
    (u64::from(l.HighPart as u32) << 32) | u64::from(l.LowPart)
}

fn find_adapter(monitor: HMONITOR) -> Result<(IDXGIAdapter1, IDXGIOutput)> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    let mut a = 0;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(a) } {
        let mut o = 0;
        while let Ok(output) = unsafe { adapter.EnumOutputs(o) } {
            let desc = unsafe { output.GetDesc()? };
            if desc.Monitor == monitor {
                return Ok((adapter, output));
            }
            o += 1;
        }
        a += 1;
    }
    Err(GpuError::MonitorNotFound)
}

fn create_device(adapter: &IDXGIAdapter1) -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
    }
    Ok((
        device.expect("Gerät erzeugt"),
        context.expect("Kontext erzeugt"),
    ))
}
