// SPDX-License-Identifier: GPL-3.0-or-later
//! Kürzen mit Neukodierung (Design, „Kürzen und Export“, Ablauf 1–5).
//!
//! 1. Quelle mit D3D11VA öffnen (sonst Software).
//! 2. Zum Keyframe vor `in_us` springen und bis `in_us` ohne Ausgabe dekodieren.
//! 3. Bilder mit `in_us ≤ t < out_us` über den Converter (ggf. skaliert) mit dem Export-Profil
//!    kodieren.
//! 4. Ton sample-genau schneiden und neu kodieren.
//! 5. Mit `+faststart+use_metadata_tags` nach `target_part_path` muxen und sichern.
//!
//! Das Umbenennen übernimmt der Dienst.

use std::path::Path;

use gameclip_core::export_plan::ExportPlan;
use gameclip_core::forecast::{AUDIO_EXPORT_BPS, AUDIO_MIX_BPS, AUDIO_TRACK_BPS};
use gameclip_gpu::{Converter, GpuDevice, InFormat, TexRef};
use gameclip_media::audio_decode::{DecodedAudio, decode_audio};
use gameclip_media::decode::{Decoder, Uploader};
use gameclip_media::demux::{VideoInfo, video_info};
use gameclip_media::encoder::{candidate_order, is_hardware};
use gameclip_media::hwdevice::{download_nv12, nv12_texture};
use gameclip_media::mux::{Track as MuxTrack, write_mp4};
use gameclip_media::{
    AudioEncoder, EncodeProfile, EncoderConfig, HwDevice, HwFrames, MediaError, VideoEncoder,
};
use gameclip_protocol::messages::ExportProfile;
use gameclip_protocol::worker::{WorkerError, WorkerJob, WorkerMessage};
use rsmpeg::avcodec::AVPacket;
use rsmpeg::ffi;

use crate::size_target::{plan_for, with_correction};

/// Fehler mit Code und Text für die letzte Meldung.
#[derive(Debug, Clone, PartialEq)]
pub struct Fail(pub WorkerError, pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Output {
    pub bytes: u64,
    pub duration_us: u64,
}

const MICROS: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 1_000_000,
};
const SAMPLES: ffi::AVRational = ffi::AVRational {
    num: 1,
    den: 48_000,
};
/// Bitrate, wenn die Quelle keine angibt (1080p60 „Mittel“).
const FALLBACK_VIDEO_BPS: u64 = 14_000_000;

fn decode_err(e: MediaError) -> Fail {
    Fail(WorkerError::DecodeFailed, e.to_string())
}

fn encode_err(e: impl std::fmt::Display) -> Fail {
    Fail(WorkerError::EncoderFailed, e.to_string())
}

/// Eine Tonspur der Ausgabe.
struct AudioOut<'a> {
    title: String,
    pcm: &'a DecodedAudio,
    bps: u64,
}

/// Führt den Auftrag aus; Meldungen gehen an `emit`.
pub fn run(job: &WorkerJob, emit: &mut dyn FnMut(WorkerMessage)) -> Result<Output, Fail> {
    let source = Path::new(&job.source);
    if !source.is_file() {
        return Err(Fail(WorkerError::InputMissing, job.source.clone()));
    }
    if job.out_us <= job.in_us {
        return Err(Fail(WorkerError::InputMissing, "leerer Zeitraum".into()));
    }
    let info = video_info(source).map_err(decode_err)?;
    let gpu = GpuDevice::for_adapter(job.adapter_luid).map_err(encode_err)?;
    // Das Spiel behält Vorrang auf der GPU (Design: niedrigster Wert).
    let _ = gpu.set_gpu_priority(-7);

    let clip_audio = decode_audio(source).map_err(decode_err)?;
    let track_audio = match (&job.profile, &job.tracks_path) {
        (ExportProfile::SeparateTracks, Some(p)) if Path::new(p).is_file() => {
            decode_audio(Path::new(p)).map_err(decode_err)?
        }
        _ => Vec::new(),
    };
    let mix = clip_audio
        .iter()
        .find(|a| a.title == "Mischung")
        .or(clip_audio.first());
    let duration_us = job.out_us - job.in_us;

    match job.profile {
        ExportProfile::TargetSize { bytes } => {
            let plan = plan_for(duration_us, bytes, (info.width, info.height, info.fps))?;
            let audio: Vec<AudioOut<'_>> = mix
                .map(|m| AudioOut {
                    title: "Mischung".into(),
                    pcm: m,
                    bps: AUDIO_EXPORT_BPS,
                })
                .into_iter()
                .collect();
            with_correction(&plan, bytes, |bps| {
                encode(job, &gpu, &info, &plan_with(&plan, bps), &audio, emit)
            })
        }
        ExportProfile::Mix | ExportProfile::SeparateTracks => {
            let mut audio: Vec<AudioOut<'_>> = mix
                .map(|m| AudioOut {
                    title: "Mischung".into(),
                    pcm: m,
                    bps: AUDIO_MIX_BPS,
                })
                .into_iter()
                .collect();
            if job.profile == ExportProfile::SeparateTracks {
                for title in ["System", "Mikrofon", "Discord"] {
                    if let Some(t) = track_audio.iter().find(|a| a.title == title) {
                        audio.push(AudioOut {
                            title: title.into(),
                            pcm: t,
                            bps: AUDIO_TRACK_BPS,
                        });
                    }
                }
            }
            let bps = if info.bit_rate > 0 {
                info.bit_rate
            } else {
                FALLBACK_VIDEO_BPS
            };
            let same = ExportPlan {
                width: info.width,
                height: info.height,
                fps: info.fps,
                video_bps: bps,
                warn_low_quality: false,
            };
            encode(job, &gpu, &info, &same, &audio, emit)
        }
    }
}

fn plan_with(plan: &ExportPlan, video_bps: u64) -> ExportPlan {
    ExportPlan { video_bps, ..*plan }
}

/// Öffnet den ersten nutzbaren Encoder: Reihenfolge aus dem Auftrag oder nach Hersteller.
fn open_encoder(
    job: &WorkerJob,
    gpu: &GpuDevice,
    device: &HwDevice,
    cfg: &EncoderConfig,
) -> Result<(VideoEncoder, Option<HwFrames>), Fail> {
    let order: Vec<String> = if job.encoder_order.is_empty() {
        candidate_order(gpu.vendor_id())
            .into_iter()
            .map(String::from)
            .collect()
    } else {
        job.encoder_order.clone()
    };
    let mut last = String::from("keine Encoder");
    for name in &order {
        if is_hardware(name) {
            let frames = HwFrames::new(device, cfg.width, cfg.height).map_err(encode_err)?;
            match VideoEncoder::open(name, cfg, Some(&frames)) {
                Ok(e) => return Ok((e, Some(frames))),
                Err(e) => last = format!("{name}: {e}"),
            }
        } else {
            match VideoEncoder::open(name, cfg, None) {
                Ok(e) => return Ok((e, None)),
                Err(e) => last = format!("{name}: {e}"),
            }
        }
    }
    // Keine zweite Hardware-Sitzung möglich: libx264 mit dem Export-Profil.
    VideoEncoder::open("libx264", cfg, None)
        .map(|e| (e, None))
        .map_err(|e| encode_err(format!("{last}; libx264: {e}")))
}

fn encode(
    job: &WorkerJob,
    gpu: &GpuDevice,
    info: &VideoInfo,
    plan: &ExportPlan,
    audio: &[AudioOut<'_>],
    emit: &mut dyn FnMut(WorkerMessage),
) -> Result<Output, Fail> {
    let (w, h, fps) = (plan.width, plan.height, plan.fps.max(1));
    let cfg = EncoderConfig {
        width: w,
        height: h,
        fps,
        bitrate_bps: plan.video_bps,
        profile: EncodeProfile::Export,
    };
    let device = HwDevice::from_gpu(gpu).map_err(encode_err)?;
    let (mut encoder, mut frames) = open_encoder(job, gpu, &device, &cfg)?;
    emit(WorkerMessage::Encoder {
        name: encoder.name().to_string(),
    });

    let source = Path::new(&job.source);
    let mut decoder = Decoder::open(source, Some(&device)).map_err(decode_err)?;
    decoder.seek_us(job.in_us as i64).map_err(decode_err)?;
    let converter = Converter::new(gpu, InFormat::Nv12, (info.width, info.height), (w, h))
        .map_err(encode_err)?;
    let mut uploader =
        Uploader::new(HwFrames::new(&device, info.width, info.height).map_err(encode_err)?);
    let sw_target = if frames.is_none() {
        Some((
            nv12_texture(gpu, w, h, false).map_err(encode_err)?,
            nv12_texture(gpu, w, h, true).map_err(encode_err)?,
        ))
    } else {
        None
    };

    let (in_us, out_us) = (job.in_us as i64, job.out_us as i64);
    let span = (out_us - in_us) as f64;
    // Ausgabe im festen Raster der Ziel-Bildrate; bei halber Rate fällt jedes zweite Bild weg.
    let out_time = |k: i64| k * 1_000_000 / i64::from(fps);
    let half_src = 500_000 / i64::from(info.fps.max(1));
    let mut k = 0i64;
    let mut packets: Vec<AVPacket> = Vec::new();
    let mut last_percent = -1i32;
    while let Some(f) = decoder.next_frame().map_err(decode_err)? {
        if f.pts_us < in_us - half_src {
            continue;
        }
        if f.pts_us >= out_us - half_src {
            break;
        }
        let rel = f.pts_us - in_us;
        if rel + half_src < out_time(k) {
            continue;
        }
        let uploaded;
        let (tex, slice) = if f.is_hardware() {
            f.texture().map_err(decode_err)?
        } else {
            uploaded = uploader.upload(&f.frame).map_err(decode_err)?;
            uploaded.texture().map_err(decode_err)?
        };
        let src = TexRef {
            tex,
            array_slice: slice,
        };
        let pts = out_time(k);
        if let Some(pool) = frames.as_mut() {
            let mut hw = pool.get().map_err(encode_err)?;
            {
                let (dst, dst_slice) = hw.texture().map_err(encode_err)?;
                converter
                    .run(
                        src,
                        TexRef {
                            tex: dst,
                            array_slice: dst_slice,
                        },
                    )
                    .map_err(encode_err)?;
            }
            encoder
                .send(&mut hw.frame, pts, k == 0)
                .map_err(encode_err)?;
        } else {
            let (target, staging) = sw_target.as_ref().expect("Software-Ziel");
            converter
                .run(
                    src,
                    TexRef {
                        tex: target,
                        array_slice: 0,
                    },
                )
                .map_err(encode_err)?;
            let mut frame = download_nv12(gpu, target, staging, w, h).map_err(encode_err)?;
            encoder.send(&mut frame, pts, k == 0).map_err(encode_err)?;
        }
        while let Some(p) = encoder.receive().map_err(encode_err)? {
            packets.push(p);
        }
        k += 1;
        let percent = ((rel as f64 / span) * 90.0) as i32;
        if percent > last_percent {
            last_percent = percent;
            emit(WorkerMessage::Progress {
                percent: percent as f32,
            });
        }
    }
    encoder.flush().map_err(encode_err)?;
    while let Some(p) = encoder.receive().map_err(encode_err)? {
        packets.push(p);
    }
    if packets.is_empty() {
        return Err(Fail(
            WorkerError::DecodeFailed,
            "keine Bilder im Zeitraum".into(),
        ));
    }
    let duration_us = out_time(k) as u64;

    // Ton: sample-genau auf die Videolänge, neu kodiert.
    let mut audio_packets = Vec::new();
    for a in audio {
        let pcm = a.pcm.cut(job.in_us, job.in_us + duration_us);
        let mut enc = AudioEncoder::new(a.bps).map_err(encode_err)?;
        let mut p = enc.push(&pcm).map_err(encode_err)?;
        p.extend(enc.finish().map_err(encode_err)?);
        audio_packets.push((a.title.clone(), enc.codec_parameters(), p));
    }
    emit(WorkerMessage::Progress { percent: 95.0 });

    let created = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();
    let tags = [
        ("gameclip_id", job.output_tag_id.as_str()),
        ("gameclip_created", created.as_str()),
        ("gameclip_version", env!("CARGO_PKG_VERSION")),
    ];
    let params = encoder.codec_parameters();
    let tracks: Vec<MuxTrack<'_>> = audio_packets
        .iter()
        .enumerate()
        .map(|(i, (title, par, p))| MuxTrack {
            params: par,
            time_base: SAMPLES,
            packets: p,
            title,
            default: i == 0,
        })
        .collect();
    let part = Path::new(&job.target_part_path);
    write_mp4(
        part,
        &MuxTrack {
            params: &params,
            time_base: MICROS,
            packets: &packets,
            title: "Video",
            default: true,
        },
        &tracks,
        &tags,
        0,
        true,
    )
    .map_err(|e| Fail(WorkerError::Io, e.to_string()))?;
    let bytes = std::fs::metadata(part)
        .map(|m| m.len())
        .map_err(|e| Fail(WorkerError::Io, e.to_string()))?;
    emit(WorkerMessage::Progress { percent: 100.0 });
    Ok(Output { bytes, duration_us })
}
