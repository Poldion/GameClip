// SPDX-License-Identifier: GPL-3.0-or-later
//! Zustandsmaschine des Dienstes (Design: „Zustandsmaschine“).
//!
//! [`transition`] ist eine reine Funktion. Der Dienst führt die zurückgegebenen
//! [`Effect`]s aus; nicht aufgeführte Kombinationen ändern nichts und werden protokolliert.

use std::time::Duration;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Wartezeiten beim Neuaufbau nach Geräteverlust; nach dem letzten Versuch folgt „Fehler“.
pub const RECOVERY_DELAYS_S: [u64; 5] = [1, 2, 4, 8, 8];

/// Höchstens ein laufender und ein wartender Speichervorgang.
pub const MAX_SAVES_IN_FLIGHT: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ServiceState {
    Starting,
    Buffering,
    Paused,
    Reconfiguring,
    Recovering { attempt: u8 },
    Failed,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
    pub struct Restrictions: u8 {
        const HOTKEY_INACTIVE = 0b001;
        const CPU_ENCODER     = 0b010;
        const HDR_ACTIVE      = 0b100;
    }
}

impl Restrictions {
    /// Namen für `ServiceStatus.restrictions` und den Tooltip.
    pub fn names(self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.contains(Restrictions::HOTKEY_INACTIVE) {
            out.push("hotkey_inactive");
        }
        if self.contains(Restrictions::CPU_ENCODER) {
            out.push("cpu_encoder");
        }
        if self.contains(Restrictions::HDR_ACTIVE) {
            out.push("hdr_active");
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError {
    DiskFull,
    FolderMissing,
    BufferEmpty,
    EncoderFailed,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateEvent {
    PipelineReady,
    PipelineFailed,
    HotkeyRegistered,
    HotkeyFailed,
    CpuEncoderOnly(bool),
    HdrChanged(bool),
    Hotkey,
    SaveFinished(Result<(), SaveError>),
    SettingsChanged { pipeline: bool },
    DeviceLost,
    RecoverySucceeded,
    RecoveryFailed,
    StopBuffer,
    StartBuffer,
    SessionLocked,
    SessionUnlocked,
    Restart,
    MicPause(bool),
    UiDisconnectedWhileSuspended,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notice {
    SaveFailed(SaveError),
    HotkeyInactive,
    CpuEncoder,
    Failed,
    NotReady,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    StartPipeline,
    StopPipeline,
    RebuildBuffer,
    BeginSave,
    QueueSave,
    PlaySound(Sound),
    Notify(Notice),
    ScheduleRecovery(Duration),
    SetMicPaused(bool),
    ReRegisterHotkey,
    UpdateTray,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Machine {
    pub state: ServiceState,
    pub restrictions: Restrictions,
    pub mic_paused: bool,
    pub saves_in_flight: u8,
    /// `true`, wenn „Pausiert“ durch das Sperren des PCs entstand (nicht durch den Nutzer).
    pub paused_by_lock: bool,
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            state: ServiceState::Starting,
            restrictions: Restrictions::empty(),
            mic_paused: false,
            saves_in_flight: 0,
            paused_by_lock: false,
        }
    }
}

/// Wendet ein Ereignis an. Gibt den neuen Zustand und die auszuführenden Wirkungen zurück.
pub fn transition(m: &Machine, event: StateEvent) -> (Machine, Vec<Effect>) {
    use Effect::*;
    use ServiceState::*;
    use StateEvent as E;

    let mut next = *m;
    let mut fx = Vec::new();

    match (m.state, event) {
        // --- Regeln für alle Zustände ---
        (_, E::HotkeyRegistered) => {
            next.restrictions.remove(Restrictions::HOTKEY_INACTIVE);
            fx.push(UpdateTray);
        }
        (_, E::HotkeyFailed) => {
            next.restrictions.insert(Restrictions::HOTKEY_INACTIVE);
            fx.extend([
                PlaySound(Sound::Failure),
                Notify(Notice::HotkeyInactive),
                UpdateTray,
            ]);
        }
        (_, E::CpuEncoderOnly(on)) => {
            next.restrictions.set(Restrictions::CPU_ENCODER, on);
            if on && !m.restrictions.contains(Restrictions::CPU_ENCODER) {
                fx.push(Notify(Notice::CpuEncoder));
            }
            fx.push(UpdateTray);
        }
        (_, E::HdrChanged(on)) => {
            next.restrictions.set(Restrictions::HDR_ACTIVE, on);
            fx.push(UpdateTray);
        }
        (_, E::SaveFinished(result)) => {
            next.saves_in_flight = m.saves_in_flight.saturating_sub(1);
            match result {
                Ok(()) => fx.push(PlaySound(Sound::Success)),
                Err(e) => fx.extend([PlaySound(Sound::Failure), Notify(Notice::SaveFailed(e))]),
            }
            if next.saves_in_flight > 0 {
                fx.push(BeginSave);
            }
            fx.push(UpdateTray);
        }
        (_, E::MicPause(on)) => {
            next.mic_paused = on;
            fx.extend([SetMicPaused(on), UpdateTray]);
        }
        (_, E::UiDisconnectedWhileSuspended) => fx.push(ReRegisterHotkey),
        (_, E::Quit) => fx.push(Exit),

        // --- Start ---
        (Starting, E::PipelineReady) | (Reconfiguring, E::PipelineReady) => {
            next.state = Buffering;
            fx.push(UpdateTray);
        }
        (Starting, E::PipelineFailed) | (Reconfiguring, E::PipelineFailed) => {
            next.state = Failed;
            fx.extend([Notify(Notice::Failed), UpdateTray]);
        }

        // --- Speichern ---
        (Buffering, E::Hotkey) => match m.saves_in_flight {
            0 => {
                next.saves_in_flight = 1;
                fx.extend([BeginSave, UpdateTray]);
            }
            n if n < MAX_SAVES_IN_FLIGHT => {
                next.saves_in_flight = n + 1;
                fx.push(QueueSave);
            }
            _ => {}
        },
        (Starting | Reconfiguring | Recovering { .. } | Paused | Failed, E::Hotkey) => {
            fx.extend([PlaySound(Sound::Failure), Notify(Notice::NotReady)]);
        }

        // --- Einstellungen ---
        (Buffering, E::SettingsChanged { pipeline: true }) => {
            next.state = Reconfiguring;
            fx.extend([RebuildBuffer, UpdateTray]);
        }
        (Failed, E::SettingsChanged { .. }) | (Failed, E::Restart) => {
            next.state = Starting;
            fx.extend([StartPipeline, UpdateTray]);
        }
        (Buffering, E::Restart) => {
            next.state = Starting;
            fx.extend([StopPipeline, StartPipeline, UpdateTray]);
        }

        // --- Geräteverlust ---
        (Buffering | Starting | Reconfiguring | Paused, E::DeviceLost) => {
            next.state = Recovering { attempt: 0 };
            fx.extend([
                ScheduleRecovery(Duration::from_secs(RECOVERY_DELAYS_S[0])),
                UpdateTray,
            ]);
        }
        (Recovering { .. }, E::RecoverySucceeded) => {
            next.state = Buffering;
            fx.push(UpdateTray);
        }
        (Recovering { attempt }, E::RecoveryFailed) => {
            let n = usize::from(attempt) + 1;
            if n < RECOVERY_DELAYS_S.len() {
                next.state = Recovering {
                    attempt: attempt + 1,
                };
                fx.push(ScheduleRecovery(Duration::from_secs(RECOVERY_DELAYS_S[n])));
            } else {
                next.state = Failed;
                fx.extend([Notify(Notice::Failed), UpdateTray]);
            }
        }

        // --- Pausieren ---
        (Buffering, E::StopBuffer) => {
            next.state = Paused;
            next.paused_by_lock = false;
            fx.extend([StopPipeline, UpdateTray]);
        }
        (Paused, E::StartBuffer) => {
            next.state = Starting;
            next.paused_by_lock = false;
            fx.extend([StartPipeline, UpdateTray]);
        }
        (Buffering | Starting | Reconfiguring | Recovering { .. }, E::SessionLocked) => {
            next.state = Paused;
            next.paused_by_lock = true;
            fx.extend([StopPipeline, UpdateTray]);
        }
        (Paused, E::SessionUnlocked) if m.paused_by_lock => {
            next.state = Starting;
            next.paused_by_lock = false;
            fx.extend([StartPipeline, UpdateTray]);
        }

        // Alles Übrige ändert nichts.
        _ => {}
    }

    (next, fx)
}
