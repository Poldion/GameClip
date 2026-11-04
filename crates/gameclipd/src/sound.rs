// SPDX-License-Identifier: GPL-3.0-or-later
//! Bestätigungstöne (Design, Datei 50 und „Tray, Töne und Benachrichtigungen“).
//!
//! Die Töne werden im Code erzeugt statt als WAV-Dateien mitgeliefert: Erfolg zwei aufsteigende,
//! Fehler zwei absteigende kurze Sinustöne mit weichem Ein- und Ausklingen. Wiedergabe über
//! WASAPI (Standardgerät, Shared Mode) auf einem eigenen Thread mit `sound.volume`.
//!
//! Der Ton landet nicht im Clip (D26): Die Spur „System“ schließt den Prozessbaum von gameclipd
//! aus. Ist stattdessen Discord ausgeschlossen (M2), meldet `play` Beginn und Dauer, und der
//! Dienst schaltet „System“ in diesem Fenster stumm.

use crossbeam_channel::{Sender, unbounded};
use gameclip_capture_win::pacer::qpc_us;
use gameclip_core::state::Sound;
use tracing::warn;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

const RATE: f32 = 48_000.0;

/// (Frequenz in Hz, Dauer in ms) je Teilton.
fn notes(kind: Sound) -> &'static [(f32, u32)] {
    match kind {
        Sound::Success => &[(880.0, 70), (1_320.0, 110)],
        Sound::Failure => &[(440.0, 120), (311.0, 180)],
    }
}

/// Dauer des Tons in µs.
pub fn duration_us(kind: Sound) -> i64 {
    notes(kind)
        .iter()
        .map(|(_, ms)| i64::from(*ms) * 1000)
        .sum()
}

/// Interleaved Stereo-Float zu 48 kHz.
pub fn synth(kind: Sound, volume: f32) -> Vec<f32> {
    let mut out = Vec::new();
    for &(freq, ms) in notes(kind) {
        let n = (RATE * ms as f32 / 1000.0) as usize;
        let fade = (RATE * 0.008) as usize;
        for i in 0..n {
            let env = (i.min(n - 1 - i) as f32 / fade as f32).min(1.0);
            let s = (i as f32 * freq * std::f32::consts::TAU / RATE).sin() * 0.4 * env * volume;
            out.extend([s, s]);
        }
    }
    out
}

pub struct SoundPlayer {
    tx: Sender<Vec<f32>>,
}

impl Default for SoundPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundPlayer {
    pub fn new() -> Self {
        let (tx, rx) = unbounded::<Vec<f32>>();
        let _ = std::thread::Builder::new()
            .name("gameclip-sound".into())
            .spawn(move || {
                let _ = wasapi::initialize_mta();
                for samples in rx {
                    if let Err(e) = render(&samples) {
                        warn!("Ton nicht abspielbar: {e}");
                    }
                }
            });
        SoundPlayer { tx }
    }

    /// Spielt den Ton ab (kehrt sofort zurück) und liefert (Beginn in QPC-µs, Dauer in µs).
    pub fn play(&self, kind: Sound, volume: f32) -> (i64, i64) {
        let start = qpc_us();
        let _ = self.tx.send(synth(kind, volume.clamp(0.0, 1.0)));
        // Puffer und Gerätelatenz: das Fenster großzügig um 100 ms verlängern.
        (start, duration_us(kind) + 100_000)
    }
}

fn render(samples: &[f32]) -> Result<(), String> {
    let err = |e: wasapi::WasapiError| e.to_string();
    let device = DeviceEnumerator::new()
        .map_err(err)?
        .get_default_device(&Direction::Render)
        .map_err(err)?;
    let mut client = device.get_iaudioclient().map_err(err)?;
    let format = WaveFormat::new(32, 32, &SampleType::Float, 48_000, 2, None);
    client
        .initialize_client(
            &format,
            &Direction::Render,
            &StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 0,
            },
        )
        .map_err(err)?;
    let event = client.set_get_eventhandle().map_err(err)?;
    let render = client.get_audiorenderclient().map_err(err)?;
    let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let frame_bytes = 8;
    let mut pos = 0;
    let mut started = false;
    while pos < bytes.len() {
        let space = client.get_available_space_in_frames().map_err(err)? as usize;
        let frames = space.min((bytes.len() - pos) / frame_bytes);
        if frames > 0 {
            render
                .write_to_device(frames, &bytes[pos..pos + frames * frame_bytes], None)
                .map_err(err)?;
            pos += frames * frame_bytes;
        }
        if !started {
            client.start_stream().map_err(err)?;
            started = true;
        }
        if event.wait_for_event(1000).is_err() {
            break;
        }
    }
    // Ausklingen lassen, bis der Puffer leer ist.
    for _ in 0..50 {
        if client.get_current_padding().map_err(err)? == 0 {
            break;
        }
        let _ = event.wait_for_event(100);
    }
    let _ = client.stop_stream();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tones_have_expected_length_and_no_clicks() {
        for kind in [Sound::Success, Sound::Failure] {
            let s = synth(kind, 1.0);
            assert_eq!(s.len() as i64 / 2, duration_us(kind) * 48 / 1000);
            // Weicher Anfang und weiches Ende: erste und letzte Samples nahe 0.
            assert!(s[0].abs() < 1e-3 && s[s.len() - 1].abs() < 0.05);
            assert!(s.iter().all(|v| v.abs() <= 0.4));
        }
        assert!(synth(Sound::Success, 0.0).iter().all(|v| *v == 0.0));
    }
}
