// SPDX-License-Identifier: GPL-3.0-or-later
//! Audio-Pipeline: Quellen erfassen, auf eine Zeitachse legen, mischen und kodieren
//! (Design, „Audio-Pipeline“ und Dateien 35–37).
//!
//! Je Quelle läuft ein WASAPI-Thread. Ein Mixer-Thread arbeitet im 10-ms-Takt und liegt
//! `LATENCY_BLOCKS` hinter der Echtzeit, damit alle Quellen ihre Blöcke geliefert haben. Er
//! kodiert die Mischung (160 kbit/s, mit Lautstärke und Limiter) und jede Quelle unverändert als
//! Einzelspur (128 kbit/s). Zeitstempel sind QPC-µs wie beim Video.

pub mod loopback;
pub mod mic;
pub mod mixer;
pub mod resample;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, unbounded};
use gameclip_media::AudioEncoder;
use rsmpeg::avcodec::{AVCodecParameters, AVPacket};

use crate::pacer::{Pacer, qpc_us};
use crate::{CaptureError, Result};
use loopback::Chunk;
use mixer::{Block, Limiter, db_to_gain, mix, mute_window, rms_dbfs};
use resample::{BLOCK, CHANNELS, SourceTrack};

/// Abstand des Mixers zur Echtzeit (100 ms).
const LATENCY_BLOCKS: i64 = 10;
/// Pegel alle 100 ms.
const LEVEL_EVERY: i64 = 10;
const MIX_BPS: u64 = 160_000;
const TRACK_BPS: u64 = 128_000;
const BLOCK_US: i64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Track {
    Mix,
    System,
    Mic,
    Discord,
}

impl Track {
    /// Spurtitel in den Dateien (`handler_name`).
    pub fn title(self) -> &'static str {
        match self {
            Track::Mix => "Mischung",
            Track::System => "System",
            Track::Mic => "Mikrofon",
            Track::Discord => "Discord",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MicConfig {
    /// `None` = Windows-Standardmikrofon.
    pub device_id: Option<String>,
    pub gain_db: f32,
}

#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Prozessbaum, der aus „System“ ausgeschlossen wird (gameclipd bzw. Discord, Q11).
    pub exclude_pid: u32,
    pub system_gain_db: f32,
    pub mic: Option<MicConfig>,
}

pub struct AudioPacket {
    pub track: Track,
    pub packet: AVPacket,
    /// QPC-µs; das erste Paket je Spur liegt durch die AAC-Anlaufzeit vor dem Start.
    pub pts_us: i64,
}

pub enum AudioEvent {
    Packet(AudioPacket),
    /// Pegel des Blocks bei `time_us` in dBFS (−100 bei Stille).
    Levels {
        time_us: i64,
        system_dbfs: f32,
        mic_dbfs: Option<f32>,
    },
    /// Eine Quelle ist ausgefallen; ihre Spur liefert ab jetzt Stille.
    SourceFailed {
        track: Track,
        error: String,
    },
}

pub enum SourceMsg {
    Chunk(Chunk),
    Failed(Track, String),
}

pub struct AudioPipeline {
    stop: Arc<AtomicBool>,
    mic_paused: Arc<AtomicBool>,
    mutes: Arc<Mutex<Vec<(i64, i64)>>>,
    threads: Vec<JoinHandle<()>>,
    start_us: i64,
    params: Vec<(Track, AVCodecParameters)>,
}

impl AudioPipeline {
    pub fn start(cfg: AudioConfig, sink: Sender<AudioEvent>) -> Result<Self> {
        let start_us = qpc_us();
        let stop = Arc::new(AtomicBool::new(false));
        let mic_paused = Arc::new(AtomicBool::new(false));
        let mutes = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = unbounded();

        let mut encoders = vec![
            (Track::Mix, AudioEncoder::new(MIX_BPS)?),
            (Track::System, AudioEncoder::new(TRACK_BPS)?),
        ];
        if cfg.mic.is_some() {
            encoders.push((Track::Mic, AudioEncoder::new(TRACK_BPS)?));
        }
        let params = encoders
            .iter()
            .map(|(t, e)| (*t, e.codec_parameters()))
            .collect();

        let mut threads = Vec::new();
        let pid = cfg.exclude_pid;
        threads.push(spawn_source(
            Track::System,
            tx.clone(),
            stop.clone(),
            move || loopback::open_loopback(pid, false),
        )?);
        if let Some(m) = &cfg.mic {
            let id = m.device_id.clone();
            threads.push(spawn_source(
                Track::Mic,
                tx.clone(),
                stop.clone(),
                move || mic::open(id.as_deref()),
            )?);
        }
        drop(tx);

        let mixer = Mixer {
            start_us,
            system: SourceTrack::new(start_us),
            mic: cfg.mic.as_ref().map(|_| SourceTrack::new(start_us)),
            system_gain: db_to_gain(cfg.system_gain_db),
            mic_gain: cfg.mic.as_ref().map_or(1.0, |m| db_to_gain(m.gain_db)),
            limiter: Limiter::default(),
            prev: None,
            encoders,
        };
        let (stop_m, paused_m, mutes_m) = (stop.clone(), mic_paused.clone(), mutes.clone());
        threads.push(
            std::thread::Builder::new()
                .name("gameclip-audio-mix".into())
                .spawn(move || mixer.run(rx, sink, &stop_m, &paused_m, &mutes_m))
                .map_err(|_| CaptureError::StartFailed)?,
        );
        Ok(AudioPipeline {
            stop,
            mic_paused,
            mutes,
            threads,
            start_us,
            params,
        })
    }

    /// Nullpunkt der Zeitachse (QPC-µs).
    pub fn start_us(&self) -> i64 {
        self.start_us
    }

    /// Codec-Parameter je Spur für den Muxer.
    pub fn codec_parameters(&self) -> &[(Track, AVCodecParameters)] {
        &self.params
    }

    /// Mikrofon pausieren: die Spur bleibt, enthält aber Stille.
    pub fn set_mic_paused(&self, paused: bool) {
        self.mic_paused.store(paused, Ordering::Relaxed);
    }

    /// Schaltet „System“ im Zeitfenster stumm (eigener Bestätigungston, D26).
    pub fn mute_system(&self, from_us: i64, duration_us: i64) {
        if let Ok(mut m) = self.mutes.lock() {
            m.push((from_us, from_us + duration_us));
        }
    }

    /// Hält alle Threads an; die letzten Pakete gehen vorher noch an den Empfänger.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn spawn_source(
    track: Track,
    tx: Sender<SourceMsg>,
    stop: Arc<AtomicBool>,
    open: impl FnOnce() -> std::result::Result<wasapi::AudioClient, String> + Send + 'static,
) -> Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("gameclip-audio-{}", track.title()))
        .spawn(move || {
            let _ = wasapi::initialize_mta();
            let result = open().and_then(|c| loopback::run_capture(c, track, &tx, &stop));
            if let Err(e) = result {
                let _ = tx.send(SourceMsg::Failed(track, e));
            }
        })
        .map_err(|_| CaptureError::StartFailed)
}

struct Mixer {
    start_us: i64,
    system: SourceTrack,
    mic: Option<SourceTrack>,
    system_gain: f32,
    mic_gain: f32,
    limiter: Limiter,
    /// Einzelspuren des vorigen Blocks (eine Block Verzögerung wie der Limiter).
    prev: Option<(Block, Option<Block>)>,
    encoders: Vec<(Track, AudioEncoder)>,
}

impl Mixer {
    fn run(
        mut self,
        rx: Receiver<SourceMsg>,
        sink: Sender<AudioEvent>,
        stop: &AtomicBool,
        mic_paused: &AtomicBool,
        mutes: &Mutex<Vec<(i64, i64)>>,
    ) {
        let Ok(mut pacer) = Pacer::with_start(100, self.start_us) else {
            return;
        };
        let mut next_block = 0i64;
        while !stop.load(Ordering::Relaxed) {
            let Ok(tick) = pacer.wait() else {
                break;
            };
            for msg in rx.try_iter() {
                match msg {
                    SourceMsg::Chunk(c) => match c.track {
                        Track::System => self.system.push(c.qpc_us, &c.samples, c.silent),
                        Track::Mic => {
                            if let Some(m) = self.mic.as_mut() {
                                m.push(c.qpc_us, &c.samples, c.silent);
                            }
                        }
                        _ => {}
                    },
                    SourceMsg::Failed(track, error) => {
                        let _ = sink.send(AudioEvent::SourceFailed { track, error });
                    }
                }
            }
            let ready = tick.n as i64 - LATENCY_BLOCKS;
            while next_block <= ready {
                let windows = mutes.lock().map(|m| m.clone()).unwrap_or_default();
                if !self.block(
                    next_block,
                    &sink,
                    mic_paused.load(Ordering::Relaxed),
                    &windows,
                ) {
                    return;
                }
                next_block += 1;
            }
            if let Ok(mut m) = mutes.lock() {
                let done = self.start_us + next_block * BLOCK_US;
                m.retain(|(_, to)| *to > done);
            }
        }
        // Restliche Pakete der Encoder abholen.
        for (track, enc) in &mut self.encoders {
            if let Ok(packets) = enc.finish() {
                for p in packets {
                    let _ = sink.send(packet_event(*track, p, self.start_us));
                }
            }
        }
    }

    /// Verarbeitet Block `k`; `false`, wenn der Empfänger weg ist.
    fn block(
        &mut self,
        k: i64,
        sink: &Sender<AudioEvent>,
        mic_paused: bool,
        mutes: &[(i64, i64)],
    ) -> bool {
        let block_us = self.start_us + k * BLOCK_US;
        let mut system = [0.0f32; BLOCK * CHANNELS];
        self.system.pull(k, &mut system);
        for (from, to) in mutes {
            mute_window(&mut system, block_us, *from, *to);
        }
        let mic = self.mic.as_mut().map(|m| {
            let mut b = [0.0f32; BLOCK * CHANNELS];
            m.pull(k, &mut b);
            if mic_paused {
                b.fill(0.0);
            }
            b
        });

        if k % LEVEL_EVERY == 0 {
            let _ = sink.send(AudioEvent::Levels {
                time_us: block_us,
                system_dbfs: rms_dbfs(&system),
                mic_dbfs: mic.as_ref().map(rms_dbfs),
            });
        }

        let mut sources: Vec<(&Block, f32)> = vec![(&system, self.system_gain)];
        if let Some(m) = &mic {
            sources.push((m, self.mic_gain));
        }
        let mixed = mix(&sources);
        let limited = self.limiter.process(&mixed);
        let prev = self.prev.replace((system, mic));
        let (Some(mix_block), Some((sys_prev, mic_prev))) = (limited, prev) else {
            return true;
        };
        for (track, enc) in &mut self.encoders {
            let input: &Block = match track {
                Track::Mix => &mix_block,
                Track::System => &sys_prev,
                Track::Mic => match &mic_prev {
                    Some(b) => b,
                    None => continue,
                },
                Track::Discord => continue,
            };
            let Ok(packets) = enc.push(input) else {
                continue;
            };
            for p in packets {
                if sink.send(packet_event(*track, p, self.start_us)).is_err() {
                    return false;
                }
            }
        }
        true
    }
}

fn packet_event(track: Track, packet: AVPacket, start_us: i64) -> AudioEvent {
    let pts_us = start_us + packet.pts * 1_000_000 / i64::from(resample::RATE as i32);
    AudioEvent::Packet(AudioPacket {
        track,
        packet,
        pts_us,
    })
}
