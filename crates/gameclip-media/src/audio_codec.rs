// SPDX-License-Identifier: GPL-3.0-or-later
//! AAC-LC-Encoder: 48 kHz Stereo, Eingabe interleaved Float in beliebig großen Blöcken.
//!
//! Die Samples werden je Kanal gepuffert (planar, FLTP) und in Blöcken zu 1024 Samples an den
//! Encoder gegeben. Zeitbasis ist 1/48 000, PTS zählt also Samples.

use rsmpeg::avcodec::{AVCodec, AVCodecContext, AVCodecParameters, AVPacket};
use rsmpeg::avutil::{AVChannelLayout, AVFrame};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;

use crate::{MediaError, Result};

pub const SAMPLE_RATE: i32 = 48_000;
pub const CHANNELS: usize = 2;

pub struct AudioEncoder {
    ctx: AVCodecContext,
    frame_size: usize,
    /// Noch nicht kodierte Samples je Kanal.
    pending: [Vec<f32>; CHANNELS],
    /// PTS des nächsten Blocks in Samples.
    next_pts: i64,
}

impl AudioEncoder {
    pub fn new(bitrate_bps: u64) -> Result<Self> {
        let codec = AVCodec::find_encoder_by_name(c"aac")
            .ok_or_else(|| MediaError::EncoderMissing("aac".into()))?;
        let mut ctx = AVCodecContext::new(&codec);
        ctx.set_sample_rate(SAMPLE_RATE);
        ctx.set_sample_fmt(ffi::AV_SAMPLE_FMT_FLTP);
        ctx.set_ch_layout(AVChannelLayout::from_nb_channels(CHANNELS as i32).into_inner());
        ctx.set_bit_rate(bitrate_bps as i64);
        ctx.set_time_base(ffi::AVRational {
            num: 1,
            den: SAMPLE_RATE,
        });
        ctx.open(None)?;
        let frame_size = ctx.frame_size as usize;
        Ok(AudioEncoder {
            ctx,
            frame_size,
            pending: [Vec::new(), Vec::new()],
            next_pts: 0,
        })
    }

    /// Samples je Block (1024 bei AAC-LC).
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }

    /// Anlaufverzögerung des Encoders in Samples; der Muxer braucht sie für die Edit-List.
    pub fn initial_padding(&self) -> i64 {
        i64::from(self.ctx.initial_padding)
    }

    pub fn codec_parameters(&self) -> AVCodecParameters {
        self.ctx.extract_codecpar()
    }

    /// Nimmt interleaved Stereo-Samples (L, R, L, R, …) an und liefert alle fertigen Pakete.
    pub fn push(&mut self, interleaved: &[f32]) -> Result<Vec<AVPacket>> {
        for pair in interleaved.as_chunks::<CHANNELS>().0 {
            for (ch, &s) in pair.iter().enumerate() {
                self.pending[ch].push(s);
            }
        }
        let mut packets = Vec::new();
        while self.pending[0].len() >= self.frame_size {
            self.encode_block(self.frame_size, &mut packets)?;
        }
        Ok(packets)
    }

    /// Kodiert den Rest (kürzerer letzter Block) und leert den Encoder.
    pub fn finish(&mut self) -> Result<Vec<AVPacket>> {
        let mut packets = Vec::new();
        let rest = self.pending[0].len();
        if rest > 0 {
            self.encode_block(rest, &mut packets)?;
        }
        self.ctx.send_frame(None)?;
        self.collect(&mut packets)?;
        Ok(packets)
    }

    fn encode_block(&mut self, samples: usize, packets: &mut Vec<AVPacket>) -> Result<()> {
        let mut frame = AVFrame::new();
        frame.set_nb_samples(samples as i32);
        frame.set_format(ffi::AV_SAMPLE_FMT_FLTP);
        frame.set_sample_rate(SAMPLE_RATE);
        frame.set_ch_layout(AVChannelLayout::from_nb_channels(CHANNELS as i32).into_inner());
        frame.get_buffer(0)?;
        for (ch, buf) in self.pending.iter_mut().enumerate() {
            let plane =
                unsafe { std::slice::from_raw_parts_mut(frame.data[ch] as *mut f32, samples) };
            plane.copy_from_slice(&buf[..samples]);
            buf.drain(..samples);
        }
        frame.set_pts(self.next_pts);
        self.next_pts += samples as i64;
        self.ctx.send_frame(Some(&frame))?;
        self.collect(packets)
    }

    fn collect(&mut self, packets: &mut Vec<AVPacket>) -> Result<()> {
        loop {
            match self.ctx.receive_packet() {
                Ok(p) => packets.push(p),
                Err(RsmpegError::EncoderDrainError | RsmpegError::EncoderFlushedError) => {
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}
