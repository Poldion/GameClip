// SPDX-License-Identifier: GPL-3.0-or-later
//! D3D11VA-Geräte- und Frames-Kontext für FFmpeg aus dem gemeinsamen `GpuDevice`.
//!
//! Der Frames-Kontext nutzt einen dynamischen Pool (`initial_pool_size = 0`): jede Anforderung
//! liefert eine eigene NV12-Textur mit `D3D11_BIND_RENDER_TARGET`. NV12-Texture-Arrays mit
//! Render-Target lehnt der AMD-Treiber ab (Design, Datei 27).

use std::ffi::c_void;
use std::ptr;

use gameclip_gpu::GpuDevice;
use rsmpeg::avutil::{AVFrame, AVHWDeviceContext, AVHWFramesContext};
use rsmpeg::ffi;
use windows::Win32::Graphics::Direct3D11::{D3D11_BIND_RENDER_TARGET, ID3D11Texture2D};
use windows_core::Interface;

use crate::{MediaError, Result};

/// Spiegel von `AVD3D11VADeviceContext` aus `libavutil/hwcontext_d3d11va.h` (FFmpeg 8.1).
/// `rusty_ffmpeg` bindet diesen Header nicht ein.
#[repr(C)]
struct D3D11VADeviceContext {
    device: *mut c_void,
    device_context: *mut c_void,
    video_device: *mut c_void,
    video_context: *mut c_void,
    lock: Option<unsafe extern "C" fn(*mut c_void)>,
    unlock: Option<unsafe extern "C" fn(*mut c_void)>,
    lock_ctx: *mut c_void,
    bind_flags: u32,
    misc_flags: u32,
}

/// Spiegel von `AVD3D11VAFramesContext`.
#[repr(C)]
struct D3D11VAFramesContext {
    texture: *mut c_void,
    bind_flags: u32,
    misc_flags: u32,
    texture_infos: *mut c_void,
}

/// FFmpeg-Gerätekontext auf demselben D3D11-Gerät wie Aufnahme und Umwandlung.
pub struct HwDevice {
    ctx: AVHWDeviceContext,
}

impl HwDevice {
    pub fn from_gpu(gpu: &GpuDevice) -> Result<Self> {
        let mut ctx = AVHWDeviceContext::alloc(ffi::AV_HWDEVICE_TYPE_D3D11VA);
        unsafe {
            let hw = &mut *(ctx.data as *mut ffi::AVHWDeviceContext);
            let d3d = &mut *(hw.hwctx as *mut D3D11VADeviceContext);
            // FFmpeg ruft beim Freigeben `Release` auf diesen Zeigern auf; deshalb eigene
            // Referenzen übergeben. Video-Gerät und -Kontext holt FFmpeg in `init` selbst.
            // `lock` bleibt leer: FFmpeg legt dann einen eigenen Mutex an; das Gerät ist
            // zusätzlich per `SetMultithreadProtected` geschützt.
            d3d.device = gpu.device.clone().into_raw();
            d3d.device_context = gpu.context.clone().into_raw();
            d3d.video_device = ptr::null_mut();
            d3d.video_context = ptr::null_mut();
        }
        ctx.init()?;
        Ok(HwDevice { ctx })
    }

    pub fn context(&self) -> &AVHWDeviceContext {
        &self.ctx
    }
}

/// Pool von NV12-Texturen einer festen Größe.
pub struct HwFrames {
    ctx: AVHWFramesContext,
    width: u32,
    height: u32,
}

impl HwFrames {
    pub fn new(device: &HwDevice, width: u32, height: u32) -> Result<Self> {
        let mut ctx = device.ctx.hwframe_ctx_alloc();
        {
            let data = ctx.data();
            data.format = ffi::AV_PIX_FMT_D3D11;
            data.sw_format = ffi::AV_PIX_FMT_NV12;
            data.width = width as i32;
            data.height = height as i32;
            data.initial_pool_size = 0;
            let d3d = unsafe { &mut *(data.hwctx as *mut D3D11VAFramesContext) };
            d3d.bind_flags = D3D11_BIND_RENDER_TARGET.0 as u32;
            d3d.misc_flags = 0;
        }
        ctx.init()?;
        Ok(HwFrames { ctx, width, height })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Eigene Referenz auf den Kontext, z. B. für `AVCodecContext::hw_frames_ctx`.
    pub fn context(&self) -> AVHWFramesContext {
        self.ctx.clone()
    }

    /// Holt eine freie Textur aus dem Pool.
    pub fn get(&mut self) -> Result<HwFrame> {
        let mut frame = AVFrame::new();
        self.ctx.get_buffer(&mut frame)?;
        Ok(HwFrame { frame })
    }
}

/// Ein Frame aus dem Pool; `data[0]` ist die Textur, `data[1]` der Index im Array.
pub struct HwFrame {
    pub frame: AVFrame,
}

impl HwFrame {
    /// Textur und Array-Index; geliehen, solange das Frame lebt.
    pub fn texture(&self) -> Result<(&ID3D11Texture2D, u32)> {
        frame_texture(&self.frame)
    }
}

/// Textur und Array-Index eines D3D11-Frames (`data[0]`, `data[1]`), geliehen vom Frame.
pub(crate) fn frame_texture(frame: &AVFrame) -> Result<(&ID3D11Texture2D, u32)> {
    if frame.format != ffi::AV_PIX_FMT_D3D11 {
        return Err(MediaError::NoTexture);
    }
    let raw = &frame.data[0] as *const *mut u8 as *const *mut c_void;
    let tex = unsafe { ID3D11Texture2D::from_raw_borrowed(&*raw) }.ok_or(MediaError::NoTexture)?;
    Ok((tex, frame.data[1] as usize as u32))
}
