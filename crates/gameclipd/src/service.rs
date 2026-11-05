// SPDX-License-Identifier: GPL-3.0-or-later
//! Hauptschleife des Dienstes (Design, Datei 40).
//!
//! Reihenfolge beim Start: Argumente (`--save-clip` folgt mit `cli.rs`) → Mutex (zweite Instanz
//! endet still mit 0) → Logging → Einstellungen → Index → Pipeline → Nachrichten-Thread
//! (Hotkey, Tray). Danach laufen alle Ereignisse durch `gameclip_core::state::transition`; die
//! Wirkungen führt `apply` aus.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, never, select, unbounded};
use gameclip_capture_win::Track;
use gameclip_capture_win::ring::Ring;
use gameclip_core::ids::MUTEX_NAME;
use gameclip_core::retention::FolderKind;
use gameclip_core::settings::{Settings, parse};
use gameclip_core::state::{Effect, Machine, SaveError, ServiceState, StateEvent, transition};
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::messages::{ExportProfile, Response, ServiceEvent};
use rsmpeg::avcodec::AVCodecParameters;
use time::OffsetDateTime;
use tracing::{error, info, warn};
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::CreateMutexW;
use windows_core::HSTRING;

use crate::capture::{Recorder, RecorderConfig, RecorderEvent};

#[path = "commands.rs"]
mod commands;
use crate::foreground::foreground_program;
use crate::index::Index;
use crate::ipc_server::{Broadcaster, IpcRequest, IpcServer};
use crate::jobs::{JobEvent, Jobs, JobsConfig, WORKER_SILENCE_TIMEOUT_S};
use crate::msgthread::{MsgEvent, MsgThread};
use crate::paths::Paths;
use crate::save::{SaveInputs, SaveRequest, Saved, save_clip};
use crate::tray::{TrayCommand, TrayState, TrayView};

/// Einstiegspunkt von `gameclipd.exe`; Rückgabe ist der Exit-Code.
pub fn run() -> i32 {
    if std::env::args().any(|a| a == "--save-clip") {
        return crate::cli::save_clip();
    }
    let Some(_mutex) = single_instance() else {
        return 0;
    };
    let base = Paths::resolve(&Settings::default());
    let _ = std::fs::create_dir_all(&base.logs);
    let _log = crate::logging::init(&base.logs);
    info!(
        "gameclipd {} startet (AUMID {})",
        env!("CARGO_PKG_VERSION"),
        crate::toast::aumid()
    );

    let (settings, settings_read_only) = load_settings(&base.settings);
    let paths = Paths::resolve(&settings);
    if let Err(e) = paths.create_all() {
        error!("Ordner lassen sich nicht anlegen: {e}");
    }
    let index = match Index::open(&paths.index) {
        Ok(i) => Arc::new(i),
        Err(e) => {
            error!("Index lässt sich nicht öffnen: {e}");
            return 1;
        }
    };
    if let Some(broken) = &index.recovered_from {
        warn!(
            "Beschädigter Index ersetzt, alte Datei: {}",
            broken.display()
        );
    }

    match Service::new(settings, settings_read_only, paths, index) {
        Ok(mut s) => {
            s.run();
            0
        }
        Err(e) => {
            error!("Start fehlgeschlagen: {e}");
            1
        }
    }
}

struct SingleInstance(HANDLE);

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

fn single_instance() -> Option<SingleInstance> {
    let handle = unsafe { CreateMutexW(None, true, &HSTRING::from(MUTEX_NAME)).ok()? };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }
        return None;
    }
    Some(SingleInstance(handle))
}

/// Einstellungen und ob die Datei nur gelesen werden darf (neuere Schema-Version).
fn load_settings(path: &std::path::Path) -> (Settings, bool) {
    match std::fs::read_to_string(path) {
        Ok(text) => match parse(&text) {
            Ok(loaded) => {
                let read_only = matches!(loaded, gameclip_core::settings::Loaded::ReadOnly(_));
                (loaded.settings().clone(), read_only)
            }
            Err(e) => {
                warn!("settings.toml unlesbar, nutze Standardwerte: {e}");
                (Settings::default(), false)
            }
        },
        Err(_) => (Settings::default(), false),
    }
}

pub(crate) fn physical_ram() -> u64 {
    let mut s = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut s) }
        .map(|_| s.ullTotalPhys)
        .unwrap_or(16 << 30)
}

/// Ein Speicherauftrag mit allem, was der Speicher-Thread braucht.
struct SaveJob {
    ring: Arc<Ring>,
    video_params: AVCodecParameters,
    audio_params: Vec<(Track, AVCodecParameters)>,
    buffer_us: i64,
    fps: u32,
    paths: Paths,
    index: Arc<Index>,
    request: SaveRequest,
}

fn save_thread(jobs: Receiver<SaveJob>, done: Sender<Result<Saved, SaveError>>) {
    for job in jobs {
        let inputs = SaveInputs {
            source: job.ring.as_ref(),
            video_params: &job.video_params,
            audio_params: &job.audio_params,
            buffer_us: job.buffer_us,
            fps: job.fps,
            paths: &job.paths,
            index: &job.index,
        };
        let result = save_clip(&inputs, &job.request);
        if done.send(result).is_err() {
            return;
        }
    }
}

struct Service {
    settings: Settings,
    paths: Paths,
    index: Arc<Index>,
    ring: Arc<Ring>,
    recorder: Option<Recorder>,
    machine: Machine,
    msg: MsgThread,
    msg_rx: Receiver<MsgEvent>,
    rec_tx: Sender<RecorderEvent>,
    rec_rx: Receiver<RecorderEvent>,
    save_tx: Sender<SaveJob>,
    save_rx: Receiver<Result<Saved, SaveError>>,
    /// Tastendrücke mit Programmname, die auf ihr Speichern warten.
    presses: VecDeque<(i64, Option<String>)>,
    recovery: Option<Receiver<std::time::Instant>>,
    last_clip: Option<PathBuf>,
    ipc_rx: Receiver<IpcRequest>,
    events: Broadcaster,
    jobs: Jobs,
    job_rx: Receiver<JobEvent>,
    sound: crate::sound::SoundPlayer,
    toast_failed: bool,
    /// Export aus der Tray-Aktion: nach Abschluss in die Zwischenablage (G1a, Größe 4).
    copy_after: Option<(u32, std::time::Instant)>,
    settings_read_only: bool,
    conns: commands::Conns,
    frames: crate::frames::FrameCache,
    watcher: Option<crate::watcher::Watcher>,
    watch_tx: Sender<FolderKind>,
    watch_rx: Receiver<FolderKind>,
    /// Entprellter Abgleich: Ordner und Zeitpunkt.
    reconcile_due: Option<(std::collections::HashSet<FolderKind>, std::time::Instant)>,
    exit: bool,
}

impl Service {
    fn new(
        settings: Settings,
        settings_read_only: bool,
        paths: Paths,
        index: Arc<Index>,
    ) -> std::io::Result<Self> {
        let (msg_tx, msg_rx) = unbounded();
        let machine = Machine::default();
        let msg = MsgThread::start(msg_tx, view(&machine, settings.export.target_bytes))?;
        let (rec_tx, rec_rx) = unbounded();
        let (save_tx, jobs) = unbounded();
        let (done_tx, save_rx) = unbounded();
        std::thread::Builder::new()
            .name("gameclip-save".into())
            .spawn(move || save_thread(jobs, done_tx))?;
        let ring_cfg =
            RecorderConfig::from_settings(&settings, std::process::id(), physical_ram()).ring;
        let (ipc_tx, ipc_rx) = unbounded();
        let events = Broadcaster::default();
        if let Err(e) = IpcServer::start(ipc_tx, events.clone()) {
            warn!("Pipe-Server startet nicht: {e}");
        }
        let (job_tx, job_rx) = unbounded();
        let (watch_tx, watch_rx) = unbounded();
        let worker_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("gameclip-worker.exe")))
            .unwrap_or_else(|| PathBuf::from("gameclip-worker.exe"));
        let jobs = Jobs::new(
            JobsConfig {
                worker_path,
                paths: paths.clone(),
                index: index.clone(),
                encoder_order: Vec::new(),
                adapter_luid: 0,
                silence_timeout: std::time::Duration::from_secs(WORKER_SILENCE_TIMEOUT_S),
            },
            job_tx,
        );
        let last_clip = index
            .latest_ok(FolderKind::Clips)
            .ok()
            .flatten()
            .map(|c| c.path);
        Ok(Service {
            settings,
            paths,
            index,
            ring: Arc::new(Ring::new(ring_cfg)),
            recorder: None,
            machine,
            msg,
            msg_rx,
            rec_tx,
            rec_rx,
            save_tx,
            save_rx,
            presses: VecDeque::new(),
            recovery: None,
            last_clip,
            ipc_rx,
            events,
            jobs,
            job_rx,
            sound: crate::sound::SoundPlayer::new(),
            toast_failed: false,
            copy_after: None,
            settings_read_only,
            conns: Default::default(),
            frames: Default::default(),
            watcher: None,
            watch_tx,
            watch_rx,
            reconcile_due: None,
            exit: false,
        })
    }

    fn run(&mut self) {
        let hotkey = self.settings.hotkey.clone();
        let registered = self.msg.register(&hotkey);
        match &registered {
            Ok(()) => info!("Hotkey registriert: {hotkey:?}"),
            Err(e) => warn!("Hotkey {hotkey:?} nicht registrierbar: {e}"),
        }
        self.feed(if registered.is_ok() {
            StateEvent::HotkeyRegistered
        } else {
            StateEvent::HotkeyFailed
        });
        for (dir, folder) in [
            (self.paths.clips.clone(), FolderKind::Clips),
            (self.paths.exports.clone(), FolderKind::Exports),
        ] {
            match crate::retention_exec::reconcile(
                &self.index,
                &self.paths,
                &dir,
                folder,
                &crate::retention_exec::WinProbe,
            ) {
                Ok(s) => info!("Abgleich {folder:?}: {s:?}"),
                Err(e) => warn!("Abgleich {folder:?}: {e}"),
            }
        }
        let started = self.start_recorder();
        self.feed(if started {
            StateEvent::PipelineReady
        } else {
            StateEvent::PipelineFailed
        });

        while !self.exit {
            let recovery = self.recovery.clone().unwrap_or_else(never);
            let debounce = match &self.reconcile_due {
                Some((_, at)) => crossbeam_channel::at(*at),
                None => never(),
            };
            select! {
                recv(self.msg_rx) -> ev => if let Ok(ev) = ev { self.on_msg(ev) },
                recv(self.rec_rx) -> ev => if let Ok(ev) = ev { self.on_recorder(ev) },
                recv(self.save_rx) -> r => if let Ok(r) = r { self.on_saved(r) },
                recv(self.ipc_rx) -> r => if let Ok(r) = r { self.on_ipc(r) },
                recv(self.job_rx) -> ev => if let Ok(ev) = ev { self.on_job(ev) },
                recv(self.watch_rx) -> f => if let Ok(folder) = f {
                    // 1 s entprellen (Design, Datei 46).
                    let at = std::time::Instant::now() + std::time::Duration::from_secs(1);
                    let due = self.reconcile_due.get_or_insert_with(|| (Default::default(), at));
                    due.0.insert(folder);
                    due.1 = at;
                },
                recv(debounce) -> _ => {
                    if let Some((folders, _)) = self.reconcile_due.take() {
                        for f in folders {
                            self.reconcile_folder(f);
                        }
                    }
                },
                recv(recovery) -> _ => {
                    self.recovery = None;
                    let ok = self.restart_recorder();
                    self.feed(if ok { StateEvent::RecoverySucceeded } else { StateEvent::RecoveryFailed });
                }
            }
        }
        if let Some(r) = self.recorder.take() {
            r.stop();
        }
        info!("gameclipd beendet");
    }

    fn feed(&mut self, ev: StateEvent) -> Vec<Effect> {
        let before = self.machine.state;
        let (next, effects) = transition(&self.machine, ev.clone());
        self.machine = next;
        if effects.is_empty() && before == next.state {
            tracing::debug!("Ereignis ohne Wirkung: {ev:?} in {before:?}");
        }
        for fx in &effects {
            self.apply(fx);
        }
        effects
    }

    fn press(&mut self, qpc_us: i64) {
        self.presses.push_back((qpc_us, foreground_program()));
        let fx = self.feed(StateEvent::Hotkey);
        if !fx
            .iter()
            .any(|f| matches!(f, Effect::BeginSave | Effect::QueueSave))
        {
            // Nicht angenommen (nicht bereit oder schon zwei in Arbeit).
            self.presses.pop_back();
            self.events.send(ServiceEvent::SaveFailed {
                code: ErrorCode::NotReady,
                message: format!("Nicht bereit ({:?})", self.machine.state),
            });
        }
    }

    fn on_msg(&mut self, ev: MsgEvent) {
        match ev {
            MsgEvent::Hotkey { qpc_us } => {
                info!("Hotkey gedrückt");
                self.press(qpc_us);
            }
            MsgEvent::SessionLocked => {
                self.feed(StateEvent::SessionLocked);
            }
            MsgEvent::SessionUnlocked => {
                self.feed(StateEvent::SessionUnlocked);
            }
            MsgEvent::Tray(cmd) => match cmd {
                TrayCommand::SaveClip => self.press(gameclip_capture_win::pacer::qpc_us()),
                TrayCommand::CopyLastClip => self.copy_last_clip(),
                TrayCommand::CopyLastClipSmall => self.copy_last_clip_small(),
                TrayCommand::TogglePause => {
                    let ev = if self.machine.state == ServiceState::Paused {
                        StateEvent::StartBuffer
                    } else {
                        StateEvent::StopBuffer
                    };
                    self.feed(ev);
                }
                TrayCommand::ToggleMic => {
                    let on = !self.machine.mic_paused;
                    self.feed(StateEvent::MicPause(on));
                }
                TrayCommand::Quit => {
                    self.feed(StateEvent::Quit);
                }
            },
        }
    }

    fn on_recorder(&mut self, ev: RecorderEvent) {
        match ev {
            RecorderEvent::DeviceLost(e) => {
                warn!("Gerät verloren: {e}");
                if let Some(r) = self.recorder.take() {
                    r.stop();
                }
                self.feed(StateEvent::DeviceLost);
            }
            RecorderEvent::HdrChanged(on) => {
                self.feed(StateEvent::HdrChanged(on));
            }
            RecorderEvent::AudioSourceFailed { track, error } => {
                warn!("Tonquelle {track:?} ausgefallen: {error}");
            }
            RecorderEvent::Levels { mic, .. } => {
                if let Some(dbfs) = mic {
                    self.events
                        .send_to(&self.conns.mic_level, ServiceEvent::MicLevel { dbfs });
                }
            }
            RecorderEvent::FormatChanged { width, height } => {
                self.events.send(ServiceEvent::FormatChanged {
                    width,
                    height,
                    hdr: self
                        .machine
                        .restrictions
                        .contains(gameclip_core::state::Restrictions::HDR_ACTIVE),
                });
            }
        }
    }

    fn on_saved(&mut self, r: Result<Saved, SaveError>) {
        match &r {
            Ok(s) => {
                info!(
                    "Clip gespeichert: {} ({} ms, {} Bytes{}{})",
                    s.path.display(),
                    s.duration_us / 1000,
                    s.size_bytes,
                    if s.truncated { ", verkürzt" } else { "" },
                    s.tracks_error
                        .as_ref()
                        .map(|e| format!(", ohne Einzelspuren: {e}"))
                        .unwrap_or_default()
                );
                self.last_clip = Some(s.path.clone());
                let text = format!(
                    "Clip gespeichert ({}, {})",
                    crate::toast::format_duration(s.duration_us),
                    crate::toast::format_size(s.size_bytes)
                );
                let detail = if s.truncated {
                    "Clip verkürzt (Aufnahme neu gestartet)".to_string()
                } else if s.tracks_error.is_some() {
                    "Ohne Einzelspuren".to_string()
                } else {
                    s.path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                };
                self.toast(&text, &detail, Some(s.row_id));
                self.enforce_limit(Some(s.row_id));
                self.events.send(ServiceEvent::NewClip {
                    clip_id: s.row_id,
                    folder: FolderKind::Clips,
                });
                self.events.send(ServiceEvent::SaveCompleted {
                    clip_id: s.row_id,
                    path: s.path.to_string_lossy().into_owned(),
                    duration_us: s.duration_us,
                    size_bytes: s.size_bytes,
                    has_tracks: s.has_tracks,
                    shortened: s.truncated,
                });
            }
            Err(e) => {
                warn!("Speichern fehlgeschlagen: {e:?}");
                self.events.send(ServiceEvent::SaveFailed {
                    code: ErrorCode::from(*e),
                    message: format!("{e:?}"),
                });
            }
        }
        self.feed(StateEvent::SaveFinished(r.map(|_| ())));
    }

    fn on_ipc(&mut self, r: IpcRequest) {
        match r {
            IpcRequest::Call {
                conn,
                id,
                cmd,
                reply,
            } => {
                let response = match self.handle(conn, cmd) {
                    Ok(body) => Response::ok(id, &body),
                    Err(e) => {
                        let mut r = Response::err(id, e.code, e.message);
                        if let Some(body) = r.error.as_mut() {
                            body.details = e.details;
                        }
                        r
                    }
                };
                let _ = reply.send(response);
            }
            IpcRequest::Disconnected(conn) => self.disconnected(conn),
        }
    }

    /// Letzten Clip auf die Zielgröße exportieren und danach in die Zwischenablage legen.
    fn copy_last_clip_small(&mut self) {
        let Some(clip) = self.index.latest_ok(FolderKind::Clips).ok().flatten() else {
            info!("Noch kein Clip zum Kopieren");
            return;
        };
        let bytes = self.settings.export.target_bytes;
        let job = self.jobs.start(
            clip.clone(),
            0,
            clip.duration_us,
            ExportProfile::TargetSize { bytes },
        );
        info!("Export {job} für die Zwischenablage gestartet ({bytes} Bytes)");
        self.copy_after = Some((job, std::time::Instant::now()));
    }

    fn on_job(&mut self, ev: JobEvent) {
        self.events.send(ev.to_service_event());
        match &ev {
            JobEvent::Progress {
                job_id, percent, ..
            } => self.jobs.progress(*job_id, *percent),
            JobEvent::Completed { clip_id, .. } => self.events.send(ServiceEvent::NewClip {
                clip_id: *clip_id,
                folder: FolderKind::Exports,
            }),
            JobEvent::Failed { .. } => {}
        }
        match ev {
            JobEvent::Completed {
                job_id, clip_id, ..
            } if !self.copy_after.is_some_and(|(id, _)| id == job_id) => {
                self.enforce_limit(Some(clip_id));
            }
            JobEvent::Completed {
                job_id,
                clip_id,
                path,
            } if self.copy_after.is_some_and(|(id, _)| id == job_id) => {
                self.enforce_limit(Some(clip_id));
                let (_, started) = self.copy_after.take().expect("geprüft");
                match crate::clipboard::copy_file(&path) {
                    Ok(()) => info!(
                        "G1a: Tray-Aktion bis Zwischenablage {} ms ({})",
                        started.elapsed().as_millis(),
                        path.display()
                    ),
                    Err(e) => warn!("Zwischenablage: {e}"),
                }
            }
            JobEvent::Failed { job_id, .. }
                if self.copy_after.is_some_and(|(id, _)| id == job_id) =>
            {
                self.copy_after = None;
            }
            _ => {}
        }
    }

    fn toast(&mut self, title: &str, text: &str, reveal: Option<u32>) {
        if let Err(e) = crate::toast::show(title, text, reveal) {
            // Ohne Paket-Identität verwirft Windows Toasts; einmal melden genügt.
            if !self.toast_failed {
                warn!("Benachrichtigung nicht anzeigbar (ohne MSIX erwartet): {e}");
                self.toast_failed = true;
            }
        }
    }

    fn enforce_limit(&mut self, protect: Option<u32>) {
        match crate::retention_exec::enforce(
            &self.index,
            &self.paths,
            &self.settings.storage,
            protect,
            &crate::retention_exec::WinProbe,
        ) {
            Ok(done) => {
                for (folders, reason) in done.warnings {
                    for folder in folders {
                        let limit_bytes = match folder {
                            FolderKind::Clips => self.settings.storage.clip_limit_bytes,
                            FolderKind::Exports => self.settings.storage.export_limit_bytes,
                        };
                        let used_bytes = self
                            .index
                            .list(folder)
                            .map(|rows| {
                                rows.iter()
                                    .filter(|r| {
                                        r.status == gameclip_core::retention::ClipStatus::Ok
                                            && !r.placeholder
                                    })
                                    .map(|r| r.size_bytes)
                                    .sum()
                            })
                            .unwrap_or(0);
                        self.events.send(ServiceEvent::StorageWarning {
                            folder,
                            used_bytes,
                            limit_bytes,
                            reason,
                        });
                    }
                }
                for d in done.deleted {
                    self.events.send(ServiceEvent::ClipDeleted {
                        clip_id: d.row_id,
                        folder: d.folder,
                    });
                }
            }
            Err(e) => warn!("Speicherlimit: {e}"),
        }
    }

    fn copy_last_clip(&mut self) {
        let path = self
            .index
            .latest_ok(FolderKind::Clips)
            .ok()
            .flatten()
            .map(|c| c.path)
            .or_else(|| self.last_clip.clone());
        match path {
            Some(p) => match crate::clipboard::copy_file(&p) {
                Ok(()) => info!("In die Zwischenablage kopiert: {}", p.display()),
                Err(e) => warn!("Zwischenablage: {e}"),
            },
            None => info!("Noch kein Clip zum Kopieren"),
        }
    }

    fn recorder_config(&self) -> RecorderConfig {
        RecorderConfig::from_settings(&self.settings, std::process::id(), physical_ram())
    }

    fn start_recorder(&mut self) -> bool {
        let cfg = self.recorder_config();
        match Recorder::start(cfg.video, cfg.audio, self.ring.clone(), self.rec_tx.clone()) {
            Ok(r) => {
                let (encoder, hardware) = r.encoder();
                info!("Aufnahme läuft: Encoder {encoder} (Hardware: {hardware})");
                let hdr = r.hdr_active();
                self.recorder = Some(r);
                self.feed(StateEvent::CpuEncoderOnly(!hardware));
                self.feed(StateEvent::HdrChanged(hdr));
                if self.machine.mic_paused {
                    self.apply(&Effect::SetMicPaused(true));
                }
                true
            }
            Err(e) => {
                error!("Aufnahme startet nicht: {e}");
                false
            }
        }
    }

    /// Neustart im selben Ring: neuer Abschnitt.
    fn restart_recorder(&mut self) -> bool {
        if let Some(r) = self.recorder.take() {
            r.stop();
        }
        self.ring.new_section();
        self.start_recorder()
    }

    fn apply(&mut self, fx: &Effect) {
        match fx {
            Effect::StartPipeline => {
                let ok = self.restart_recorder();
                let ev = if ok {
                    StateEvent::PipelineReady
                } else {
                    StateEvent::PipelineFailed
                };
                self.feed(ev);
            }
            Effect::StopPipeline => {
                if let Some(r) = self.recorder.take() {
                    r.stop();
                }
            }
            Effect::RebuildBuffer => {
                if let Some(r) = self.recorder.take() {
                    r.stop();
                }
                self.ring = Arc::new(Ring::new(self.recorder_config().ring));
                let ok = self.start_recorder();
                self.feed(if ok {
                    StateEvent::PipelineReady
                } else {
                    StateEvent::PipelineFailed
                });
            }
            Effect::BeginSave => self.begin_save(),
            Effect::QueueSave => {}
            Effect::PlaySound(s) => {
                if self.settings.sound.enabled {
                    // Ausgeschlossen ist der eigene Prozessbaum: der Ton landet nicht im Clip (D26).
                    // Mit Discord-Ausschluss (M2) kommt hier `mute_system` für das Fenster dazu.
                    let _ = self.sound.play(*s, self.settings.sound.volume);
                }
            }
            Effect::Notify(n) => {
                let (title, text) = notice_text(n);
                self.toast(title, &text, None);
            }
            Effect::ScheduleRecovery(d) => {
                self.recovery = Some(crossbeam_channel::after(*d));
            }
            Effect::SetMicPaused(on) => {
                if let Some(a) = self.recorder.as_ref().and_then(|r| r.audio()) {
                    a.set_mic_paused(*on);
                }
            }
            Effect::ReRegisterHotkey => {
                let hk = self.settings.hotkey.clone();
                let ev = if self.msg.register(&hk).is_ok() {
                    StateEvent::HotkeyRegistered
                } else {
                    StateEvent::HotkeyFailed
                };
                self.feed(ev);
            }
            Effect::UpdateTray => {
                self.msg
                    .update_tray(view(&self.machine, self.settings.export.target_bytes));
                self.events.send(ServiceEvent::StateChanged(self.status()));
            }
            Effect::Exit => self.exit = true,
        }
    }

    fn begin_save(&mut self) {
        let Some((pressed_us, program)) = self.presses.pop_front() else {
            return;
        };
        let Some(rec) = self.recorder.as_ref() else {
            self.feed(StateEvent::SaveFinished(Err(SaveError::BufferEmpty)));
            return;
        };
        let job = SaveJob {
            ring: self.ring.clone(),
            video_params: rec.video_params().clone(),
            audio_params: rec.audio_params().to_vec(),
            buffer_us: i64::from(self.settings.capture.buffer_seconds) * 1_000_000,
            fps: self.settings.capture.fps,
            paths: self.paths.clone(),
            index: self.index.clone(),
            request: SaveRequest {
                pressed_us,
                program,
                now_local: OffsetDateTime::now_local()
                    .unwrap_or_else(|_| OffsetDateTime::now_utc()),
            },
        };
        if self.save_tx.send(job).is_err() {
            self.feed(StateEvent::SaveFinished(Err(SaveError::EncoderFailed)));
        }
    }
}

fn notice_text(n: &gameclip_core::state::Notice) -> (&'static str, String) {
    use gameclip_core::state::Notice;
    match n {
        Notice::SaveFailed(e) => (
            "Clip nicht gespeichert",
            match e {
                SaveError::DiskFull => "Kein Platz auf dem Laufwerk".into(),
                SaveError::FolderMissing => "Clip-Ordner fehlt oder ist schreibgeschützt".into(),
                SaveError::BufferEmpty => "Noch nichts aufgenommen".into(),
                SaveError::EncoderFailed => "Fehler beim Schreiben".into(),
                SaveError::Timeout => "Aufnahme hängt".into(),
            },
        ),
        Notice::HotkeyInactive => (
            "Hotkey inaktiv",
            "Die Taste ist von einem anderen Programm belegt".into(),
        ),
        Notice::CpuEncoder => ("Aufnahme über CPU", "Kein Hardware-Encoder nutzbar".into()),
        Notice::Failed => (
            "Aufnahme gestoppt",
            "GameClip konnte die Aufnahme nicht starten".into(),
        ),
        Notice::NotReady => ("Noch nicht bereit", "Die Aufnahme startet gerade".into()),
    }
}

fn view(m: &Machine, target_bytes: u64) -> TrayView {
    use gameclip_core::state::Restrictions;
    let restricted = !m.restrictions.is_empty();
    let state = match m.state {
        ServiceState::Starting | ServiceState::Reconfiguring => TrayState::Starting,
        ServiceState::Buffering if m.saves_in_flight > 0 => TrayState::Saving,
        ServiceState::Buffering if restricted => TrayState::Restricted,
        ServiceState::Buffering => TrayState::Buffering,
        ServiceState::Recovering { .. } => TrayState::Starting,
        ServiceState::Paused => TrayState::Paused,
        ServiceState::Failed => TrayState::Failed,
    };
    let mut lines = vec![match m.state {
        ServiceState::Starting => "GameClip startet".to_string(),
        ServiceState::Buffering => "GameClip nimmt auf".into(),
        ServiceState::Paused if m.paused_by_lock => "Pausiert (PC gesperrt)".into(),
        ServiceState::Paused => "Pausiert".into(),
        ServiceState::Reconfiguring => "Puffer wird zurückgesetzt".into(),
        ServiceState::Recovering { attempt } => format!("Erholt sich (Versuch {})", attempt + 1),
        ServiceState::Failed => "Fehler – Aufnahme gestoppt".into(),
    }];
    if m.restrictions.contains(Restrictions::HOTKEY_INACTIVE) {
        lines.push("Hotkey inaktiv".into());
    }
    if m.restrictions.contains(Restrictions::CPU_ENCODER) {
        lines.push("CPU-Encoder".into());
    }
    if m.restrictions.contains(Restrictions::HDR_ACTIVE) {
        lines.push("HDR wird nicht unterstützt".into());
    }
    if m.mic_paused {
        lines.push("Mikrofon pausiert".into());
    }
    TrayView {
        state,
        tooltip: lines.join("\n"),
        paused: m.state == ServiceState::Paused,
        mic_paused: m.mic_paused,
        target_mb: target_bytes.div_ceil(1_000_000),
    }
}
