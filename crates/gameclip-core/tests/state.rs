// SPDX-License-Identifier: GPL-3.0-or-later
use std::time::Duration;

use gameclip_core::state::{
    Effect, Machine, Notice, RECOVERY_DELAYS_S, Restrictions, SaveError, ServiceState, Sound,
    StateEvent, transition,
};

fn buffering() -> Machine {
    Machine {
        state: ServiceState::Buffering,
        ..Machine::default()
    }
}

fn step(m: Machine, e: StateEvent) -> (Machine, Vec<Effect>) {
    transition(&m, e)
}

#[test]
fn start_to_buffering() {
    let (m, fx) = step(Machine::default(), StateEvent::PipelineReady);
    assert_eq!(m.state, ServiceState::Buffering);
    assert_eq!(fx, [Effect::UpdateTray]);
}

#[test]
fn start_failure() {
    let (m, fx) = step(Machine::default(), StateEvent::PipelineFailed);
    assert_eq!(m.state, ServiceState::Failed);
    assert!(fx.contains(&Effect::Notify(Notice::Failed)));
}

#[test]
fn hotkey_starts_then_queues_then_ignores() {
    let (m, fx) = step(buffering(), StateEvent::Hotkey);
    assert_eq!(m.saves_in_flight, 1);
    assert!(fx.contains(&Effect::BeginSave));
    let (m, fx) = step(m, StateEvent::Hotkey);
    assert_eq!(m.saves_in_flight, 2);
    assert_eq!(fx, [Effect::QueueSave]);
    let (m, fx) = step(m, StateEvent::Hotkey);
    assert_eq!(m.saves_in_flight, 2);
    assert!(fx.is_empty());
}

#[test]
fn finished_save_starts_queued_one() {
    let m = Machine {
        saves_in_flight: 2,
        ..buffering()
    };
    let (m, fx) = step(m, StateEvent::SaveFinished(Ok(())));
    assert_eq!(m.saves_in_flight, 1);
    assert!(fx.contains(&Effect::PlaySound(Sound::Success)));
    assert!(fx.contains(&Effect::BeginSave));
    let (m, fx) = step(m, StateEvent::SaveFinished(Err(SaveError::DiskFull)));
    assert_eq!(m.saves_in_flight, 0);
    assert!(fx.contains(&Effect::Notify(Notice::SaveFailed(SaveError::DiskFull))));
    assert!(!fx.contains(&Effect::BeginSave));
}

#[test]
fn save_finishes_during_reconfiguring_and_recovering() {
    for state in [
        ServiceState::Reconfiguring,
        ServiceState::Recovering { attempt: 1 },
    ] {
        let m = Machine {
            state,
            saves_in_flight: 1,
            ..Machine::default()
        };
        let (m, fx) = step(m, StateEvent::SaveFinished(Ok(())));
        assert_eq!(m.saves_in_flight, 0);
        assert!(fx.contains(&Effect::PlaySound(Sound::Success)));
    }
}

#[test]
fn hotkey_when_not_ready_plays_failure() {
    for state in [
        ServiceState::Starting,
        ServiceState::Reconfiguring,
        ServiceState::Recovering { attempt: 0 },
        ServiceState::Paused,
        ServiceState::Failed,
    ] {
        let m = Machine {
            state,
            ..Machine::default()
        };
        let (m2, fx) = step(m, StateEvent::Hotkey);
        assert_eq!(m2.saves_in_flight, 0);
        assert!(fx.contains(&Effect::PlaySound(Sound::Failure)), "{state:?}");
    }
}

#[test]
fn restrictions_apply_in_any_state() {
    for state in [
        ServiceState::Starting,
        ServiceState::Buffering,
        ServiceState::Failed,
    ] {
        let m = Machine {
            state,
            ..Machine::default()
        };
        let (m, _) = step(m, StateEvent::HotkeyFailed);
        assert!(m.restrictions.contains(Restrictions::HOTKEY_INACTIVE));
        let (m, _) = step(m, StateEvent::HdrChanged(true));
        let (m, _) = step(m, StateEvent::CpuEncoderOnly(true));
        assert_eq!(
            m.restrictions.names(),
            ["hotkey_inactive", "cpu_encoder", "hdr_active"]
        );
        let (m, _) = step(m, StateEvent::HotkeyRegistered);
        let (m, _) = step(m, StateEvent::HdrChanged(false));
        assert_eq!(m.restrictions, Restrictions::CPU_ENCODER);
        assert_eq!(m.state, state);
    }
}

#[test]
fn pipeline_settings_rebuild_buffer_others_do_not() {
    let (m, fx) = step(buffering(), StateEvent::SettingsChanged { pipeline: false });
    assert_eq!(m.state, ServiceState::Buffering);
    assert!(fx.is_empty());
    let (m, fx) = step(buffering(), StateEvent::SettingsChanged { pipeline: true });
    assert_eq!(m.state, ServiceState::Reconfiguring);
    assert!(fx.contains(&Effect::RebuildBuffer));
    let (m, _) = step(m, StateEvent::PipelineReady);
    assert_eq!(m.state, ServiceState::Buffering);
}

#[test]
fn recovery_backs_off_then_fails() {
    let (mut m, fx) = step(buffering(), StateEvent::DeviceLost);
    assert_eq!(m.state, ServiceState::Recovering { attempt: 0 });
    assert!(fx.contains(&Effect::ScheduleRecovery(Duration::from_secs(
        RECOVERY_DELAYS_S[0]
    ))));
    for (attempt, &delay) in RECOVERY_DELAYS_S.iter().enumerate().skip(1) {
        let (next, fx) = step(m, StateEvent::RecoveryFailed);
        assert_eq!(
            next.state,
            ServiceState::Recovering {
                attempt: attempt as u8
            }
        );
        assert_eq!(fx, [Effect::ScheduleRecovery(Duration::from_secs(delay))]);
        m = next;
    }
    let (m, fx) = step(m, StateEvent::RecoveryFailed);
    assert_eq!(m.state, ServiceState::Failed);
    assert!(fx.contains(&Effect::Notify(Notice::Failed)));
    let (m, fx) = step(m, StateEvent::Restart);
    assert_eq!(m.state, ServiceState::Starting);
    assert!(fx.contains(&Effect::StartPipeline));
}

#[test]
fn recovery_success() {
    let m = Machine {
        state: ServiceState::Recovering { attempt: 2 },
        ..Machine::default()
    };
    assert_eq!(
        step(m, StateEvent::RecoverySucceeded).0.state,
        ServiceState::Buffering
    );
}

#[test]
fn user_pause_and_resume() {
    let (m, fx) = step(buffering(), StateEvent::StopBuffer);
    assert_eq!(m.state, ServiceState::Paused);
    assert!(!m.paused_by_lock);
    assert!(fx.contains(&Effect::StopPipeline));
    // Entsperren hebt eine Pause durch den Nutzer nicht auf.
    let (m, fx) = step(m, StateEvent::SessionUnlocked);
    assert_eq!(m.state, ServiceState::Paused);
    assert!(fx.is_empty());
    let (m, fx) = step(m, StateEvent::StartBuffer);
    assert_eq!(m.state, ServiceState::Starting);
    assert!(fx.contains(&Effect::StartPipeline));
}

#[test]
fn lock_pauses_and_unlock_resumes() {
    let (m, fx) = step(buffering(), StateEvent::SessionLocked);
    assert_eq!(m.state, ServiceState::Paused);
    assert!(m.paused_by_lock);
    assert!(fx.contains(&Effect::StopPipeline));
    let (m, fx) = step(m, StateEvent::SessionUnlocked);
    assert_eq!(m.state, ServiceState::Starting);
    assert!(!m.paused_by_lock);
    assert!(fx.contains(&Effect::StartPipeline));
}

#[test]
fn mic_pause_is_not_a_restriction() {
    let (m, fx) = step(buffering(), StateEvent::MicPause(true));
    assert!(m.mic_paused);
    assert!(m.restrictions.is_empty());
    assert!(fx.contains(&Effect::SetMicPaused(true)));
}

#[test]
fn ui_disconnect_reregisters_hotkey_and_quit_exits() {
    assert_eq!(
        step(buffering(), StateEvent::UiDisconnectedWhileSuspended).1,
        [Effect::ReRegisterHotkey]
    );
    assert_eq!(step(buffering(), StateEvent::Quit).1, [Effect::Exit]);
}

#[test]
fn unlisted_pairs_change_nothing() {
    let m = buffering();
    for e in [
        StateEvent::PipelineReady,
        StateEvent::RecoverySucceeded,
        StateEvent::StartBuffer,
    ] {
        let (next, fx) = step(m, e);
        assert_eq!(next, m);
        assert!(fx.is_empty());
    }
}
