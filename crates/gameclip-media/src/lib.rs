// SPDX-License-Identifier: GPL-3.0-or-later
//! FFmpeg-Anbindung für Dienst und Worker (Design, Dateien 27–29).

#![cfg(windows)]

pub mod encoder;
pub mod hwdevice;

pub use encoder::{EncodeProfile, EncoderCandidate, EncoderConfig, VideoEncoder, probe};
pub use hwdevice::{HwDevice, HwFrame, HwFrames};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("FFmpeg: {0}")]
    Ffmpeg(#[from] rsmpeg::error::RsmpegError),
    #[error("Windows-Fehler: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("Encoder {0} ist in diesem FFmpeg-Build nicht enthalten")]
    EncoderMissing(String),
    #[error("Frame ohne D3D11-Textur")]
    NoTexture,
    #[error("Testsitzung fehlgeschlagen: {0}")]
    ProbeFailed(&'static str),
}

pub type Result<T> = std::result::Result<T, MediaError>;
