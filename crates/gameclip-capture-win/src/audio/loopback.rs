// SPDX-License-Identifier: GPL-3.0-or-later
//! Prozess-Loopback über das Crate `wasapi` und die gemeinsame Erfassungsschleife
//! (Design, Datei 35).
//!
//! Format fest 48 kHz Float Stereo; `autoconvert` lässt Windows umrechnen. Zeitstempel aus
//! `pu64QPCPosition` (100 ns) in µs.

use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_channel::Sender;
use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat};

use super::{SourceMsg, Track};
use crate::pacer::qpc_us;

/// Puffer von WASAPI (20 ms in 100 ns).
const BUFFER_HNS: i64 = 200_000;
/// Wartezeit auf ein Ereignis, danach wird `stop` geprüft.
const WAIT_MS: u32 = 100;

/// Ein Block einer Quelle.
pub struct Chunk {
    pub track: Track,
    /// QPC-Zeit des ersten Frames in µs.
    pub qpc_us: i64,
    pub samples: Vec<f32>,
    pub silent: bool,
}

/// Loopback-Client: `include = false` nimmt alles außer dem Prozessbaum `pid` auf (System),
/// `include = true` nur diesen Baum (Discord, ab M2).
pub fn open_loopback(pid: u32, include: bool) -> Result<AudioClient, String> {
    AudioClient::new_application_loopback_client(pid, include).map_err(|e| e.to_string())
}

pub fn format() -> WaveFormat {
    WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, None)
}

/// Erfasst bis `stop` und schickt die Blöcke an den Mixer.
pub fn run_capture(
    mut client: AudioClient,
    track: Track,
    tx: &Sender<SourceMsg>,
    stop: &AtomicBool,
) -> Result<(), String> {
    let err = |e: wasapi::WasapiError| e.to_string();
    let format = format();
    let block_align = format.get_blockalign() as usize;
    client
        .initialize_client(
            &format,
            &Direction::Capture,
            &StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: BUFFER_HNS,
            },
        )
        .map_err(err)?;
    let event = client.set_get_eventhandle().map_err(err)?;
    let capture = client.get_audiocaptureclient().map_err(err)?;
    client.start_stream().map_err(err)?;

    let mut bytes = Vec::new();
    while !stop.load(Ordering::Relaxed) {
        if event.wait_for_event(WAIT_MS).is_err() {
            continue;
        }
        while let Some(frames) = capture.get_next_packet_size().map_err(err)? {
            if frames == 0 {
                break;
            }
            bytes.resize(frames as usize * block_align, 0);
            let (read, info) = capture.read_from_device(&mut bytes).map_err(err)?;
            if read == 0 {
                break;
            }
            let data = &bytes[..read as usize * block_align];
            let samples: Vec<f32> = data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            // Ohne gültigen Zeitstempel: Ankunftszeit minus Blockdauer.
            let qpc = if info.timestamp == 0 || info.flags.timestamp_error {
                qpc_us() - i64::from(read) * 1_000_000 / 48_000
            } else {
                (info.timestamp / 10) as i64
            };
            let msg = SourceMsg::Chunk(Chunk {
                track,
                qpc_us: qpc,
                samples,
                silent: info.flags.silent,
            });
            if tx.send(msg).is_err() {
                return Ok(());
            }
        }
    }
    let _ = client.stop_stream();
    Ok(())
}
