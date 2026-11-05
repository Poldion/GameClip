// SPDX-License-Identifier: GPL-3.0-or-later
//! Export-Aufträge an `gameclip-worker` (Design, Datei 49).
//!
//! Warteschlange auf einem eigenen Thread, höchstens ein laufender Auftrag. Je Auftrag:
//! Ausgabename (`export_file_name` + `dedupe` + `.part`), Index-Eintrag `pending`,
//! Zielgröße vorab prüfen (`TooSmall` → `TooLarge` ohne Worker), Worker mit
//! `BELOW_NORMAL_PRIORITY_CLASS` in einem Job-Objekt mit `KILL_ON_JOB_CLOSE` starten, Auftrag
//! als JSON über stdin, Meldungen von stdout. Ohne Meldung für `silence_timeout` wird der Worker
//! beendet. Erfolg: umbenennen, `ok`. Fehler oder Abbruch: `.part` und Eintrag löschen.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use gameclip_core::export_plan::plan;
use gameclip_core::naming::{ExportSuffix, dedupe, export_file_name};
use gameclip_core::retention::FolderKind;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::messages::{ExportProfile, JobInfo, JobState, ServiceEvent};
use gameclip_protocol::worker::{WorkerError, WorkerJob, WorkerMessage};
use time::OffsetDateTime;
use tracing::{info, warn};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};

use crate::index::{ClipRow, Index, NewClip};
use crate::paths::Paths;

pub const WORKER_SILENCE_TIMEOUT_S: u64 = 30;
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent {
    Progress {
        job_id: u32,
        percent: f32,
        encoder: String,
    },
    Completed {
        job_id: u32,
        clip_id: u32,
        path: PathBuf,
    },
    Failed {
        job_id: u32,
        code: ErrorCode,
        message: String,
    },
}

impl JobEvent {
    pub fn job_id(&self) -> u32 {
        match self {
            JobEvent::Progress { job_id, .. }
            | JobEvent::Completed { job_id, .. }
            | JobEvent::Failed { job_id, .. } => *job_id,
        }
    }

    pub fn to_service_event(&self) -> ServiceEvent {
        match self.clone() {
            JobEvent::Progress {
                job_id,
                percent,
                encoder,
            } => ServiceEvent::JobProgress {
                job_id,
                percent,
                encoder,
            },
            JobEvent::Completed {
                job_id, clip_id, ..
            } => ServiceEvent::JobCompleted { job_id, clip_id },
            JobEvent::Failed {
                job_id,
                code,
                message,
            } => ServiceEvent::JobFailed {
                job_id,
                code,
                message,
            },
        }
    }
}

struct Request {
    job_id: u32,
    paths: Paths,
    clip: ClipRow,
    in_us: u64,
    out_us: u64,
    profile: ExportProfile,
}

pub struct JobsConfig {
    pub worker_path: PathBuf,
    pub paths: Paths,
    pub index: Arc<Index>,
    /// Reihenfolge der Encoder aus der Prüfung des Dienstes (leer = Worker entscheidet).
    pub encoder_order: Vec<String>,
    pub adapter_luid: u64,
    pub silence_timeout: Duration,
}

pub struct Jobs {
    tx: Sender<Request>,
    next_id: Mutex<u32>,
    cancelled: Arc<Mutex<HashSet<u32>>>,
    /// Wartende und laufender Auftrag für `ListJobs`.
    list: Arc<Mutex<Vec<JobInfo>>>,
    /// Aktuelle Ordner; jeder Auftrag nimmt sie beim Einreihen mit.
    paths: Mutex<Paths>,
}

impl Jobs {
    pub fn new(cfg: JobsConfig, events: Sender<JobEvent>) -> Self {
        let (tx, rx) = unbounded();
        let cancelled = Arc::new(Mutex::new(HashSet::new()));
        let list = Arc::new(Mutex::new(Vec::new()));
        let (c, l) = (cancelled.clone(), list.clone());
        let paths = Mutex::new(cfg.paths.clone());
        std::thread::Builder::new()
            .name("gameclip-jobs".into())
            .spawn(move || runner(cfg, rx, events, c, l))
            .expect("Auftrags-Thread startet");
        Jobs {
            tx,
            next_id: Mutex::new(1),
            cancelled,
            list,
            paths,
        }
    }

    /// Reiht einen Export ein und liefert die `job_id`.
    pub fn start(&self, clip: ClipRow, in_us: u64, out_us: u64, profile: ExportProfile) -> u32 {
        let job_id = {
            let mut n = self.next_id.lock().expect("Mutex");
            let id = *n;
            *n += 1;
            id
        };
        if let Ok(mut l) = self.list.lock() {
            l.push(JobInfo {
                job_id,
                clip_id: clip.row_id,
                profile,
                state: JobState::Queued,
                percent: 0.0,
            });
        }
        let paths = self.paths.lock().map(|p| p.clone()).expect("Mutex");
        let _ = self.tx.send(Request {
            job_id,
            paths,
            clip,
            in_us,
            out_us,
            profile,
        });
        job_id
    }

    /// Neue Ordner (nach `SetSettings`) für künftige Aufträge.
    pub fn set_paths(&self, paths: Paths) {
        if let Ok(mut p) = self.paths.lock() {
            *p = paths;
        }
    }

    /// Fortschritt für `ListJobs` (aus den Ereignissen).
    pub fn progress(&self, job_id: u32, percent: f32) {
        if let Ok(mut l) = self.list.lock()
            && let Some(j) = l.iter_mut().find(|j| j.job_id == job_id)
        {
            j.percent = percent;
        }
    }

    /// Laufender und wartende Aufträge.
    pub fn list(&self) -> Vec<JobInfo> {
        self.list.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// Bricht einen wartenden oder laufenden Auftrag ab.
    pub fn cancel(&self, job_id: u32) {
        if let Ok(mut c) = self.cancelled.lock() {
            c.insert(job_id);
        }
    }
}

fn worker_error_code(e: WorkerError) -> ErrorCode {
    match e {
        WorkerError::InputMissing => ErrorCode::NotFound,
        WorkerError::DecodeFailed | WorkerError::EncoderFailed => ErrorCode::EncoderFailed,
        WorkerError::TooLarge => ErrorCode::TooLarge,
        WorkerError::Io => ErrorCode::DiskFull,
    }
}

fn runner(
    cfg: JobsConfig,
    rx: Receiver<Request>,
    events: Sender<JobEvent>,
    cancelled: Arc<Mutex<HashSet<u32>>>,
    list: Arc<Mutex<Vec<JobInfo>>>,
) {
    let set = |job_id: u32, f: &dyn Fn(&mut JobInfo)| {
        if let Ok(mut l) = list.lock()
            && let Some(j) = l.iter_mut().find(|j| j.job_id == job_id)
        {
            f(j);
        }
    };
    for req in rx {
        let job_id = req.job_id;
        let is_cancelled = || {
            cancelled
                .lock()
                .map(|c| c.contains(&job_id))
                .unwrap_or(false)
        };
        if is_cancelled() {
            let _ = events.send(JobEvent::Failed {
                job_id,
                code: ErrorCode::Busy,
                message: "abgebrochen".into(),
            });
            continue;
        }
        let started = Instant::now();
        set(job_id, &|j| j.state = JobState::Running);
        let result = run_job(&cfg, &req, &events, &is_cancelled);
        if let Ok(mut l) = list.lock() {
            l.retain(|j| j.job_id != job_id);
        }
        let ev = match result {
            Ok((clip_id, path)) => {
                info!(
                    "Export {job_id} fertig in {} ms: {}",
                    started.elapsed().as_millis(),
                    path.display()
                );
                JobEvent::Completed {
                    job_id,
                    clip_id,
                    path,
                }
            }
            Err((code, message)) => {
                warn!("Export {job_id} fehlgeschlagen: {code:?} {message}");
                JobEvent::Failed {
                    job_id,
                    code,
                    message,
                }
            }
        };
        let _ = events.send(ev);
    }
}

fn part_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

struct JobHandle(HANDLE);

impl Drop for JobHandle {
    fn drop(&mut self) {
        // Schließen beendet den Worker, falls er noch läuft (KILL_ON_JOB_CLOSE).
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

fn kill_on_close_job() -> windows::core::Result<JobHandle> {
    unsafe {
        let job = CreateJobObjectW(None, None)?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const _,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )?;
        Ok(JobHandle(job))
    }
}

type JobResult = Result<(u32, PathBuf), (ErrorCode, String)>;

fn run_job(
    cfg: &JobsConfig,
    req: &Request,
    events: &Sender<JobEvent>,
    is_cancelled: &dyn Fn() -> bool,
) -> JobResult {
    let source = &req.clip.path;
    if !source.is_file() {
        return Err((ErrorCode::NotFound, source.display().to_string()));
    }
    // Zielgröße vorab: zu klein → ohne Worker.
    if let ExportProfile::TargetSize { bytes } = req.profile {
        let info = gameclip_media::demux::video_info(source)
            .map_err(|e| (ErrorCode::EncoderFailed, e.to_string()))?;
        if let Err(e) = plan(
            req.out_us.saturating_sub(req.in_us),
            bytes,
            (info.width, info.height, info.fps),
        ) {
            return Err((ErrorCode::TooLarge, format!("{e:?}")));
        }
    }

    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "clip".into());
    let suffix = match req.profile {
        ExportProfile::Mix => ExportSuffix::Trim,
        ExportProfile::SeparateTracks => ExportSuffix::Tracks,
        ExportProfile::TargetSize { bytes } => ExportSuffix::TargetSize { bytes },
    };
    std::fs::create_dir_all(&req.paths.exports)
        .map_err(|e| (ErrorCode::FolderMissing, e.to_string()))?;
    let name = dedupe(&export_file_name(&stem, suffix), |n| {
        let p = req.paths.exports.join(n);
        p.exists() || part_path(&p).exists()
    });
    let final_path = req.paths.exports.join(name);
    let part = part_path(&final_path);
    let tag_id = uuid::Uuid::new_v4().to_string();
    let row_id = cfg
        .index
        .insert_pending(&NewClip {
            tag_id: &tag_id,
            folder: FolderKind::Exports,
            path: &final_path,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: Some(req.clip.row_id),
        })
        .map_err(|e| (ErrorCode::Internal, e.to_string()))?;

    let result = run_worker(cfg, req, &tag_id, &part, events, is_cancelled).and_then(
        |(bytes, duration_us)| {
            std::fs::rename(&part, &final_path)
                .map_err(|e| (ErrorCode::DiskFull, e.to_string()))?;
            cfg.index
                .mark_ok(row_id, &final_path, bytes, duration_us, false)
                .map_err(|e| (ErrorCode::Internal, e.to_string()))?;
            Ok((row_id, final_path.clone()))
        },
    );
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
        let _ = cfg.index.remove(row_id);
    }
    result
}

fn run_worker(
    cfg: &JobsConfig,
    req: &Request,
    tag_id: &str,
    part: &Path,
    events: &Sender<JobEvent>,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<(u64, u64), (ErrorCode, String)> {
    let tracks = req.paths.track_file(&req.clip.tag_id);
    let job = WorkerJob {
        source: req.clip.path.to_string_lossy().into_owned(),
        tracks_path: tracks
            .is_file()
            .then(|| tracks.to_string_lossy().into_owned()),
        target_part_path: part.to_string_lossy().into_owned(),
        output_tag_id: tag_id.to_string(),
        in_us: req.in_us,
        out_us: req.out_us,
        profile: req.profile,
        encoder_order: cfg.encoder_order.clone(),
        adapter_luid: cfg.adapter_luid,
    };
    let internal = |e: std::io::Error| (ErrorCode::Internal, e.to_string());
    let job_object = kill_on_close_job().map_err(|e| (ErrorCode::Internal, e.to_string()))?;
    let mut child = Command::new(&cfg.worker_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW)
        .spawn()
        .map_err(internal)?;
    unsafe {
        let _ = AssignProcessToJobObject(job_object.0, HANDLE(child.as_raw_handle()));
    }
    {
        let mut stdin = child.stdin.take().expect("stdin");
        let text = serde_json::to_string(&job).expect("Auftrag serialisierbar");
        stdin.write_all(text.as_bytes()).map_err(internal)?;
    }
    let stdout = child.stdout.take().expect("stdout");
    let (line_tx, line_rx) = unbounded();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });

    let mut encoder = String::new();
    let mut done = None;
    let mut error = None;
    let mut last_message = Instant::now();
    loop {
        if is_cancelled() {
            unsafe {
                let _ = TerminateJobObject(job_object.0, 1);
            }
            let _ = child.wait();
            return Err((ErrorCode::Busy, "abgebrochen".into()));
        }
        match line_rx.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                last_message = Instant::now();
                match serde_json::from_str::<WorkerMessage>(&line) {
                    Ok(WorkerMessage::Encoder { name }) => encoder = name,
                    Ok(WorkerMessage::Progress { percent }) => {
                        let _ = events.send(JobEvent::Progress {
                            job_id: req.job_id,
                            percent,
                            encoder: encoder.clone(),
                        });
                    }
                    Ok(WorkerMessage::Done { bytes, duration_us }) => {
                        done = Some((bytes, duration_us))
                    }
                    Ok(WorkerMessage::Error { code, message }) => error = Some((code, message)),
                    Err(e) => warn!("Unlesbare Meldung des Workers: {e}: {line}"),
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if last_message.elapsed() >= cfg.silence_timeout {
                    unsafe {
                        let _ = TerminateJobObject(job_object.0, 1);
                    }
                    let _ = child.wait();
                    return Err((ErrorCode::Timeout, "Worker antwortet nicht".into()));
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let status = child.wait().map_err(internal)?;
    match (status.code(), done, error) {
        (Some(0), Some(d), _) => Ok(d),
        (Some(1), _, Some((code, message))) => Err((worker_error_code(code), message)),
        (code, _, _) => Err((
            ErrorCode::Internal,
            format!("Worker abgestürzt (Exit {code:?})"),
        )),
    }
}
