// SPDX-License-Identifier: GPL-3.0-or-later
//! Alle Tonspuren einer Datei in PCM dekodieren (für den Worker: Kürzen und Export).
//!
//! Ergebnis je Spur: Titel (`handler_name`) und interleaved Stereo-Float zu 48 kHz; Sample 0
//! liegt bei Anzeigezeit 0. Samples vor 0 (AAC-Anlauf, durch die Edit-List ausgeblendet) fallen
//! weg. 60 s Stereo sind rund 23 MB, deshalb wird alles auf einmal dekodiert.

use std::path::Path;

use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;

use crate::demux::open_input;
use crate::{MediaError, Result};

pub const RATE: i64 = 48_000;

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub title: String,
    /// Interleaved Stereo, 48 kHz.
    pub samples: Vec<f32>,
}

impl DecodedAudio {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    /// Ausschnitt `[from_us, to_us)` auf das Sample genau; fehlende Samples am Ende als Stille.
    pub fn cut(&self, from_us: u64, to_us: u64) -> Vec<f32> {
        let a = (from_us as i64 * RATE / 1_000_000) as usize;
        let b = (to_us as i64 * RATE / 1_000_000) as usize;
        let mut out = vec![0.0f32; (b - a) * 2];
        let end = (b * 2).min(self.samples.len());
        if a * 2 < end {
            out[..end - a * 2].copy_from_slice(&self.samples[a * 2..end]);
        }
        out
    }
}

pub fn decode_audio(path: &Path) -> Result<Vec<DecodedAudio>> {
    let mut input = open_input(path)?;
    struct Stream {
        index: usize,
        title: String,
        ctx: AVCodecContext,
        time_base: ffi::AVRational,
        start: i64,
        out: Vec<f32>,
    }
    let mut streams = Vec::new();
    for (index, s) in input.streams().iter().enumerate() {
        let par = s.codecpar();
        if par.codec_type != ffi::AVMEDIA_TYPE_AUDIO {
            continue;
        }
        let codec = rsmpeg::avcodec::AVCodec::find_decoder(par.codec_id)
            .ok_or_else(|| MediaError::EncoderMissing(format!("Decoder {}", par.codec_id)))?;
        let mut ctx = AVCodecContext::new(&codec);
        ctx.apply_codecpar(&par)?;
        ctx.set_pkt_timebase(s.time_base);
        ctx.open(None)?;
        let title = s
            .metadata()
            .and_then(|m| {
                m.iter()
                    .find(|e| e.key() == c"handler_name")
                    .map(|e| e.value().to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        let start = if s.start_time == ffi::AV_NOPTS_VALUE {
            0
        } else {
            s.start_time
        };
        streams.push(Stream {
            index,
            title,
            ctx,
            time_base: s.time_base,
            start,
            out: Vec::new(),
        });
    }

    fn drain(s: &mut Stream) -> Result<()> {
        loop {
            let frame = match s.ctx.receive_frame() {
                Ok(f) => f,
                Err(RsmpegError::DecoderDrainError | RsmpegError::DecoderFlushedError) => {
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };
            let pts = if frame.best_effort_timestamp != ffi::AV_NOPTS_VALUE {
                frame.best_effort_timestamp
            } else {
                frame.pts
            };
            let first = unsafe {
                ffi::av_rescale_q(
                    pts - s.start,
                    s.time_base,
                    ffi::AVRational {
                        num: 1,
                        den: RATE as i32,
                    },
                )
            };
            let n = frame.nb_samples as usize;
            let skip = (-first).clamp(0, n as i64) as usize;
            let channels = frame.ch_layout.nb_channels.max(1) as usize;
            // Planar Float (FLTP, AAC-Decoder): linker und rechter Kanal getrennt.
            let left = unsafe { std::slice::from_raw_parts(frame.data[0] as *const f32, n) };
            let right = if channels > 1 {
                unsafe { std::slice::from_raw_parts(frame.data[1] as *const f32, n) }
            } else {
                left
            };
            for i in skip..n {
                s.out.push(left[i]);
                s.out.push(right[i]);
            }
        }
    }

    while let Some(p) = input.read_packet()? {
        if let Some(s) = streams
            .iter_mut()
            .find(|s| s.index == p.stream_index as usize)
        {
            s.ctx.send_packet(Some(&p))?;
            drain(s)?;
        }
    }
    for s in &mut streams {
        s.ctx.send_packet(None)?;
        drain(s)?;
    }
    Ok(streams
        .into_iter()
        .map(|s| DecodedAudio {
            title: s.title,
            samples: s.out,
        })
        .collect())
}
