// SPDX-License-Identifier: GPL-3.0-or-later
//! H.264-Encoder: Auswahl, Optionen je Profil und Prüfung mit einer echten Testsitzung.
//!
//! Zeitbasis ist immer 1/1 000 000 (Mikrosekunden), damit Aufnahmezeiten direkt als PTS dienen.

use std::ffi::CString;

use gameclip_gpu::GpuDevice;
use rsmpeg::UnsafeDerefMut;
use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVPacket};
use rsmpeg::avutil::{AVDictionary, AVFrame};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;

use crate::hwdevice::{HwDevice, HwFrames};
use crate::{MediaError, Result};

/// Mikrosekunden je Sekunde, zugleich Nenner der Zeitbasis.
pub const TIME_BASE_DEN: i32 = 1_000_000;

const VENDOR_NVIDIA: u32 = 0x10DE;
const VENDOR_AMD: u32 = 0x1002;
const VENDOR_INTEL: u32 = 0x8086;

/// Anzahl Bilder der Testsitzung in `probe`.
const PROBE_FRAMES: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeProfile {
    /// Ringpuffer: niedrige Latenz, konstante Bitrate.
    Capture,
    /// Kürzen und Größen-Export im Worker.
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_bps: u64,
    pub profile: EncodeProfile,
}

/// Ergebnis der Prüfung eines Encoders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncoderCandidate {
    pub name: String,
    pub ok: bool,
    pub reason: Option<String>,
}

/// Prüfreihenfolge: Hersteller-Encoder der Monitor-GPU, dann `libx264`.
/// `h264_mf` kommt erst nach S2 hinzu (Cargo-Feature `media-foundation`, Design Q5).
pub fn candidate_order(vendor_id: u32) -> Vec<&'static str> {
    let vendor = match vendor_id {
        VENDOR_NVIDIA => Some("h264_nvenc"),
        VENDOR_AMD => Some("h264_amf"),
        VENDOR_INTEL => Some("h264_qsv"),
        _ => None,
    };
    vendor.into_iter().chain(["libx264"]).collect()
}

/// `true` für Encoder, die D3D11-Frames annehmen.
pub fn is_hardware(name: &str) -> bool {
    name != "libx264"
}

/// Bilder je Bildgruppe. Die Aufnahme braucht einen Keyframe je Sekunde (Ringpuffer, Schnitt an
/// Bildgruppen). Exporte nehmen 5 s: Bei niedrigen Zielbitraten verbrauchen Keyframes sonst den
/// Großteil der Bits (S11, 2026-11).
pub fn gop_frames(fps: u32, profile: EncodeProfile) -> u32 {
    let fps = fps.max(1);
    match profile {
        EncodeProfile::Capture => fps,
        EncodeProfile::Export => fps * 5,
    }
}

/// Encoder-spezifische Optionen (Design, Datei 27).
fn private_options(name: &str, profile: EncodeProfile) -> Vec<(&'static str, &'static str)> {
    use EncodeProfile::*;
    match (name, profile) {
        ("h264_nvenc", Capture) => vec![
            ("preset", "p4"),
            ("tune", "ll"),
            ("rc", "cbr"),
            ("forced-idr", "1"),
        ],
        ("h264_nvenc", Export) => vec![("preset", "p5"), ("rc", "vbr"), ("forced-idr", "1")],
        ("h264_amf", Capture) => vec![("usage", "lowlatency"), ("rc", "cbr"), ("forced_idr", "1")],
        ("h264_amf", Export) => vec![
            ("usage", "transcoding"),
            ("rc", "vbr_peak"),
            ("forced_idr", "1"),
        ],
        ("libx264", Capture) => vec![("preset", "superfast"), ("tune", "zerolatency")],
        ("libx264", Export) => vec![("preset", "veryfast")],
        _ => vec![],
    }
}

pub struct VideoEncoder {
    ctx: AVCodecContext,
    name: String,
    hardware: bool,
    /// Bilder je Bildgruppe (`gop_frames`).
    gop: u32,
    /// Bilder seit dem letzten angeforderten IDR.
    since_key: u32,
}

impl VideoEncoder {
    /// Öffnet `name`. Hardware-Encoder brauchen `frames` (D3D11-Pool in Ausgabegröße).
    pub fn open(name: &str, cfg: &EncoderConfig, frames: Option<&HwFrames>) -> Result<Self> {
        if name == "h264_qsv" {
            // QSV braucht einen abgeleiteten QSV-Frames-Kontext mit festem Pool; ob das mit
            // eigenen Einzeltexturen geht, klärt S2 auf Intel-Hardware.
            return Err(MediaError::EncoderMissing(
                "h264_qsv (D3D11-Eingabe erst nach S2)".into(),
            ));
        }
        let cname = CString::new(name).expect("Encodername ohne NUL");
        let codec = AVCodec::find_encoder_by_name(&cname)
            .ok_or_else(|| MediaError::EncoderMissing(name.into()))?;
        let hardware = is_hardware(name);

        let mut ctx = AVCodecContext::new(&codec);
        ctx.set_width(cfg.width as i32);
        ctx.set_height(cfg.height as i32);
        ctx.set_time_base(ffi::AVRational {
            num: 1,
            den: TIME_BASE_DEN,
        });
        ctx.set_framerate(ffi::AVRational {
            num: cfg.fps as i32,
            den: 1,
        });
        let gop = gop_frames(cfg.fps, cfg.profile);
        ctx.set_gop_size(gop as i32);
        ctx.set_max_b_frames(0);
        ctx.set_bit_rate(cfg.bitrate_bps as i64);
        ctx.set_flags(ctx.flags | ffi::AV_CODEC_FLAG_CLOSED_GOP as i32);
        unsafe {
            let raw = ctx.deref_mut();
            raw.rc_max_rate = cfg.bitrate_bps as i64;
            raw.rc_buffer_size = cfg.bitrate_bps.min(i32::MAX as u64) as i32;
        }
        if hardware {
            let frames = frames.ok_or(MediaError::NoTexture)?;
            ctx.set_pix_fmt(ffi::AV_PIX_FMT_D3D11);
            ctx.set_hw_frames_ctx(frames.context());
        } else {
            ctx.set_pix_fmt(ffi::AV_PIX_FMT_NV12);
        }

        let mut dict = AVDictionary::new(c"profile", c"high", 0);
        for (key, value) in private_options(name, cfg.profile) {
            let key = CString::new(key).expect("Schlüssel ohne NUL");
            let value = CString::new(value).expect("Wert ohne NUL");
            dict = dict.set(&key, &value, 0);
        }
        ctx.open(Some(dict))?;
        Ok(VideoEncoder {
            ctx,
            name: name.into(),
            hardware,
            gop,
            // Das erste Bild wird damit immer ein IDR.
            since_key: gop,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_hardware(&self) -> bool {
        self.hardware
    }

    /// Parameter für den Muxer (SPS/PPS in `extradata`).
    pub fn codec_parameters(&self) -> rsmpeg::avcodec::AVCodecParameters {
        self.ctx.extract_codecpar()
    }

    /// Sendet ein Bild mit Zeitstempel in µs. `keyframe` erzwingt ein IDR.
    ///
    /// Spätestens nach einer Bildgruppe wird ebenfalls ein IDR verlangt. `gop_size` allein reicht
    /// nicht: AMF mit `usage lowlatency`/`ultralowlatency` setzt dann nur den ersten Keyframe
    /// (gemessen auf RX 6750 XT, 2026-11); der Ringpuffer braucht aber einen je Sekunde.
    pub fn send(&mut self, frame: &mut AVFrame, pts_us: i64, keyframe: bool) -> Result<()> {
        let keyframe = keyframe || self.since_key >= self.gop;
        self.since_key = if keyframe { 1 } else { self.since_key + 1 };
        frame.set_pts(pts_us);
        frame.set_pict_type(if keyframe {
            ffi::AV_PICTURE_TYPE_I
        } else {
            ffi::AV_PICTURE_TYPE_NONE
        });
        self.ctx.send_frame(Some(frame))?;
        Ok(())
    }

    /// Nächstes fertiges Paket, `None` wenn der Encoder gerade keins hat oder geleert ist.
    pub fn receive(&mut self) -> Result<Option<AVPacket>> {
        match self.ctx.receive_packet() {
            Ok(p) => Ok(Some(p)),
            Err(RsmpegError::EncoderDrainError | RsmpegError::EncoderFlushedError) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Leitet das Leeren ein; danach `receive` bis `None`.
    pub fn flush(&mut self) -> Result<()> {
        self.ctx.send_frame(None)?;
        Ok(())
    }
}

/// Prüft die Encoder in `candidate_order` mit je einer Testsitzung über 10 Bilder.
pub fn probe(gpu: &GpuDevice, cfg: &EncoderConfig) -> Vec<EncoderCandidate> {
    candidate_order(gpu.vendor_id())
        .into_iter()
        .map(|name| match test_session(gpu, name, cfg) {
            Ok(()) => EncoderCandidate {
                name: name.into(),
                ok: true,
                reason: None,
            },
            Err(e) => EncoderCandidate {
                name: name.into(),
                ok: false,
                reason: Some(e.to_string()),
            },
        })
        .collect()
}

fn test_session(gpu: &GpuDevice, name: &str, cfg: &EncoderConfig) -> Result<()> {
    let frame_us = i64::from(TIME_BASE_DEN) / i64::from(cfg.fps.max(1));
    let mut packets = Vec::new();
    if is_hardware(name) {
        let device = HwDevice::from_gpu(gpu)?;
        let mut frames = HwFrames::new(&device, cfg.width, cfg.height)?;
        let mut enc = VideoEncoder::open(name, cfg, Some(&frames))?;
        for i in 0..PROBE_FRAMES {
            let mut f = frames.get()?;
            enc.send(&mut f.frame, i * frame_us, i == 0)?;
            while let Some(p) = enc.receive()? {
                packets.push(p);
            }
        }
        drain(&mut enc, &mut packets)?;
    } else {
        let mut enc = VideoEncoder::open(name, cfg, None)?;
        for i in 0..PROBE_FRAMES {
            let mut f = AVFrame::new();
            f.set_format(ffi::AV_PIX_FMT_NV12);
            f.set_width(cfg.width as i32);
            f.set_height(cfg.height as i32);
            f.get_buffer(0)?;
            enc.send(&mut f, i * frame_us, i == 0)?;
            while let Some(p) = enc.receive()? {
                packets.push(p);
            }
        }
        drain(&mut enc, &mut packets)?;
    }
    check_packets(&packets)
}

fn drain(enc: &mut VideoEncoder, packets: &mut Vec<AVPacket>) -> Result<()> {
    enc.flush()?;
    while let Some(p) = enc.receive()? {
        packets.push(p);
    }
    Ok(())
}

fn check_packets(packets: &[AVPacket]) -> Result<()> {
    let first = packets
        .first()
        .ok_or(MediaError::ProbeFailed("kein Paket erhalten"))?;
    if first.flags & ffi::AV_PKT_FLAG_KEY as i32 == 0 {
        return Err(MediaError::ProbeFailed("erstes Paket ist kein Keyframe"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::candidate_order;

    #[test]
    fn vendor_encoder_comes_first_then_software() {
        assert_eq!(candidate_order(0x10DE), ["h264_nvenc", "libx264"]);
        assert_eq!(candidate_order(0x1002), ["h264_amf", "libx264"]);
        assert_eq!(candidate_order(0x8086), ["h264_qsv", "libx264"]);
        assert_eq!(candidate_order(0x1414), ["libx264"]);
    }
}
