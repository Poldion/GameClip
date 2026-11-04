// SPDX-License-Identifier: GPL-3.0-or-later
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ErrorCode {
    NotReady,
    InvalidArgument,
    NotFound,
    Busy,
    HotkeyTaken,
    DiskFull,
    FolderMissing,
    BufferEmpty,
    EncoderFailed,
    Timeout,
    TooLarge,
    VersionMismatch,
    UnknownCommand,
    Internal,
}

impl From<gameclip_core::state::SaveError> for ErrorCode {
    fn from(e: gameclip_core::state::SaveError) -> Self {
        use gameclip_core::state::SaveError;
        match e {
            SaveError::DiskFull => ErrorCode::DiskFull,
            SaveError::FolderMissing => ErrorCode::FolderMissing,
            SaveError::BufferEmpty => ErrorCode::BufferEmpty,
            SaveError::EncoderFailed => ErrorCode::EncoderFailed,
            SaveError::Timeout => ErrorCode::Timeout,
        }
    }
}
