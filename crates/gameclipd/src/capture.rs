// SPDX-License-Identifier: GPL-3.0-or-later
//! Aufnahme des Dienstes: Video- und Audio-Pipeline in den Ringpuffer.
//!
//! Ein Pump-Thread nimmt die Pakete beider Pipelines entgegen und legt sie im Ring ab. Meldet
//! der Ring `NeedKeyframe` (Pool erschöpft), fordert er sofort ein IDR an. Ereignisse, die die
//! Zustandsmaschine betreffen, gehen als `RecorderEvent` an die Hauptschleife.

use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, select, unbounded};
use gameclip_capture_win::ring::{PushOutcome, Ring, RingConfig, Stream};
use gameclip_capture_win::{
    AudioConfig, AudioEvent, AudioPipeline, CaptureError, KeyframeHandle, MicConfig, Track,
    VideoConfig, VideoEvent, VideoPipeline,
};
use gameclip_core::forecast::{forecast, video_bps};
use gameclip_core::settings::Settings;
use gameclip_gpu::MonitorTarget;
use rsmpeg::avcodec::AVCodecParameters;

#[derive(Debug)]
pub enum RecorderEvent {
    DeviceLost(String),
    HdrChanged(bool),
    AudioSourceFailed {
        track: Track,
        error: String,
    },
    /// Pegel in dBFS alle 100 ms.
    Levels {
        system: f32,
        mic: Option<f32>,
    },
    /// Die Quelle hat ihre Größe geändert.
    FormatChanged {
        width: u32,
        height: u32,
    },
}

/// Monitor aus `capture.monitor` (GDI-Gerätename); unbekannt oder `None` = Hauptmonitor.
fn monitor_target(id: Option<&str>) -> MonitorTarget {
    id.and_then(|id| {
        gameclip_gpu::list_monitors()
            .ok()?
            .into_iter()
            .find(|m| m.id == id)
            .map(|m| MonitorTarget::Handle(m.monitor))
    })
    .unwrap_or(MonitorTarget::Primary)
}

pub struct RecorderConfig {
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub ring: RingConfig,
}

impl RecorderConfig {
    /// Aus den Einstellungen; `exclude_pid` ist der eigene Prozess (Q11).
    pub fn from_settings(settings: &Settings, exclude_pid: u32, physical_ram: u64) -> Self {
        let c = &settings.capture;
        let (width, height) = c.resolution.size();
        let fc = forecast(settings, physical_ram);
        RecorderConfig {
            video: VideoConfig {
                monitor: monitor_target(c.monitor.as_deref()),
                width,
                height,
                fps: c.fps,
                bitrate_bps: video_bps(c.resolution, c.fps, c.quality),
                cursor: c.cursor,
            },
            audio: AudioConfig {
                exclude_pid,
                system_gain_db: settings.audio.gain_db.system,
                mic: settings.audio.mic_enabled.then(|| MicConfig {
                    device_id: settings.audio.mic_device.clone(),
                    gain_db: settings.audio.gain_db.mic,
                }),
            },
            ring: RingConfig::from_forecast(&fc, c.buffer_seconds),
        }
    }
}

pub struct Recorder {
    video: Option<VideoPipeline>,
    audio: Option<AudioPipeline>,
    ring: Arc<Ring>,
    pump: Option<JoinHandle<()>>,
    video_params: AVCodecParameters,
    audio_params: Vec<(Track, AVCodecParameters)>,
    encoder: String,
    hardware: bool,
    hdr_active: bool,
}

impl Recorder {
    /// Startet die Pipelines in den vorhandenen Ring. Der Ring überlebt Neustarts (Geräteverlust,
    /// Sperren); der Aufrufer beginnt dann mit `Ring::new_section` einen neuen Abschnitt.
    pub fn start(
        video_cfg: VideoConfig,
        audio_cfg: AudioConfig,
        ring: Arc<Ring>,
        events: Sender<RecorderEvent>,
    ) -> Result<Self, CaptureError> {
        let (vtx, vrx) = unbounded();
        let (atx, arx) = unbounded();
        let video = VideoPipeline::start(video_cfg, vtx)?;
        let audio = AudioPipeline::start(audio_cfg, atx)?;
        let video_params = video.info().params.clone();
        let audio_params = audio.codec_parameters().to_vec();
        let (encoder, hardware, hdr_active) = (
            video.info().encoder.clone(),
            video.info().hardware,
            video.info().hdr_active,
        );
        let keyframe = video.keyframe_handle();
        let pump_ring = ring.clone();
        let pump = std::thread::Builder::new()
            .name("gameclip-pump".into())
            .spawn(move || pump(vrx, arx, &pump_ring, &keyframe, &events))
            .map_err(|_| CaptureError::StartFailed)?;
        Ok(Recorder {
            video: Some(video),
            audio: Some(audio),
            ring,
            pump: Some(pump),
            video_params,
            audio_params,
            encoder,
            hardware,
            hdr_active,
        })
    }

    pub fn ring(&self) -> &Arc<Ring> {
        &self.ring
    }

    pub fn video_params(&self) -> &AVCodecParameters {
        &self.video_params
    }

    pub fn audio_params(&self) -> &[(Track, AVCodecParameters)] {
        &self.audio_params
    }

    pub fn encoder(&self) -> (&str, bool) {
        (&self.encoder, self.hardware)
    }

    pub fn hdr_active(&self) -> bool {
        self.hdr_active
    }

    pub fn audio(&self) -> Option<&AudioPipeline> {
        self.audio.as_ref()
    }

    /// Hält beide Pipelines und den Pump-Thread an.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        if let Some(v) = self.video.take() {
            v.stop();
        }
        if let Some(a) = self.audio.take() {
            a.stop();
        }
        if let Some(p) = self.pump.take() {
            let _ = p.join();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn packet_bytes(p: &rsmpeg::avcodec::AVPacket) -> &[u8] {
    if p.data.is_null() || p.size <= 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(p.data, p.size as usize) }
}

fn pump(
    vrx: Receiver<VideoEvent>,
    arx: Receiver<AudioEvent>,
    ring: &Ring,
    keyframe: &KeyframeHandle,
    events: &Sender<RecorderEvent>,
) {
    let (mut video_open, mut audio_open) = (true, true);
    while video_open || audio_open {
        select! {
            recv(vrx) -> ev => match ev {
                Ok(VideoEvent::Packet(p)) => {
                    let outcome = ring.push(
                        Stream::Video,
                        p.pts_us,
                        p.dts_us,
                        p.keyframe,
                        packet_bytes(&p.packet),
                    );
                    if outcome == PushOutcome::NeedKeyframe {
                        keyframe.force();
                    }
                }
                Ok(VideoEvent::DeviceLost(e)) => {
                    let _ = events.send(RecorderEvent::DeviceLost(e));
                }
                Ok(VideoEvent::HdrChanged(h)) => {
                    let _ = events.send(RecorderEvent::HdrChanged(h));
                }
                Ok(VideoEvent::FormatChanged { width, height }) => {
                    let _ = events.send(RecorderEvent::FormatChanged { width, height });
                }
                Err(_) => video_open = false,
            },
            recv(arx) -> ev => match ev {
                Ok(AudioEvent::Packet(p)) => {
                    ring.push(
                        Stream::Audio(p.track),
                        p.pts_us,
                        p.pts_us,
                        true,
                        packet_bytes(&p.packet),
                    );
                }
                Ok(AudioEvent::Levels { system_dbfs, mic_dbfs, .. }) => {
                    let _ = events.send(RecorderEvent::Levels { system: system_dbfs, mic: mic_dbfs });
                }
                Ok(AudioEvent::SourceFailed { track, error }) => {
                    let _ = events.send(RecorderEvent::AudioSourceFailed { track, error });
                }
                Err(_) => audio_open = false,
            },
        }
    }
}
