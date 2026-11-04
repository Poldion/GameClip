// SPDX-License-Identifier: GPL-3.0-or-later
//! Videoweg: WGC → Taktgeber → VideoProcessor → Encoder (Design, Datei 34).
//!
//! Ein eigener Thread richtet Gerät, Aufnahme und Encoder ein und läuft dann im festen Takt.
//! Je Takt nimmt er das neueste Bild (oder wiederholt das letzte), wandelt es in NV12 der
//! Ausgabegröße und kodiert es. Zeitstempel sind QPC-Zeiten in µs (Taktzeit, nicht
//! Aufnahmezeit), damit der Videostrom lückenlos im festen Raster liegt.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, bounded};
use gameclip_gpu::{Converter, GpuDevice, InFormat, MonitorTarget, TexRef, hdr};
use gameclip_media::encoder::is_hardware;
use gameclip_media::hwdevice::{download_nv12, nv12_texture};
use gameclip_media::{EncodeProfile, EncoderConfig, HwDevice, HwFrames, VideoEncoder, probe};
use rsmpeg::avcodec::{AVCodecParameters, AVPacket};
use rsmpeg::ffi;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

use crate::pacer::Pacer;
use crate::wgc::WgcCapture;
use crate::{CaptureError, Result};

#[derive(Debug, Clone, Copy)]
pub struct VideoConfig {
    pub monitor: MonitorTarget,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_bps: u64,
    pub cursor: bool,
}

// SAFETY: `MonitorTarget::Handle` enthält ein HMONITOR; das ist ein Kennwert, kein Zeiger, auf
// den zugegriffen wird.
struct SendConfig(VideoConfig);
unsafe impl Send for SendConfig {}

/// Ein kodiertes Videopaket; `pts_us`/`dts_us` in QPC-µs.
pub struct VideoPacket {
    pub packet: AVPacket,
    pub pts_us: i64,
    pub dts_us: i64,
    pub keyframe: bool,
}

pub enum VideoEvent {
    Packet(VideoPacket),
    /// Gerät verloren oder Fehler im Takt; der Thread hat sich beendet.
    DeviceLost(String),
    HdrChanged(bool),
    /// Die Quelle hat die Größe geändert; nur die Eingabe des VideoProcessors ändert sich.
    FormatChanged {
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VideoStats {
    pub ticks: u64,
    /// Kodierte Bilder (Takte vor dem ersten WGC-Bild liefern keins).
    pub encoded: u64,
    /// Takte ohne neues Bild (letztes Bild wiederholt).
    pub repeated: u64,
    /// Takte, die der Thread verpasst hat.
    pub skipped_ticks: u64,
    /// Von WGC geliefert bzw. vor dem Abholen überschrieben.
    pub captured: u64,
    pub overwritten: u64,
}

/// Was beim Start feststeht.
pub struct StartInfo {
    pub encoder: String,
    pub hardware: bool,
    pub params: AVCodecParameters,
    pub border_removed: bool,
    pub hdr_active: bool,
    pub adapter: String,
}

/// Fordert beim nächsten Bild ein IDR an.
#[derive(Clone)]
pub struct KeyframeHandle(Arc<AtomicBool>);

impl KeyframeHandle {
    pub fn force(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

pub struct VideoPipeline {
    stop: Arc<AtomicBool>,
    force_kf: Arc<AtomicBool>,
    stats: Arc<Mutex<VideoStats>>,
    thread: Option<JoinHandle<()>>,
    info: StartInfo,
}

impl VideoPipeline {
    /// Startet den Aufnahme-Thread und wartet, bis Aufnahme und Encoder bereit sind.
    pub fn start(cfg: VideoConfig, sink: Sender<VideoEvent>) -> Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let force_kf = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(Mutex::new(VideoStats::default()));
        let (ready_tx, ready_rx) = bounded(1);
        let thread = {
            let (stop, force_kf, stats) = (stop.clone(), force_kf.clone(), stats.clone());
            let cfg = SendConfig(cfg);
            std::thread::Builder::new()
                .name("gameclip-video".into())
                .spawn(move || {
                    let cfg = cfg;
                    run(cfg.0, sink, stop, force_kf, stats, ready_tx)
                })
                .map_err(|_| CaptureError::StartFailed)?
        };
        let info = ready_rx.recv().map_err(|_| CaptureError::StartFailed)??;
        Ok(VideoPipeline {
            stop,
            force_kf,
            stats,
            thread: Some(thread),
            info,
        })
    }

    pub fn info(&self) -> &StartInfo {
        &self.info
    }

    /// Das nächste kodierte Bild wird ein IDR (für Schnappschüsse und nach Lücken).
    pub fn force_keyframe(&self) {
        self.force_kf.store(true, Ordering::Relaxed);
    }

    /// Griff zum Anfordern eines IDR von einem anderen Thread (Pump in den Ringpuffer).
    pub fn keyframe_handle(&self) -> KeyframeHandle {
        KeyframeHandle(self.force_kf.clone())
    }

    pub fn stats(&self) -> VideoStats {
        self.stats.lock().map(|s| *s).unwrap_or_default()
    }

    /// Hält den Thread an; restliche Pakete gehen vorher noch an den Empfänger.
    pub fn stop(mut self) -> VideoStats {
        self.shutdown();
        self.stats()
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for VideoPipeline {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Zustand des Aufnahme-Threads.
struct Pipeline {
    cfg: VideoConfig,
    gpu: GpuDevice,
    capture: WgcCapture,
    encoder: VideoEncoder,
    /// Hardware-Weg: Pool des Encoders. Software-Weg: `None`.
    frames: Option<HwFrames>,
    /// Software-Weg: eigenes NV12-Ziel und Staging-Textur zum Zurücklesen.
    sw: Option<(ID3D11Texture2D, ID3D11Texture2D)>,
    converter: Option<Converter>,
    hdr: bool,
}

fn setup(cfg: &VideoConfig) -> Result<(Pipeline, StartInfo)> {
    let gpu = GpuDevice::for_monitor(cfg.monitor)?;
    let enc_cfg = EncoderConfig {
        width: cfg.width,
        height: cfg.height,
        fps: cfg.fps,
        bitrate_bps: cfg.bitrate_bps,
        profile: EncodeProfile::Capture,
    };
    let candidates = probe(&gpu, &enc_cfg);
    let Some(name) = candidates.iter().find(|c| c.ok).map(|c| c.name.clone()) else {
        let reasons: Vec<String> = candidates
            .iter()
            .map(|c| format!("{}: {}", c.name, c.reason.as_deref().unwrap_or("?")))
            .collect();
        return Err(CaptureError::NoEncoder(reasons.join("; ")));
    };
    let hardware = is_hardware(&name);
    let (encoder, frames, sw) = if hardware {
        let device = HwDevice::from_gpu(&gpu)?;
        let frames = HwFrames::new(&device, cfg.width, cfg.height)?;
        let encoder = VideoEncoder::open(&name, &enc_cfg, Some(&frames))?;
        (encoder, Some(frames), None)
    } else {
        let encoder = VideoEncoder::open(&name, &enc_cfg, None)?;
        let target = nv12_texture(&gpu, cfg.width, cfg.height, false)?;
        let staging = nv12_texture(&gpu, cfg.width, cfg.height, true)?;
        (encoder, None, Some((target, staging)))
    };
    let capture = WgcCapture::start(&gpu, cfg.fps, cfg.cursor)?;
    let hdr = hdr::is_hdr_active(&gpu.output).unwrap_or(false);
    let info = StartInfo {
        encoder: name,
        hardware,
        params: encoder.codec_parameters(),
        border_removed: capture.border_removed,
        hdr_active: hdr,
        adapter: gpu.adapter_name(),
    };
    Ok((
        Pipeline {
            cfg: *cfg,
            gpu,
            capture,
            encoder,
            frames,
            sw,
            converter: None,
            hdr,
        },
        info,
    ))
}

fn run(
    cfg: VideoConfig,
    sink: Sender<VideoEvent>,
    stop: Arc<AtomicBool>,
    force_kf: Arc<AtomicBool>,
    stats: Arc<Mutex<VideoStats>>,
    ready: Sender<Result<StartInfo>>,
) {
    let mut p = match setup(&cfg) {
        Ok((p, info)) => {
            let _ = ready.send(Ok(info));
            p
        }
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    drop(ready);
    if let Err(e) = p.run_loop(&sink, &stop, &force_kf, &stats) {
        let _ = sink.send(VideoEvent::DeviceLost(e.to_string()));
    }
}

impl Pipeline {
    fn run_loop(
        &mut self,
        sink: &Sender<VideoEvent>,
        stop: &AtomicBool,
        force_kf: &AtomicBool,
        stats: &Mutex<VideoStats>,
    ) -> Result<()> {
        let mut pacer = Pacer::new(self.cfg.fps)?;
        let mut last_seq = u64::MAX;
        let mut first = true;
        while !stop.load(Ordering::Relaxed) {
            let tick = pacer.wait()?;
            let mut repeated = false;
            let mut encoded = false;

            if tick.n % u64::from(self.cfg.fps) == 0 {
                self.gpu.check_removed()?;
                let hdr = hdr::is_hdr_active(&self.gpu.output).unwrap_or(self.hdr);
                if hdr != self.hdr {
                    self.hdr = hdr;
                    let _ = sink.send(VideoEvent::HdrChanged(hdr));
                }
            }

            if let Some(frame) = self.capture.acquire() {
                repeated = frame.seq == last_seq;
                last_seq = frame.seq;
                if self.converter.as_ref().map(|c| c.in_size()) != Some(frame.size) {
                    let changed = self.converter.is_some();
                    self.converter = Some(Converter::new(
                        &self.gpu,
                        InFormat::Bgra,
                        frame.size,
                        (self.cfg.width, self.cfg.height),
                    )?);
                    if changed {
                        let _ = sink.send(VideoEvent::FormatChanged {
                            width: frame.size.0,
                            height: frame.size.1,
                        });
                    }
                }
                let keyframe = first || force_kf.swap(false, Ordering::Relaxed);
                first = false;
                self.encode(&frame.texture, tick.time_us, keyframe)?;
                encoded = true;
                drop(frame);
                if !self.drain(sink)? {
                    break;
                }
            }

            let counters = self.capture.counters();
            if let Ok(mut s) = stats.lock() {
                s.ticks += 1;
                s.encoded += u64::from(encoded);
                s.skipped_ticks += tick.skipped;
                s.repeated += u64::from(repeated);
                s.captured = counters.arrived;
                s.overwritten = counters.overwritten;
            }
        }
        self.encoder.flush()?;
        self.drain(sink)?;
        Ok(())
    }

    fn encode(&mut self, src: &ID3D11Texture2D, pts_us: i64, keyframe: bool) -> Result<()> {
        let conv = self.converter.as_ref().expect("Converter angelegt");
        let src = TexRef {
            tex: src,
            array_slice: 0,
        };
        if let Some(frames) = self.frames.as_mut() {
            let mut hw = frames.get()?;
            {
                let (tex, slice) = hw.texture()?;
                conv.run(
                    src,
                    TexRef {
                        tex,
                        array_slice: slice,
                    },
                )?;
            }
            self.encoder.send(&mut hw.frame, pts_us, keyframe)?;
        } else {
            let (target, staging) = self.sw.as_ref().expect("Software-Ziel angelegt");
            conv.run(
                src,
                TexRef {
                    tex: target,
                    array_slice: 0,
                },
            )?;
            let mut frame =
                download_nv12(&self.gpu, target, staging, self.cfg.width, self.cfg.height)?;
            self.encoder.send(&mut frame, pts_us, keyframe)?;
        }
        Ok(())
    }

    /// Gibt alle fertigen Pakete weiter; `false`, wenn der Empfänger weg ist.
    fn drain(&mut self, sink: &Sender<VideoEvent>) -> Result<bool> {
        while let Some(packet) = self.encoder.receive()? {
            let dts = if packet.dts == ffi::AV_NOPTS_VALUE {
                packet.pts
            } else {
                packet.dts
            };
            let event = VideoEvent::Packet(VideoPacket {
                pts_us: packet.pts,
                dts_us: dts,
                keyframe: packet.flags & ffi::AV_PKT_FLAG_KEY as i32 != 0,
                packet,
            });
            if sink.send(event).is_err() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
