// SPDX-License-Identifier: GPL-3.0-or-later
//! Tests auf der echten GPU: Encoder-Prüfung und die Kette BGRA → NV12-Pool → Hardware-Encoder.
#![cfg(windows)]

use gameclip_gpu::{Converter, GpuDevice, InFormat, MonitorTarget, TexRef};
use gameclip_media::encoder::{TIME_BASE_DEN, is_hardware};
use gameclip_media::{EncodeProfile, EncoderConfig, HwDevice, HwFrames, VideoEncoder, probe};
use rsmpeg::ffi;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

const CFG: EncoderConfig = EncoderConfig {
    width: 1920,
    height: 1080,
    fps: 60,
    bitrate_bps: 12_000_000,
    profile: EncodeProfile::Capture,
};

fn bgra_source(gpu: &GpuDevice, w: u32, h: u32) -> ID3D11Texture2D {
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
        BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
        ..Default::default()
    };
    let mut tex = None;
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut tex))
            .unwrap()
    };
    tex.unwrap()
}

#[test]
fn probe_reports_every_candidate() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let result = probe(&gpu, &CFG);
    for c in &result {
        println!("{}: {} {}", c.name, c.ok, c.reason.as_deref().unwrap_or(""));
    }
    assert_eq!(result.last().unwrap().name, "libx264");
    assert!(result.last().unwrap().ok, "libx264 muss immer gehen");
    if gpu.vendor_id() == 0x1002 {
        assert!(result[0].ok, "h264_amf auf AMD-Hardware");
    }
}

#[test]
fn capture_chain_keeps_forced_keyframe() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let Some(name) = probe(&gpu, &CFG)
        .into_iter()
        .find(|c| c.ok && is_hardware(&c.name))
        .map(|c| c.name)
    else {
        println!("kein Hardware-Encoder verfügbar, Test übersprungen");
        return;
    };

    // Ultrawide-Quelle, damit der Converter auch skaliert und Ränder setzt.
    let src = bgra_source(&gpu, 2560, 1080);
    let conv = Converter::new(&gpu, InFormat::Bgra, (2560, 1080), (CFG.width, CFG.height)).unwrap();
    let device = HwDevice::from_gpu(&gpu).unwrap();
    let mut frames = HwFrames::new(&device, CFG.width, CFG.height).unwrap();
    let mut enc = VideoEncoder::open(&name, &CFG, Some(&frames)).unwrap();

    let frame_us = i64::from(TIME_BASE_DEN) / i64::from(CFG.fps);
    let mut packets = Vec::new();
    for i in 0..90 {
        let mut f = frames.get().unwrap();
        {
            let (tex, slice) = f.texture().unwrap();
            conv.run(
                TexRef {
                    tex: &src,
                    array_slice: 0,
                },
                TexRef {
                    tex,
                    array_slice: slice,
                },
            )
            .unwrap();
        }
        // Bild 37 liegt mitten in der zweiten GOP: ein Schnappschuss erzwingt dort ein IDR.
        enc.send(&mut f.frame, i * frame_us, i == 0 || i == 37)
            .unwrap();
        while let Some(p) = enc.receive().unwrap() {
            packets.push((p.pts, p.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0));
        }
    }
    enc.flush().unwrap();
    while let Some(p) = enc.receive().unwrap() {
        packets.push((p.pts, p.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0));
    }

    let keys: Vec<i64> = packets
        .iter()
        .filter(|(_, k)| *k)
        .map(|(pts, _)| pts / frame_us)
        .collect();
    println!(
        "{name}: {} Pakete, Keyframes bei Bild {keys:?}",
        packets.len()
    );
    assert_eq!(packets.len(), 90);
    assert!(packets[0].1, "erstes Paket ist ein Keyframe");
    assert!(keys.contains(&37), "erzwungener Keyframe bei Bild 37");
    // Keine B-Frames: Pakete kommen in Anzeigereihenfolge.
    assert!(packets.windows(2).all(|w| w[0].0 < w[1].0));
}

/// Kodiert `count` Bilder auf dem Hardware-Encoder; `force` sagt je Bild, ob ein IDR verlangt
/// wird. Liefert die Bildnummern der Keyframes oder `None` ohne Hardware-Encoder.
fn hw_keyframes(count: i64, force: impl Fn(i64) -> bool) -> Option<(String, Vec<i64>)> {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let name = probe(&gpu, &CFG)
        .into_iter()
        .find(|c| c.ok && is_hardware(&c.name))?
        .name;
    let src = bgra_source(&gpu, CFG.width, CFG.height);
    let conv = Converter::new(
        &gpu,
        InFormat::Bgra,
        (CFG.width, CFG.height),
        (CFG.width, CFG.height),
    )
    .unwrap();
    let device = HwDevice::from_gpu(&gpu).unwrap();
    let mut frames = HwFrames::new(&device, CFG.width, CFG.height).unwrap();
    let mut enc = VideoEncoder::open(&name, &CFG, Some(&frames)).unwrap();
    let frame_us = i64::from(TIME_BASE_DEN) / i64::from(CFG.fps);
    let mut keys = Vec::new();
    let mut collect = |enc: &mut VideoEncoder| {
        while let Some(p) = enc.receive().unwrap() {
            if p.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0 {
                keys.push(p.pts / frame_us);
            }
        }
    };
    for i in 0..count {
        let mut f = frames.get().unwrap();
        {
            let (tex, slice) = f.texture().unwrap();
            conv.run(
                TexRef {
                    tex: &src,
                    array_slice: 0,
                },
                TexRef {
                    tex,
                    array_slice: slice,
                },
            )
            .unwrap();
        }
        enc.send(&mut f.frame, i * frame_us, force(i)).unwrap();
        collect(&mut enc);
    }
    enc.flush().unwrap();
    collect(&mut enc);
    Some((name, keys))
}

#[test]
fn regular_keyframe_every_second() {
    // GOP = fps: ohne Anforderung ein Keyframe je Sekunde (Bild 0, 60, 120, 180).
    let Some((name, keys)) = hw_keyframes(200, |i| i == 0) else {
        return;
    };
    println!("{name}: Keyframes bei Bild {keys:?}");
    assert_eq!(keys, [0, 60, 120, 180]);
}

#[test]
fn forced_keyframe_restarts_the_second() {
    // IDR bei Bild 37 → nächster regulärer Keyframe eine Sekunde später (97), nicht bei 60.
    let Some((name, keys)) = hw_keyframes(200, |i| i == 37) else {
        return;
    };
    println!("{name}: Keyframes bei Bild {keys:?}");
    assert_eq!(keys, [0, 37, 97, 157]);
}
