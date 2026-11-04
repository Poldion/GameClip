// SPDX-License-Identifier: GPL-3.0-or-later
//! FFmpeg-Anbindung für Dienst und Worker (Design, Dateien 27–29).

#![cfg(windows)]

pub mod audio_codec;
pub mod decode;
pub mod demux;
pub mod encoder;
pub mod hwdevice;
pub mod mux;

pub use audio_codec::AudioEncoder;
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
    #[error("Pfad oder Text mit NUL bzw. nicht UTF-8: {0}")]
    InvalidPath(String),
    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("Datei ohne Videospur")]
    NoVideoStream,
    #[error("Pixelformat {0} lässt sich nicht nach NV12 umwandeln")]
    UnsupportedFormat(i32),
}

pub type Result<T> = std::result::Result<T, MediaError>;
