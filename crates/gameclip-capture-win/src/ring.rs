// SPDX-License-Identifier: GPL-3.0-or-later
//! Ringpuffer aus kodierten Paketen (Design, „Ringpuffer“ und Datei 38).
//!
//! * **Speicher:** Pool aus Blöcken zu `BLOCK_BYTES`, beim Anlegen belegt (jede Seite wird
//!   einmal beschrieben, damit Windows sie wirklich zuteilt). Ein Paket, das nicht mehr in den
//!   Rest des aktuellen Blocks passt, beginnt im nächsten freien Block. Pakete größer als ein
//!   Block bekommen eine eigene Zuteilung außerhalb des Pools (gezählt).
//! * **Verwerfen:** nach Zeit (Pufferlänge) und nach Platz, immer die älteste ganze
//!   Video-Bildgruppe; Ton vor dem neuen ältesten Keyframe fällt mit weg.
//! * **Schnappschuss:** hält seine Blöcke per Referenzzählung (`Arc`); ein Block wird erst
//!   wiederverwendet, wenn weder Index noch Schnappschuss ihn halten.
//! * **Pool erschöpft:** Video bis zum nächsten Keyframe verwerfen, Ton im selben Zeitraum
//!   ebenfalls, IDR anfordern (`PushOutcome::NeedKeyframe`), erstes Paket danach mit
//!   `gap_before`. Ein Schnappschuss über die Lücke wird an ihr getrennt.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use gameclip_core::forecast::{BLOCK_BYTES, Forecast};

use crate::audio::Track;

const PAGE: usize = 4096;
/// Dauer eines AAC-Pakets in µs (1024 Samples zu 48 kHz), aufgerundet.
const AUDIO_PACKET_US: i64 = 21_334;

/// Spur im Ringpuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stream {
    Video,
    Audio(Track),
}

/// Speicherblock. Genau ein Schreiber (unter dem Mutex des Rings) schreibt nur hinter den
/// bereits veröffentlichten Bereich; Leser lesen nur veröffentlichte Bereiche. Deshalb Rohzeiger
/// statt `&mut`-Referenzen, damit sich Schreib- und Lesezugriffe nie überlappen.
pub struct BlockBuf {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: siehe oben; Lese- und Schreibbereiche sind disjunkt, Wiederverwendung eines Blocks
// erfolgt nur, wenn keine andere Referenz mehr existiert (`Arc::strong_count == 1`, geprüft unter
// dem Mutex, unter dem auch alle Klone entstehen).
unsafe impl Send for BlockBuf {}
unsafe impl Sync for BlockBuf {}

impl BlockBuf {
    fn new(len: usize) -> Self {
        let mut v = vec![0u8; len].into_boxed_slice();
        // Jede Seite einmal beschreiben: belegt den Speicher sofort (committed).
        for i in (0..len).step_by(PAGE) {
            v[i] = 1;
        }
        let ptr = Box::into_raw(v) as *mut u8;
        BlockBuf { ptr, len }
    }

    /// Schreibt `data` ab `offset`. Nur der Ring ruft das auf, für einen unveröffentlichten
    /// Bereich.
    fn write(&self, offset: usize, data: &[u8]) {
        assert!(offset + data.len() <= self.len);
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len()) };
    }

    fn read(&self, offset: usize, len: usize) -> &[u8] {
        assert!(offset + len <= self.len);
        unsafe { std::slice::from_raw_parts(self.ptr.add(offset), len) }
    }
}

impl Drop for BlockBuf {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                self.ptr, self.len,
            )))
        };
    }
}

/// Eintrag im Paketindex.
#[derive(Clone)]
pub struct PacketRef {
    pub stream: Stream,
    pub pts_us: i64,
    pub dts_us: i64,
    pub keyframe: bool,
    /// Vor diesem Paket wurde wegen erschöpften Pools verworfen.
    pub gap_before: bool,
    pub section: u32,
    block: Arc<BlockBuf>,
    offset: usize,
    len: usize,
}

impl PacketRef {
    pub fn data(&self) -> &[u8] {
        self.block.read(self.offset, self.len)
    }

    /// FFmpeg-Paket ohne Kopie: der `AVBufferRef` hält eine `Arc` auf den Block, bis FFmpeg
    /// ihn freigibt. Zeitstempel in µs (QPC); der Muxer verschiebt sie um t0.
    pub fn to_avpacket(&self) -> rsmpeg::avcodec::AVPacket {
        use rsmpeg::UnsafeDerefMut;
        use rsmpeg::ffi;

        unsafe extern "C" fn release(opaque: *mut std::ffi::c_void, _data: *mut u8) {
            drop(unsafe { Box::from_raw(opaque as *mut Arc<BlockBuf>) });
        }
        let holder = Box::into_raw(Box::new(self.block.clone()));
        let data = unsafe { self.block.ptr.add(self.offset) };
        let buf = unsafe {
            ffi::av_buffer_create(
                data,
                self.len,
                Some(release),
                holder as *mut std::ffi::c_void,
                ffi::AV_BUFFER_FLAG_READONLY as i32,
            )
        };
        assert!(!buf.is_null(), "av_buffer_create: kein Speicher");
        let mut p = rsmpeg::avcodec::AVPacket::new();
        unsafe {
            let raw = p.deref_mut();
            raw.buf = buf;
            raw.data = data;
            raw.size = self.len as i32;
            raw.pts = self.pts_us;
            raw.dts = self.dts_us;
            if self.keyframe {
                raw.flags |= ffi::AV_PKT_FLAG_KEY as i32;
            }
        }
        p
    }
}

/// Ergebnis eines Schnappschusses: Pakete je Spur in Ablageordnung.
pub struct Snapshot {
    /// PTS des ersten Keyframes (gemeinsamer Nullpunkt, Design „Speichern per Hotkey“ 6).
    pub t0_us: i64,
    pub end_us: i64,
    /// Der Clip beginnt später als verlangt (Abschnittswechsel oder Lücke).
    pub truncated: bool,
    pub video: Vec<PacketRef>,
    pub audio: Vec<(Track, Vec<PacketRef>)>,
}

impl Snapshot {
    pub fn is_empty(&self) -> bool {
        self.video.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    Stored,
    /// Verworfen (Pool erschöpft, warte auf Keyframe).
    Dropped,
    /// Pool gerade erschöpft: der Aufrufer muss ein IDR anfordern.
    NeedKeyframe,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RingStats {
    pub packets: usize,
    pub pool_blocks: usize,
    pub free_blocks: usize,
    pub oversize_packets: u64,
    pub dropped_packets: u64,
    pub gaps: u64,
}

pub struct RingConfig {
    pub pool_blocks: usize,
    pub duration_us: i64,
    pub block_bytes: usize,
}

impl RingConfig {
    /// Poolgröße aus Prognose plus Reserve (Design „Prognose“).
    pub fn from_forecast(f: &Forecast, buffer_seconds: u32) -> Self {
        RingConfig {
            pool_blocks: ((f.buffer_bytes + f.reserve_bytes) / BLOCK_BYTES) as usize,
            duration_us: i64::from(buffer_seconds) * 1_000_000,
            block_bytes: BLOCK_BYTES as usize,
        }
    }
}

struct State {
    duration_us: i64,
    block_bytes: usize,
    pool: Vec<Arc<BlockBuf>>,
    current: Option<(usize, usize)>,
    index: VecDeque<PacketRef>,
    section: u32,
    dropping: bool,
    pending_gap: bool,
    newest_video_us: Option<i64>,
    stats: RingStats,
}

pub struct Ring {
    state: Mutex<State>,
    arrived: Condvar,
}

/// Was der Speichervorgang vom Puffer braucht; Tests setzen eine Attrappe ein.
pub trait PacketSource {
    /// Wartet, bis ein Videopaket mit PTS ≥ `pts_us` im Puffer liegt.
    fn wait_for(&self, pts_us: i64, timeout: Duration) -> bool;
    fn snapshot(&self, from_us: i64, to_us: i64) -> Snapshot;
}

impl Ring {
    pub fn new(cfg: RingConfig) -> Self {
        let pool = (0..cfg.pool_blocks)
            .map(|_| Arc::new(BlockBuf::new(cfg.block_bytes)))
            .collect();
        Ring {
            state: Mutex::new(State {
                duration_us: cfg.duration_us,
                block_bytes: cfg.block_bytes,
                pool,
                current: None,
                index: VecDeque::new(),
                section: 0,
                dropping: false,
                pending_gap: false,
                newest_video_us: None,
                stats: RingStats {
                    pool_blocks: cfg.pool_blocks,
                    ..RingStats::default()
                },
            }),
            arrived: Condvar::new(),
        }
    }

    /// Neuer Abschnitt (Geräteverlust, Neustart): ältere Daten bleiben, ein Schnappschuss
    /// nimmt aber nur den neuesten Abschnitt.
    pub fn new_section(&self) {
        if let Ok(mut s) = self.state.lock() {
            s.section += 1;
            s.dropping = false;
            s.pending_gap = false;
        }
    }

    /// Gepufferte Dauer im neuesten Abschnitt (ältester Keyframe bis neuestes Bild), für den Status.
    pub fn buffered_us(&self) -> u64 {
        let s = self.state.lock().expect("Ring-Mutex");
        let section = s.section;
        let oldest = s
            .index
            .iter()
            .find(|p| p.stream == Stream::Video && p.keyframe && p.section == section)
            .map(|p| p.pts_us);
        match (oldest, s.newest_video_us) {
            (Some(a), Some(b)) if b > a => (b - a) as u64,
            _ => 0,
        }
    }

    pub fn stats(&self) -> RingStats {
        let mut s = self.state.lock().expect("Ring-Mutex");
        s.stats.packets = s.index.len();
        s.stats.free_blocks = s.pool.iter().filter(|b| Arc::strong_count(b) == 1).count();
        s.stats
    }

    pub fn push(
        &self,
        stream: Stream,
        pts_us: i64,
        dts_us: i64,
        keyframe: bool,
        data: &[u8],
    ) -> PushOutcome {
        let mut s = self.state.lock().expect("Ring-Mutex");
        let is_video = stream == Stream::Video;
        if s.dropping && !(is_video && keyframe) {
            s.stats.dropped_packets += 1;
            return PushOutcome::Dropped;
        }
        if is_video {
            s.newest_video_us = Some(s.newest_video_us.map_or(pts_us, |n| n.max(pts_us)));
            s.evict_by_time();
        }
        // Ein Keyframe beginnt eine neue Bildgruppe und darf dafür auch die letzte alte verdrängen;
        // sonst käme die Aufnahme nach einer Lücke nie wieder in Gang.
        let Some((block, offset)) = s.allocate(data.len(), is_video && keyframe) else {
            s.stats.dropped_packets += 1;
            let first = !s.dropping;
            s.dropping = true;
            s.pending_gap = true;
            if first {
                s.stats.gaps += 1;
                return PushOutcome::NeedKeyframe;
            }
            return PushOutcome::Dropped;
        };
        s.dropping = false;
        block.write(offset, data);
        let gap_before = is_video && std::mem::take(&mut s.pending_gap);
        let section = s.section;
        s.index.push_back(PacketRef {
            stream,
            pts_us,
            dts_us,
            keyframe,
            gap_before,
            section,
            block,
            offset,
            len: data.len(),
        });
        drop(s);
        if is_video {
            self.arrived.notify_all();
        }
        PushOutcome::Stored
    }
}

impl State {
    /// Älteste Bildgruppe verwerfen; `false`, wenn nur noch eine (oder keine) übrig ist.
    fn evict_gop(&mut self) -> bool {
        let mut keys = self
            .index
            .iter()
            .filter(|p| p.stream == Stream::Video && p.keyframe);
        let (Some(_), Some(second)) = (keys.next(), keys.next()) else {
            return false;
        };
        let (new_first_us, new_first_section) = (second.pts_us, second.section);
        self.drop_before(new_first_us, new_first_section);
        true
    }

    /// Verwirft alles (auch die letzte Bildgruppe); nur für einen neuen Keyframe.
    fn evict_all(&mut self) -> bool {
        if self.index.is_empty() {
            return false;
        }
        self.index.clear();
        true
    }

    fn drop_before(&mut self, new_first_us: i64, new_first_section: u32) {
        self.index.retain(|p| {
            let older_section = p.section < new_first_section;
            match p.stream {
                Stream::Video => {
                    !(older_section || p.section == new_first_section && p.pts_us < new_first_us)
                }
                // Ton vor dem neuen ältesten Keyframe fällt mit weg.
                Stream::Audio(_) => !(older_section || p.pts_us + AUDIO_PACKET_US <= new_first_us),
            }
        });
    }

    fn evict_by_time(&mut self) {
        let Some(newest) = self.newest_video_us else {
            return;
        };
        let limit = newest - self.duration_us;
        loop {
            let mut keys = self
                .index
                .iter()
                .filter(|p| p.stream == Stream::Video && p.keyframe);
            // Die Bildgruppe, die die Grenze enthält, bleibt: verwerfen nur, wenn schon der
            // zweite Keyframe vor der Grenze liegt.
            match (keys.next(), keys.next()) {
                (Some(_), Some(second)) if second.pts_us <= limit => {}
                _ => return,
            }
            if !self.evict_gop() {
                return;
            }
        }
    }

    /// Platz für `len` Bytes: im aktuellen Block, im nächsten freien Block, nach Verwerfen
    /// alter Bildgruppen oder außerhalb des Pools (größer als ein Block).
    fn allocate(&mut self, len: usize, may_evict_all: bool) -> Option<(Arc<BlockBuf>, usize)> {
        if len > self.block_bytes {
            self.stats.oversize_packets += 1;
            return Some((Arc::new(BlockBuf::new(len)), 0));
        }
        if let Some((i, fill)) = self.current
            && fill + len <= self.block_bytes
        {
            self.current = Some((i, fill + len));
            return Some((self.pool[i].clone(), fill));
        }
        loop {
            let current = self.current.map(|c| c.0);
            if let Some(i) = (0..self.pool.len())
                .find(|&i| Some(i) != current && Arc::strong_count(&self.pool[i]) == 1)
            {
                self.current = Some((i, len));
                return Some((self.pool[i].clone(), 0));
            }
            if !self.evict_gop() && !(may_evict_all && self.evict_all()) {
                return None;
            }
        }
    }
}

impl PacketSource for Ring {
    fn wait_for(&self, pts_us: i64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut s = self.state.lock().expect("Ring-Mutex");
        loop {
            if s.newest_video_us.is_some_and(|n| n >= pts_us) {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            s = self
                .arrived
                .wait_timeout(s, deadline - now)
                .expect("Ring-Mutex")
                .0;
        }
    }

    fn snapshot(&self, from_us: i64, to_us: i64) -> Snapshot {
        let s = self.state.lock().expect("Ring-Mutex");
        let section = s.section;
        let video: Vec<&PacketRef> = s
            .index
            .iter()
            .filter(|p| p.stream == Stream::Video && p.section == section && p.pts_us <= to_us)
            .collect();
        // Start: letzter Keyframe ≤ from_us, sonst der früheste vorhandene; eine Lücke dahinter
        // schneidet wie ein Abschnittswechsel.
        let keys: Vec<usize> = (0..video.len()).filter(|&i| video[i].keyframe).collect();
        let mut start = keys
            .iter()
            .rev()
            .find(|&&i| video[i].pts_us <= from_us)
            .or(keys.first())
            .copied();
        let mut cut_at_gap = false;
        if let Some(st) = start
            && let Some(gap) = (st + 1..video.len()).rev().find(|&i| video[i].gap_before)
        {
            start = Some(gap);
            cut_at_gap = true;
        }
        let Some(start) = start else {
            return Snapshot {
                t0_us: from_us,
                end_us: to_us,
                truncated: true,
                video: Vec::new(),
                audio: Vec::new(),
            };
        };
        let t0 = video[start].pts_us;
        let video: Vec<PacketRef> = video[start..].iter().map(|p| (*p).clone()).collect();
        let end = video.last().map_or(t0, |p| p.pts_us);

        let mut audio: Vec<(Track, Vec<PacketRef>)> = Vec::new();
        for p in s.index.iter().filter(|p| p.section == section) {
            let Stream::Audio(track) = p.stream else {
                continue;
            };
            if p.pts_us + AUDIO_PACKET_US <= t0 || p.pts_us > end {
                continue;
            }
            match audio.iter_mut().find(|(t, _)| *t == track) {
                Some((_, list)) => list.push(p.clone()),
                None => audio.push((track, vec![p.clone()])),
            }
        }
        Snapshot {
            t0_us: t0,
            end_us: end,
            // Verkürzt nur durch Lücke oder Abschnittswechsel; ein noch nicht voller Puffer ergibt
            // einfach einen kürzeren Clip ohne Hinweis (Design, „Speichern per Hotkey“ 3).
            truncated: t0 > from_us
                && (cut_at_gap
                    || video[0].gap_before
                    || s.index.iter().any(|p| p.section < section)),
            video,
            audio,
        }
    }
}
