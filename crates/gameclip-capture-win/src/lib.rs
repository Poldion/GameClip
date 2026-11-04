// SPDX-License-Identifier: GPL-3.0-or-later
//! Aufnahme unter Windows (Design, Dateien 31–39). Bisher umgesetzt: der Videoweg
//! WGC → Taktgeber → VideoProcessor → Encoder und die Audio-Pipeline (System, Mikrofon,
//! Mischung) und der Ringpuffer. Discord folgt in M2.

#![cfg(windows)]

pub mod audio;
pub mod pacer;
pub mod ring;
pub mod video_pipeline;
pub mod wgc;

pub use audio::{AudioConfig, AudioEvent, AudioPacket, AudioPipeline, MicConfig, Track};
pub use video_pipeline::{
    KeyframeHandle, VideoConfig, VideoEvent, VideoPacket, VideoPipeline, VideoStats,
};

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("Windows-Fehler: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("GPU: {0}")]
    Gpu(#[from] gameclip_gpu::GpuError),
    #[error("Medien: {0}")]
    Media(#[from] gameclip_media::MediaError),
    #[error("kein Encoder nutzbar: {0}")]
    NoEncoder(String),
    #[error("Aufnahme-Thread beendet sich beim Start")]
    StartFailed,
}

pub type Result<T> = std::result::Result<T, CaptureError>;
