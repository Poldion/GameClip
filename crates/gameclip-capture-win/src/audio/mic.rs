// SPDX-License-Identifier: GPL-3.0-or-later
//! Mikrofon im Shared Mode (Design, Datei 36). Gerät aus `audio.mic_device` oder das
//! Windows-Standardmikrofon. Pause und Pegel macht der Mixer.
//!
//! Noch offen: Neubinden bei `IMMNotificationClient` (Gerät entfernt, Standard gewechselt).

use wasapi::{AudioClient, DeviceEnumerator, Direction};

/// Ein Aufnahmegerät für die Auswahl in den Einstellungen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicDevice {
    pub id: String,
    pub name: String,
}

/// Initialisiert COM (MTA) auf dem aufrufenden Thread, falls noch nicht geschehen.
pub fn list_devices() -> Result<Vec<MicDevice>, String> {
    let _ = wasapi::initialize_mta();
    let err = |e: wasapi::WasapiError| e.to_string();
    let enumerator = DeviceEnumerator::new().map_err(err)?;
    let collection = enumerator
        .get_device_collection(&Direction::Capture)
        .map_err(err)?;
    let mut out = Vec::new();
    for device in &collection {
        let device = device.map_err(err)?;
        out.push(MicDevice {
            id: device.get_id().map_err(err)?,
            name: device.get_friendlyname().unwrap_or_default(),
        });
    }
    Ok(out)
}

/// ID des Windows-Standardmikrofons.
pub fn default_device_id() -> Result<String, String> {
    let _ = wasapi::initialize_mta();
    let err = |e: wasapi::WasapiError| e.to_string();
    DeviceEnumerator::new()
        .map_err(err)?
        .get_default_device(&Direction::Capture)
        .map_err(err)?
        .get_id()
        .map_err(err)
}

/// Client für `device_id` oder das Standardgerät.
pub fn open(device_id: Option<&str>) -> Result<AudioClient, String> {
    let err = |e: wasapi::WasapiError| e.to_string();
    let enumerator = DeviceEnumerator::new().map_err(err)?;
    let device = match device_id {
        Some(id) => enumerator.get_device(id),
        None => enumerator.get_default_device(&Direction::Capture),
    }
    .map_err(err)?;
    device.get_iaudioclient().map_err(err)
}
