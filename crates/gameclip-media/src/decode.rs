// SPDX-License-Identifier: GPL-3.0-or-later
//! Videodekoder für den Worker: D3D11VA mit Ausweichen auf Software (Design, Datei 29).
//!
//! Hardware-Frames liegen als Slice eines NV12-Texture-Arrays vor (`data[0]` Textur,
//! `data[1]` Index) und gehen direkt in den `Converter`. Software-Frames lädt der Worker in ein
//! D3D11-Frame des Worker-Pools hoch (`Uploader`), damit Converter und Encoder dieselbe
//! Eingabe bekommen.

use std::path::Path;

use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use crate::demux::open_input;
use crate::hwdevice::{HwDevice, HwFrame, HwFrames};
use crate::{MediaError, Result};

const MICROS: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 1_000_000,
};

/// Ein dekodiertes Bild mit Anzeigezeit in µs ab Spurbeginn.
pub struct DecodedFrame {
    pub frame: AVFrame,
    pub pts_us: i64,
}

impl DecodedFrame {
    /// `true`, wenn das Bild in einer D3D11-Textur liegt.
    pub fn is_hardware(&self) -> bool {
        self.frame.format == ffi::AV_PIX_FMT_D3D11
    }

    /// Textur und Array-Index eines Hardware-Frames.
    pub fn texture(&self) -> Result<(&windows::Win32::Graphics::Direct3D11::ID3D11Texture2D, u32)> {
        crate::hwdevice::frame_texture(&self.frame)
    }
}

pub struct Decoder {
    input: AVFormatContextInput,
    stream_index: usize,
    ctx: AVCodecContext,
    time_base: ffi::AVRational,
    start: i64,
    hardware: bool,
    eof: bool,
}

/// Wählt D3D11, wenn der Decoder es anbietet; sonst das erste Software-Format.
unsafe extern "C" fn pick_d3d11(
    _ctx: *mut ffi::AVCodecContext,
    formats: *const ffi::AVPixelFormat,
) -> ffi::AVPixelFormat {
    let mut p = formats;
    unsafe {
        while *p != ffi::AV_PIX_FMT_NONE {
            if *p == ffi::AV_PIX_FMT_D3D11 {
                return ffi::AV_PIX_FMT_D3D11;
            }
            p = p.add(1);
        }
        *formats
    }
}

impl Decoder {
    /// Öffnet die Videospur. Mit `device` wird D3D11VA versucht, bei Fehlern Software.
    pub fn open(path: &Path, device: Option<&HwDevice>) -> Result<Self> {
        if let Some(dev) = device {
            match Self::open_with(path, Some(dev)) {
                Ok(d) => return Ok(d),
                Err(e) => eprintln!("D3D11VA-Dekoder nicht nutzbar, weiche auf Software aus: {e}"),
            }
        }
        Self::open_with(path, None)
    }

    fn open_with(path: &Path, device: Option<&HwDevice>) -> Result<Self> {
        let input = open_input(path)?;
        let (stream_index, codec) = input
            .find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)?
            .ok_or(MediaError::NoVideoStream)?;
        let stream = &input.streams()[stream_index];
        let time_base = stream.time_base;
        let start = if stream.start_time == ffi::AV_NOPTS_VALUE {
            0
        } else {
            stream.start_time
        };
        let mut ctx = AVCodecContext::new(&codec);
        ctx.apply_codecpar(&stream.codecpar())?;
        ctx.set_pkt_timebase(time_base);
        if let Some(dev) = device {
            ctx.set_hw_device_ctx(dev.context().clone());
            ctx.set_get_format(Some(pick_d3d11));
        }
        ctx.open(None)?;
        Ok(Decoder {
            input,
            stream_index,
            ctx,
            time_base,
            start,
            hardware: device.is_some(),
            eof: false,
        })
    }

    /// `true`, wenn D3D11VA angefordert wurde. Ob ein Bild wirklich in einer Textur liegt,
    /// sagt `DecodedFrame::is_hardware` (der Decoder kann in `get_format` Software wählen).
    pub fn is_hardware(&self) -> bool {
        self.hardware
    }

    /// Springt zum letzten Keyframe bei oder vor `t_us` (ab Spurbeginn). Danach liefert
    /// `next_frame` Bilder ab diesem Keyframe; der Aufrufer verwirft die vor `t_us`.
    pub fn seek_us(&mut self, t_us: i64) -> Result<()> {
        let ts = self.start + unsafe { ffi::av_rescale_q(t_us, MICROS, self.time_base) };
        self.input.seek(
            self.stream_index as i32,
            ts,
            ffi::AVSEEK_FLAG_BACKWARD as i32,
        )?;
        self.ctx.flush_buffers();
        self.eof = false;
        Ok(())
    }

    /// Nächstes Bild in Anzeigereihenfolge, `None` am Ende.
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>> {
        loop {
            match self.ctx.receive_frame() {
                Ok(frame) => {
                    let pts = if frame.best_effort_timestamp != ffi::AV_NOPTS_VALUE {
                        frame.best_effort_timestamp
                    } else {
                        frame.pts
                    };
                    let pts_us =
                        unsafe { ffi::av_rescale_q(pts - self.start, self.time_base, MICROS) };
                    return Ok(Some(DecodedFrame { frame, pts_us }));
                }
                Err(RsmpegError::DecoderDrainError) => {}
                Err(RsmpegError::DecoderFlushedError) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
            if self.eof {
                return Ok(None);
            }
            match self.input.read_packet()? {
                Some(p) if p.stream_index as usize == self.stream_index => {
                    self.ctx.send_packet(Some(&p))?;
                }
                Some(_) => {}
                None => {
                    self.ctx.send_packet(None)?;
                    self.eof = true;
                }
            }
        }
    }
}

/// Lädt Software-Frames in Texturen des Pools hoch. Der Pool nimmt nur NV12 an; andere
/// Formate (der H.264-Software-Dekoder liefert `yuv420p`) wandelt swscale vorher um.
pub struct Uploader {
    pool: HwFrames,
    /// Umwandlung für (Format, Breite, Höhe) der Quelle und Zielbild in NV12.
    sws: Option<(i32, i32, i32, SwsContext, AVFrame)>,
}

impl Uploader {
    pub fn new(pool: HwFrames) -> Self {
        Uploader { pool, sws: None }
    }

    pub fn pool(&mut self) -> &mut HwFrames {
        &mut self.pool
    }

    pub fn upload(&mut self, frame: &AVFrame) -> Result<HwFrame> {
        let mut hw = self.pool.get()?;
        if frame.format == ffi::AV_PIX_FMT_NV12 {
            hw.frame.hwframe_transfer_data(frame)?;
        } else {
            let key = (frame.format, frame.width, frame.height);
            if self.sws.as_ref().map(|s| (s.0, s.1, s.2)) != Some(key) {
                let sws = SwsContext::get_context(
                    frame.width,
                    frame.height,
                    frame.format,
                    frame.width,
                    frame.height,
                    ffi::AV_PIX_FMT_NV12,
                    ffi::SWS_POINT,
                    None,
                    None,
                    None,
                )
                .ok_or(MediaError::UnsupportedFormat(frame.format))?;
                let mut nv12 = AVFrame::new();
                nv12.set_format(ffi::AV_PIX_FMT_NV12);
                nv12.set_width(frame.width);
                nv12.set_height(frame.height);
                nv12.get_buffer(0)?;
                self.sws = Some((key.0, key.1, key.2, sws, nv12));
            }
            let (_, _, _, sws, nv12) = self.sws.as_mut().expect("eben angelegt");
            sws.scale_frame(frame, 0, frame.height, nv12)?;
            hw.frame.hwframe_transfer_data(nv12)?;
        }
        hw.frame.set_pts(frame.pts);
        Ok(hw)
    }
}
