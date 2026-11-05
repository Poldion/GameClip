// SPDX-License-Identifier: GPL-3.0-or-later
//! Skalieren und Umwandeln auf der GPU mit dem D3D11-VideoProcessor.
//!
//! Eingabe BGRA (Aufnahme) oder NV12 (Worker), Ausgabe NV12 in BT.709 mit begrenztem
//! Wertebereich, Seitenverhältnis mit schwarzem Rand. Ziele sind meist Slices eines
//! Texture-Arrays aus FFmpegs Frame-Pool.

use std::cell::RefCell;
use std::mem::ManuallyDrop;

use windows_core::Interface;

use windows::Win32::Foundation::{RECT, TRUE};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_TEX2D_ARRAY_VPOV, D3D11_TEX2D_VPIV, D3D11_TEX2D_VPOV, D3D11_TEXTURE2D_DESC,
    D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE, D3D11_VIDEO_PROCESSOR_CONTENT_DESC,
    D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0,
    D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC, D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0,
    D3D11_VIDEO_PROCESSOR_STREAM, D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
    D3D11_VPIV_DIMENSION_TEXTURE2D, D3D11_VPOV_DIMENSION_TEXTURE2D,
    D3D11_VPOV_DIMENSION_TEXTURE2DARRAY, ID3D11Texture2D, ID3D11VideoProcessor,
    ID3D11VideoProcessorEnumerator, ID3D11VideoProcessorInputView, ID3D11VideoProcessorOutputView,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709, DXGI_COLOR_SPACE_TYPE,
    DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709, DXGI_RATIONAL,
};

use crate::device::GpuDevice;
use crate::{GpuError, Result};

/// Format der Eingabetextur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InFormat {
    Bgra,
    Nv12,
}

/// Eine Textur und der Index im Texture-Array (0 bei Einzeltexturen).
#[derive(Clone, Copy)]
pub struct TexRef<'a> {
    pub tex: &'a ID3D11Texture2D,
    pub array_slice: u32,
}

pub struct Converter {
    gpu_device: windows::Win32::Graphics::Direct3D11::ID3D11VideoDevice,
    video_context: windows::Win32::Graphics::Direct3D11::ID3D11VideoContext1,
    enumerator: ID3D11VideoProcessorEnumerator,
    processor: ID3D11VideoProcessor,
    in_size: (u32, u32),
    out_size: (u32, u32),
    /// Views je (Textur, Slice). Neue Views je Bild kosteten auf der RX 6750 XT rund 2,5 ms
    /// (gemessen 2026-11). Eine View hält ihre Textur am Leben, der Zeiger bleibt also eindeutig;
    /// die Obergrenze verhindert, dass dynamische Pools den Speicher wachsen lassen.
    inputs: RefCell<Vec<((usize, u32), ID3D11VideoProcessorInputView)>>,
    outputs: RefCell<Vec<((usize, u32), ID3D11VideoProcessorOutputView)>>,
}

/// Höchstzahl zwischengespeicherter Views je Richtung.
const VIEW_CACHE: usize = 64;

fn cached<V: Clone>(
    cache: &RefCell<Vec<((usize, u32), V)>>,
    key: (usize, u32),
    make: impl FnOnce() -> Result<V>,
) -> Result<V> {
    if let Some((_, v)) = cache.borrow().iter().find(|(k, _)| *k == key) {
        return Ok(v.clone());
    }
    let v = make()?;
    let mut c = cache.borrow_mut();
    if c.len() >= VIEW_CACHE {
        c.clear();
    }
    c.push((key, v.clone()));
    Ok(v)
}

impl Converter {
    pub fn new(
        gpu: &GpuDevice,
        in_format: InFormat,
        in_size: (u32, u32),
        out_size: (u32, u32),
    ) -> Result<Self> {
        for (w, h) in [in_size, out_size] {
            if w == 0 || h == 0 {
                return Err(GpuError::InvalidSize(w, h));
            }
        }
        let rate = DXGI_RATIONAL {
            Numerator: 60,
            Denominator: 1,
        };
        let content = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: rate,
            InputWidth: in_size.0,
            InputHeight: in_size.1,
            OutputFrameRate: rate,
            OutputWidth: out_size.0,
            OutputHeight: out_size.1,
            Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };
        let enumerator = unsafe { gpu.video_device.CreateVideoProcessorEnumerator(&content)? };
        let processor = unsafe { gpu.video_device.CreateVideoProcessor(&enumerator, 0)? };

        let in_space: DXGI_COLOR_SPACE_TYPE = match in_format {
            InFormat::Bgra => DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709,
            InFormat::Nv12 => DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
        };
        let (src, dst) = letterbox(in_size, out_size);
        unsafe {
            let vc = &gpu.video_context;
            vc.VideoProcessorSetStreamColorSpace1(&processor, 0, in_space);
            vc.VideoProcessorSetOutputColorSpace1(
                &processor,
                DXGI_COLOR_SPACE_YCBCR_STUDIO_G22_LEFT_P709,
            );
            vc.VideoProcessorSetStreamFrameFormat(
                &processor,
                0,
                D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            );
            vc.VideoProcessorSetStreamAutoProcessingMode(&processor, 0, false);
            vc.VideoProcessorSetStreamSourceRect(&processor, 0, true, Some(&src));
            vc.VideoProcessorSetStreamDestRect(&processor, 0, true, Some(&dst));
            let full = RECT {
                left: 0,
                top: 0,
                right: out_size.0 as i32,
                bottom: out_size.1 as i32,
            };
            vc.VideoProcessorSetOutputTargetRect(&processor, true, Some(&full));
            // Schwarzer Rand als RGB. Der AMD-Treiber liest eine YCbCr-Hintergrundfarbe als RGB
            // (gemessen: Y = 105 statt 16), RGB-Schwarz ist dagegen eindeutig.
            let black = windows::Win32::Graphics::Direct3D11::D3D11_VIDEO_COLOR {
                Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_VIDEO_COLOR_0 {
                    RGBA: windows::Win32::Graphics::Direct3D11::D3D11_VIDEO_COLOR_RGBA {
                        R: 0.0,
                        G: 0.0,
                        B: 0.0,
                        A: 1.0,
                    },
                },
            };
            vc.VideoProcessorSetOutputBackgroundColor(&processor, false, &black);
        }

        Ok(Converter {
            gpu_device: gpu.video_device.clone(),
            video_context: gpu.video_context.clone(),
            enumerator,
            processor,
            in_size,
            out_size,
            inputs: RefCell::new(Vec::new()),
            outputs: RefCell::new(Vec::new()),
        })
    }

    pub fn in_size(&self) -> (u32, u32) {
        self.in_size
    }

    pub fn out_size(&self) -> (u32, u32) {
        self.out_size
    }

    /// Wandelt `src` in `dst` um. `dst` muss NV12 mit `D3D11_BIND_RENDER_TARGET` sein.
    pub fn run(&self, src: TexRef<'_>, dst: TexRef<'_>) -> Result<()> {
        let input = cached(
            &self.inputs,
            (src.tex.as_raw() as usize, src.array_slice),
            || self.input_view(src),
        )?;
        let output = cached(
            &self.outputs,
            (dst.tex.as_raw() as usize, dst.array_slice),
            || self.output_view(dst),
        )?;
        let stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: TRUE,
            pInputSurface: ManuallyDrop::new(Some(input)),
            ..Default::default()
        };
        let streams = [stream];
        let result = unsafe {
            self.video_context
                .VideoProcessorBlt(&self.processor, &output, 0, &streams)
        };
        // Die Referenz im Stream-Struct selbst freigeben.
        let [mut stream] = streams;
        unsafe { ManuallyDrop::drop(&mut stream.pInputSurface) };
        result?;
        Ok(())
    }

    fn input_view(&self, src: TexRef<'_>) -> Result<ID3D11VideoProcessorInputView> {
        let desc = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPIV {
                    MipSlice: 0,
                    ArraySlice: src.array_slice,
                },
            },
        };
        let mut view = None;
        unsafe {
            self.gpu_device.CreateVideoProcessorInputView(
                src.tex,
                &self.enumerator,
                &desc,
                Some(&mut view),
            )?
        };
        Ok(view.expect("Input-View erzeugt"))
    }

    fn output_view(&self, dst: TexRef<'_>) -> Result<ID3D11VideoProcessorOutputView> {
        let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { dst.tex.GetDesc(&mut tex_desc) };
        let desc = if tex_desc.ArraySize > 1 {
            D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
                ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2DARRAY,
                Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                    Texture2DArray: D3D11_TEX2D_ARRAY_VPOV {
                        MipSlice: 0,
                        FirstArraySlice: dst.array_slice,
                        ArraySize: 1,
                    },
                },
            }
        } else {
            D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
                ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 },
                },
            }
        };
        let mut view = None;
        unsafe {
            self.gpu_device.CreateVideoProcessorOutputView(
                dst.tex,
                &self.enumerator,
                &desc,
                Some(&mut view),
            )?
        };
        Ok(view.expect("Output-View erzeugt"))
    }
}

/// Quell- und Zielrechteck für eine Skalierung mit erhaltenem Seitenverhältnis.
/// Die Zielmaße werden auf gerade Zahlen gerundet (NV12 braucht gerade Werte).
pub fn letterbox(in_size: (u32, u32), out_size: (u32, u32)) -> (RECT, RECT) {
    let (iw, ih) = (f64::from(in_size.0), f64::from(in_size.1));
    let (ow, oh) = (f64::from(out_size.0), f64::from(out_size.1));
    let scale = (ow / iw).min(oh / ih);
    let w = ((iw * scale / 2.0).round() * 2.0) as i32;
    let h = ((ih * scale / 2.0).round() * 2.0) as i32;
    let left = ((out_size.0 as i32 - w) / 2) & !1;
    let top = ((out_size.1 as i32 - h) / 2) & !1;
    let src = RECT {
        left: 0,
        top: 0,
        right: in_size.0 as i32,
        bottom: in_size.1 as i32,
    };
    let dst = RECT {
        left,
        top,
        right: left + w,
        bottom: top + h,
    };
    (src, dst)
}

#[cfg(test)]
mod tests {
    use super::letterbox;

    #[test]
    fn same_aspect_fills_output() {
        let (_, dst) = letterbox((2560, 1440), (1920, 1080));
        assert_eq!(
            (dst.left, dst.top, dst.right, dst.bottom),
            (0, 0, 1920, 1080)
        );
    }

    #[test]
    fn ultrawide_gets_bars_top_and_bottom() {
        let (_, dst) = letterbox((3440, 1440), (1920, 1080));
        assert_eq!((dst.left, dst.right), (0, 1920));
        assert_eq!(dst.bottom - dst.top, 804);
        assert_eq!(dst.top, 138);
    }

    #[test]
    fn four_by_three_gets_bars_left_and_right() {
        let (_, dst) = letterbox((1600, 1200), (1920, 1080));
        assert_eq!((dst.top, dst.bottom), (0, 1080));
        assert_eq!(dst.right - dst.left, 1440);
        assert_eq!(dst.left, 240);
    }
}
