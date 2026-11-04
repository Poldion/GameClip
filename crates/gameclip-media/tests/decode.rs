// SPDX-License-Identifier: GPL-3.0-or-later
//! Bildzeiten, Tags und Dekodieren (Software, D3D11VA, Hochladen) mit nummerierten Bildern.
#![cfg(windows)]

use std::path::PathBuf;

use gameclip_gpu::{Converter, GpuDevice, InFormat, MonitorTarget, TexRef};
use gameclip_media::decode::{Decoder, Uploader};
use gameclip_media::demux::{frame_times_us, read_tags};
use gameclip_media::encoder::TIME_BASE_DEN;
use gameclip_media::mux::{Track, write_mp4};
use gameclip_media::{
    AudioEncoder, EncodeProfile, EncoderConfig, HwDevice, HwFrames, VideoEncoder,
};
use gameclip_testsupport::synth::{nv12_frame, read_number, sine};
use rsmpeg::ffi;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};

const W: u32 = 640;
const H: u32 = 360;
const FPS: u32 = 30;
const FRAMES: u32 = 90;
const FRAME_US: i64 = 1_000_000 / FPS as i64;

/// Schreibt einmal je Testlauf einen 3-s-Clip mit nummerierten Bildern und Ton.
fn test_clip(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("gameclip-decode-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);

    let cfg = EncoderConfig {
        width: W,
        height: H,
        fps: FPS,
        bitrate_bps: 6_000_000,
        profile: EncodeProfile::Capture,
    };
    let mut venc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let mut vpk = Vec::new();
    for i in 0..FRAMES {
        let mut f = nv12_frame(i, W, H);
        venc.send(&mut f, i64::from(i) * FRAME_US, i == 0).unwrap();
        while let Some(p) = venc.receive().unwrap() {
            vpk.push(p);
        }
    }
    venc.flush().unwrap();
    while let Some(p) = venc.receive().unwrap() {
        vpk.push(p);
    }
    let mut aenc = AudioEncoder::new(128_000).unwrap();
    let mut apk = Vec::new();
    for start in (0..48_000 * 3).step_by(480) {
        apk.extend(aenc.push(&sine(start, 480, 440.0)).unwrap());
    }
    apk.extend(aenc.finish().unwrap());

    let (vpar, apar) = (venc.codec_parameters(), aenc.codec_parameters());
    let tb = |den| ffi::AVRational { num: 1, den };
    write_mp4(
        &path,
        &Track {
            params: &vpar,
            time_base: tb(TIME_BASE_DEN),
            packets: &vpk,
            title: "Video",
            default: true,
        },
        &[Track {
            params: &apar,
            time_base: tb(48_000),
            packets: &apk,
            title: "Mischung",
            default: true,
        }],
        &[("gameclip_tag_id", "7f3a"), ("gameclip_game", "Valorant")],
        0,
        false,
    )
    .unwrap();
    path
}

fn sw_number(f: &rsmpeg::avutil::AVFrame) -> u32 {
    let luma =
        unsafe { std::slice::from_raw_parts(f.data[0], (f.linesize[0] * f.height) as usize) };
    read_number(luma, f.linesize[0] as usize)
}

fn nv12_texture(gpu: &GpuDevice, usage_staging: bool) -> ID3D11Texture2D {
    let mut desc = D3D11_TEXTURE2D_DESC {
        Width: W,
        Height: H,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_NV12,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
        ..Default::default()
    };
    if usage_staging {
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    }
    let mut tex = None;
    unsafe {
        gpu.device
            .CreateTexture2D(&desc, None, Some(&mut tex))
            .unwrap()
    };
    tex.unwrap()
}

/// Wandelt eine (Array-)Textur über den Converter in ein Einzelziel und liest die Nummer.
fn gpu_number(gpu: &GpuDevice, conv: &Converter, tex: &ID3D11Texture2D, slice: u32) -> u32 {
    let target = nv12_texture(gpu, false);
    let staging = nv12_texture(gpu, true);
    conv.run(
        TexRef {
            tex,
            array_slice: slice,
        },
        TexRef {
            tex: &target,
            array_slice: 0,
        },
    )
    .unwrap();
    unsafe {
        gpu.context.CopyResource(&staging, &target);
        let mut m = D3D11_MAPPED_SUBRESOURCE::default();
        gpu.context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut m))
            .unwrap();
        let luma = std::slice::from_raw_parts(m.pData as *const u8, (m.RowPitch * H) as usize);
        let n = read_number(luma, m.RowPitch as usize);
        gpu.context.Unmap(&staging, 0);
        n
    }
}

fn is_key(f: &rsmpeg::avutil::AVFrame) -> bool {
    f.flags & ffi::AV_FRAME_FLAG_KEY as i32 != 0
}

#[test]
fn frame_times_and_tags() {
    let path = test_clip("times.mp4");
    let times = frame_times_us(&path).unwrap();
    assert_eq!(times.len(), FRAMES as usize);
    for (i, t) in times.iter().enumerate() {
        assert!(
            (*t as i64 - i as i64 * FRAME_US).abs() <= 1,
            "Bild {i}: {t} µs"
        );
    }
    let tags = read_tags(&path).unwrap();
    assert_eq!(tags["gameclip_tag_id"], "7f3a");
    assert_eq!(tags["gameclip_game"], "Valorant");
}

#[test]
fn software_decode_returns_every_frame_in_order() {
    let path = test_clip("sw.mp4");
    let mut dec = Decoder::open(&path, None).unwrap();
    let mut n = 0;
    while let Some(f) = dec.next_frame().unwrap() {
        assert!(!f.is_hardware());
        assert_eq!(sw_number(&f.frame), n, "Bildnummer");
        assert_eq!(f.pts_us, i64::from(n) * FRAME_US);
        n += 1;
    }
    assert_eq!(n, FRAMES);
}

#[test]
fn seek_lands_on_keyframe_before_target() {
    let path = test_clip("seek.mp4");
    let mut dec = Decoder::open(&path, None).unwrap();
    // Mindestens ein Ziel liegt zwischen Keyframes; sonst prüft der Test nichts.
    let mut jumped_back = false;
    for target in [0, 1_500_000, 2_100_000] {
        dec.seek_us(target).unwrap();
        let first = dec.next_frame().unwrap().unwrap();
        let num = sw_number(&first.frame);
        println!(
            "Ziel {target} µs → Keyframe Bild {num} bei {} µs",
            first.pts_us
        );
        assert!(
            is_key(&first.frame),
            "erstes Bild nach dem Sprung ist ein Keyframe"
        );
        assert!(first.pts_us <= target);
        if target > 0 {
            jumped_back |= first.pts_us < target - FRAME_US;
        }
        assert_eq!(i64::from(num) * FRAME_US, first.pts_us);
        // Bis zum Ziel weiterlesen: das Zielbild kommt genau.
        let mut f = first;
        while f.pts_us < target - FRAME_US / 2 {
            f = dec.next_frame().unwrap().unwrap();
        }
        assert_eq!(i64::from(sw_number(&f.frame)) * FRAME_US, f.pts_us);
    }
    assert!(jumped_back, "kein Ziel lag zwischen zwei Keyframes");
}

#[test]
fn d3d11va_decode_feeds_converter() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let device = HwDevice::from_gpu(&gpu).unwrap();
    let path = test_clip("hw.mp4");
    let mut dec = Decoder::open(&path, Some(&device)).unwrap();
    let conv = Converter::new(&gpu, InFormat::Nv12, (W, H), (W, H)).unwrap();
    let mut n = 0;
    let mut hw = 0;
    while let Some(f) = dec.next_frame().unwrap() {
        let num = if f.is_hardware() {
            hw += 1;
            let (tex, slice) = f.texture().unwrap();
            gpu_number(&gpu, &conv, tex, slice)
        } else {
            sw_number(&f.frame)
        };
        assert_eq!(num, n, "Bildnummer");
        n += 1;
    }
    println!("{n} Bilder, davon {hw} aus D3D11VA");
    assert_eq!(n, FRAMES);
    assert_eq!(hw, FRAMES, "D3D11VA soll auf dieser GPU H.264 dekodieren");
}

#[test]
fn software_frames_upload_into_pool() {
    let gpu = GpuDevice::for_monitor(MonitorTarget::Primary).unwrap();
    let device = HwDevice::from_gpu(&gpu).unwrap();
    let mut uploader = Uploader::new(HwFrames::new(&device, W, H).unwrap());
    let conv = Converter::new(&gpu, InFormat::Nv12, (W, H), (W, H)).unwrap();
    let path = test_clip("upload.mp4");
    let mut dec = Decoder::open(&path, None).unwrap();
    let mut n = 0;
    while let Some(f) = dec.next_frame().unwrap() {
        if n == 0 {
            println!("Software-Format: {}", f.frame.format);
        }
        let up = uploader.upload(&f.frame).unwrap();
        let (tex, slice) = up.texture().unwrap();
        assert_eq!(gpu_number(&gpu, &conv, tex, slice), n);
        n += 1;
    }
    assert_eq!(n, FRAMES);
}
