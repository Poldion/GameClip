// SPDX-License-Identifier: GPL-3.0-or-later
//! D3D11-Gerät, Adapterwahl und VideoProcessor für GameClip.
//!
//! Aufnahme (`gameclip-capture-win`) und Worker nutzen dieses Crate gemeinsam, damit der
//! Worker nicht vom Aufnahme-Crate abhängt (Design, Datei 26).

#![cfg(windows)]

pub mod convert;
pub mod device;
pub mod hdr;

pub use convert::{Converter, InFormat, TexRef};
pub use device::{GpuDevice, MonitorDesc, MonitorTarget, list_monitors};

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("Windows-Fehler: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("kein Adapter mit dem gewählten Monitor gefunden")]
    MonitorNotFound,
    #[error("ungültige Größe {0}×{1}")]
    InvalidSize(u32, u32),
}

pub type Result<T> = std::result::Result<T, GpuError>;
