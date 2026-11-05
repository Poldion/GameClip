// SPDX-License-Identifier: GPL-3.0-or-later
//! Bildzeiten für den Editor (Design, Datei 48): `GetFrameTimes` über
//! `gameclip-media::demux::frame_times_us`, die letzten 4 Clips im Speicher.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

const KEEP: usize = 4;

#[derive(Default)]
pub struct FrameCache {
    entries: VecDeque<(u32, PathBuf, Vec<u64>)>,
}

impl FrameCache {
    /// Bildzeiten des Clips (µs Anzeigezeit); gelesen, wenn nicht im Speicher.
    pub fn get(&mut self, clip_id: u32, path: &Path) -> gameclip_media::Result<Vec<u64>> {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|(id, p, _)| *id == clip_id && p == path)
        {
            let e = self.entries.remove(pos).expect("Position gültig");
            let times = e.2.clone();
            self.entries.push_front(e);
            return Ok(times);
        }
        let times = gameclip_media::demux::frame_times_us(path)?;
        self.entries
            .push_front((clip_id, path.to_path_buf(), times.clone()));
        self.entries.truncate(KEEP);
        Ok(times)
    }

    /// Nach Löschen oder Umschreiben eines Clips.
    pub fn forget(&mut self, clip_id: u32) {
        self.entries.retain(|(id, _, _)| *id != clip_id);
    }
}
