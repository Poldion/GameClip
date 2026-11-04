// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests auf der echten GPU: BGRA → NV12 über den VideoProcessor, Werte zurücklesen.
#![cfg(windows)]

use gameclip_gpu::{Converter, GpuDevice, InFormat, MonitorTarget, TexRef, hdr};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SUBRESOURCE_DATA, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC,
};

fn tex_desc(w: u32, h: u32, format: DXGI_FORMAT, array: u32) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: w,
        Height: h,
        MipLevels: 1,
        ArraySize: array,
        Format: format,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        ..Default::default()
    }
}

/// BGRA-Textur, deren obere Hälfte `top` und untere Hälfte `bottom` hat (je [B, G, R, A]).
fn bgra_texture(gpu: &GpuDevice, w: u32, h: u32, top: [u8; 4], bottom: [u8; 4]) -> ID3D11Texture2D {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let c = if y < h / 2 { top } else { bottom };
        for _ in 0..w {
            pixels.extend_from_slice(&c);
        }
    }
    let mut desc = tex_desc(w, h, DXGI_FORMAT_B8G8R8A8_UNORM, 1);
    desc.BindFlags = (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32;
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: pixels.as_ptr().cast(),
        SysMemPitch: w * 4,
        SysMemSlicePitch: 0,
    };
    let mut tex = None;
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, Some(&init), Some(&mut tex))
            .unwrap()
    };
    tex.unwrap()
}

fn nv12_target(gpu: &GpuDevice, w: u32, h: u32, array: u32) -> ID3D11Texture2D {
    let mut desc = tex_desc(w, h, DXGI_FORMAT_NV12, array);
    desc.BindFlags = D3D11_BIND_RENDER_TARGET.0 as u32;
    let mut tex = None;
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut tex))
            .unwrap()
    };
    tex.unwrap()
}

/// Liest die Y-Ebene eines Slices zurück (Zeilen hintereinander, ohne Pitch).
fn read_luma(gpu: &GpuDevice, tex: &ID3D11Texture2D, slice: u32, w: u32, h: u32) -> Vec<u8> {
    let mut desc = tex_desc(w, h, DXGI_FORMAT_NV12, 1);
    desc.Usage = D3D11_USAGE_STAGING;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    let mut staging = None;
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut staging))
            .unwrap()
    };
    let staging = staging.unwrap();
    unsafe {
        gpu.context
            .CopySubresourceRegion(&staging, 0, 0, 0, 0, tex, slice, None);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        gpu.context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .unwrap();
        let base = mapped.pData as *const u8;
        let mut out = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            let row =
                std::slice::from_raw_parts(base.add((y * mapped.RowPitch) as usize), w as usize);
            out.extend_from_slice(row);
        }
        gpu.context.Unmap(&staging, 0);
        out
    }
}

fn assert_near(actual: u8, expected: u8, what: &str) {
    assert!(
        actual.abs_diff(expected) <= 2,
        "{what}: erwartet ≈{expected}, erhalten {actual}"
    );
}

#[test]
fn device_on_primary_monitor() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    assert!(!gpu.adapter_name().is_empty());
    assert_ne!(gpu.adapter_luid, 0);
    gpu.check_removed().unwrap();
    gpu.winrt_device().unwrap();
    let hdr = hdr::is_hdr_active(&gpu.output).unwrap();
    println!("Adapter: {} · HDR: {hdr}", gpu.adapter_name());
}

#[test]
fn bgra_to_nv12_limited_range_bt709() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let (w, h) = (64, 64);
    let white = [255, 255, 255, 255];
    let black = [0, 0, 0, 255];
    let src = bgra_texture(&gpu, w, h, white, black);
    let dst = nv12_target(&gpu, w, h, 1);
    let conv = Converter::new(&gpu, InFormat::Bgra, (w, h), (w, h)).unwrap();
    conv.run(
        TexRef {
            tex: &src,
            array_slice: 0,
        },
        TexRef {
            tex: &dst,
            array_slice: 0,
        },
    )
    .unwrap();
    let y = read_luma(&gpu, &dst, 0, w, h);
    assert_near(y[(8 * w + 32) as usize], 235, "weiß");
    assert_near(y[(56 * w + 32) as usize], 16, "schwarz");
}

#[test]
fn red_has_bt709_luma() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let (w, h) = (64, 64);
    let red = [0, 0, 255, 255];
    let src = bgra_texture(&gpu, w, h, red, red);
    let dst = nv12_target(&gpu, w, h, 1);
    let conv = Converter::new(&gpu, InFormat::Bgra, (w, h), (w, h)).unwrap();
    conv.run(
        TexRef {
            tex: &src,
            array_slice: 0,
        },
        TexRef {
            tex: &dst,
            array_slice: 0,
        },
    )
    .unwrap();
    // BT.709 begrenzt: Y = 16 + 219 × 0,2126 ≈ 63 (BT.601 ergäbe ≈ 81).
    assert_near(
        read_luma(&gpu, &dst, 0, w, h)[(32 * w + 32) as usize],
        63,
        "rot",
    );
}

#[test]
fn nv12_render_target_arrays_are_rejected_on_some_drivers() {
    // Befund auf AMD RX 6750 XT (2026-11): NV12-Arrays nur mit BIND_DECODER, Render-Targets
    // nur als Einzeltexturen. Der Test hält das Verhalten fest; Ziele sind deshalb immer
    // Einzeltexturen (Design, Datei 27).
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let mut desc = tex_desc(64, 64, DXGI_FORMAT_NV12, 4);
    desc.BindFlags = D3D11_BIND_RENDER_TARGET.0 as u32;
    let mut tex = None;
    let array_rt = unsafe { gpu.device.CreateTexture2D(&desc, None, Some(&mut tex)) };
    println!(
        "NV12-Array als Render-Target: {}",
        if array_rt.is_ok() {
            "erlaubt"
        } else {
            "abgelehnt"
        }
    );
}

#[test]
fn letterbox_into_single_target() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let white = [255, 255, 255, 255];
    // 64×32 in 64×64: oben und unten je 16 Zeilen schwarzer Rand.
    let src = bgra_texture(&gpu, 64, 32, white, white);
    let dst = nv12_target(&gpu, 64, 64, 1);
    let conv = Converter::new(&gpu, InFormat::Bgra, (64, 32), (64, 64)).unwrap();
    conv.run(
        TexRef {
            tex: &src,
            array_slice: 0,
        },
        TexRef {
            tex: &dst,
            array_slice: 0,
        },
    )
    .unwrap();
    let y = read_luma(&gpu, &dst, 0, 64, 64);
    assert_near(y[4 * 64 + 32], 16, "Rand oben");
    assert_near(y[32 * 64 + 32], 235, "Bild");
    assert_near(y[60 * 64 + 32], 16, "Rand unten");
}
