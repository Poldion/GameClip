// SPDX-License-Identifier: GPL-3.0-or-later
//! Rahmung: 4 Byte Länge (u32, Little Endian), danach UTF-8-JSON.

use std::io::{self, Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::version::MAX_MESSAGE_BYTES;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("Ein-/Ausgabefehler: {0}")]
    Io(#[from] io::Error),
    #[error("Nachricht mit {0} Byte ist größer als erlaubt")]
    TooLarge(usize),
    #[error("ungültiges JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Liest einen Rahmen. Ist er zu groß, muss der Aufrufer die Verbindung trennen.
pub fn read_frame(reader: &mut impl Read) -> Result<Vec<u8>, FrameError> {
    let mut len = [0u8; 4];
    reader.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_MESSAGE_BYTES {
        return Err(FrameError::TooLarge(len));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> Result<(), FrameError> {
    if payload.len() > MAX_MESSAGE_BYTES {
        return Err(FrameError::TooLarge(payload.len()));
    }
    writer.write_all(&(payload.len() as u32).to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(reader: &mut impl Read) -> Result<T, FrameError> {
    Ok(serde_json::from_slice(&read_frame(reader)?)?)
}

pub fn write_json<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), FrameError> {
    write_frame(writer, &serde_json::to_vec(value)?)
}
