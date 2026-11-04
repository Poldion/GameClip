// SPDX-License-Identifier: GPL-3.0-or-later
//! Lesen von Bildzeiten und Tags aus fertigen Dateien (Design, Datei 29).

use std::collections::BTreeMap;
use std::ffi::CString;
use std::path::Path;

use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::ffi;

use crate::{MediaError, Result};

const MICROS: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 1_000_000,
};

/// Tags der Datei (Format-Metadaten), z. B. `gameclip_tag_id`.
pub type Tags = BTreeMap<String, String>;

pub(crate) fn open_input(path: &Path) -> Result<AVFormatContextInput> {
    let url = path
        .to_str()
        .and_then(|s| CString::new(s).ok())
        .ok_or_else(|| MediaError::InvalidPath(path.display().to_string()))?;
    Ok(AVFormatContextInput::open(&url)?)
}

/// Anzeigezeiten aller Bilder der Videospur in µs, aufsteigend. Pakete, die die Edit-List
/// ausblendet (`AV_PKT_FLAG_DISCARD`), fehlen; die Zeiten beginnen beim Startzeitpunkt der Spur.
pub fn frame_times_us(path: &Path) -> Result<Vec<u64>> {
    let mut input = open_input(path)?;
    let (index, _) = input
        .find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)?
        .ok_or(MediaError::NoVideoStream)?;
    let stream = &input.streams()[index];
    let tb = stream.time_base;
    let start = if stream.start_time == ffi::AV_NOPTS_VALUE {
        0
    } else {
        stream.start_time
    };
    let mut times = Vec::new();
    while let Some(p) = input.read_packet()? {
        if p.stream_index as usize != index || p.flags & ffi::AV_PKT_FLAG_DISCARD as i32 != 0 {
            continue;
        }
        let pts = if p.pts == ffi::AV_NOPTS_VALUE {
            p.dts
        } else {
            p.pts
        };
        let us = unsafe { ffi::av_rescale_q(pts - start, tb, MICROS) };
        if us >= 0 {
            times.push(us as u64);
        }
    }
    times.sort_unstable();
    Ok(times)
}

/// Eckdaten der Videospur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    /// Gerundete mittlere Bildrate.
    pub fps: u32,
    pub duration_us: u64,
    /// Bitrate der Videospur laut Datei (0, wenn unbekannt).
    pub bit_rate: u64,
}

pub fn video_info(path: &Path) -> Result<VideoInfo> {
    let input = open_input(path)?;
    let (index, _) = input
        .find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)?
        .ok_or(MediaError::NoVideoStream)?;
    let s = &input.streams()[index];
    let par = s.codecpar();
    let rate = s.avg_frame_rate;
    let fps = if rate.den > 0 && rate.num > 0 {
        (f64::from(rate.num) / f64::from(rate.den)).round() as u32
    } else {
        60
    };
    let duration_us = if s.duration > 0 {
        unsafe { ffi::av_rescale_q(s.duration, s.time_base, MICROS) as u64 }
    } else if input.duration > 0 {
        input.duration as u64
    } else {
        0
    };
    Ok(VideoInfo {
        width: par.width as u32,
        height: par.height as u32,
        fps,
        duration_us,
        bit_rate: par.bit_rate.max(0) as u64,
    })
}

/// Format-Metadaten der Datei.
pub fn read_tags(path: &Path) -> Result<Tags> {
    let input = open_input(path)?;
    Ok(input
        .metadata()
        .map(|d| {
            d.iter()
                .map(|e| {
                    (
                        e.key().to_string_lossy().into_owned(),
                        e.value().to_string_lossy().into_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default())
}
