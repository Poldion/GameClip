// SPDX-License-Identifier: GPL-3.0-or-later
//! Auftrag an `gameclip-worker` (JSON über stdin) und seine Meldungen (JSON-Zeilen über stdout).
//!
//! Nur zwischen Dienst und Worker; diese Typen gehen nicht an die Oberfläche.

use serde::{Deserialize, Serialize};

use crate::messages::ExportProfile;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerJob {
    pub source: String,
    pub tracks_path: Option<String>,
    pub target_part_path: String,
    /// Neue `gameclip_id` der Ausgabedatei.
    pub output_tag_id: String,
    pub in_us: u64,
    pub out_us: u64,
    pub profile: ExportProfile,
    pub encoder_order: Vec<String>,
    pub adapter_luid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerError {
    InputMissing,
    DecodeFailed,
    EncoderFailed,
    TooLarge,
    Io,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerMessage {
    Progress { percent: f32 },
    Encoder { name: String },
    Done { bytes: u64, duration_us: u64 },
    Error { code: WorkerError, message: String },
}
