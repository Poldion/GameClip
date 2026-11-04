// SPDX-License-Identifier: GPL-3.0-or-later
//! Auftrags-Warteschlange mit `fake-worker` (Design, Datei 55). Nur mit `--features test-support`.
#![cfg(all(windows, feature = "test-support"))]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::Receiver;
use gameclip_core::retention::{ClipStatus, FolderKind};
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::messages::ExportProfile;
use gameclipd::index::{ClipRow, Index, NewClip};
use gameclipd::jobs::{JobEvent, Jobs, JobsConfig};
use gameclipd::paths::Paths;
use time::OffsetDateTime;

struct Env {
    dir: tempfile::TempDir,
    index: Arc<Index>,
    jobs: Jobs,
    events: Receiver<JobEvent>,
    paths: Paths,
}

fn env(silence: Duration) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths {
        clips: dir.path().join("clips"),
        exports: dir.path().join("exports"),
        tracks: dir.path().join("tracks"),
        index: dir.path().join("index.sqlite"),
        settings: dir.path().join("settings.toml"),
        logs: dir.path().join("logs"),
    };
    paths.create_all().unwrap();
    let index = Arc::new(Index::open(&paths.index).unwrap());
    let (tx, events) = crossbeam_channel::unbounded();
    let jobs = Jobs::new(
        JobsConfig {
            worker_path: PathBuf::from(env!("CARGO_BIN_EXE_fake-worker")),
            paths: paths.clone(),
            index: index.clone(),
            encoder_order: Vec::new(),
            adapter_luid: 0,
            silence_timeout: silence,
        },
        tx,
    );
    Env {
        dir,
        index,
        jobs,
        events,
        paths,
    }
}

/// Legt eine Quelldatei mit diesem Namen im Clip-Ordner und ihren Index-Eintrag an.
fn clip(e: &Env, name: &str) -> ClipRow {
    let path = e.paths.clips.join(name);
    std::fs::write(&path, b"quelle").unwrap();
    let row = e
        .index
        .insert_pending(&NewClip {
            tag_id: "t",
            folder: FolderKind::Clips,
            path: &path,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: None,
        })
        .unwrap();
    e.index.mark_ok(row, &path, 6, 5_000_000, false).unwrap();
    e.index.get(row).unwrap().unwrap()
}

fn finish(e: &Env) -> (Vec<f32>, JobEvent) {
    let mut progress = Vec::new();
    loop {
        match e
            .events
            .recv_timeout(Duration::from_secs(20))
            .expect("Ereignis")
        {
            JobEvent::Progress { percent, .. } => progress.push(percent),
            other => return (progress, other),
        }
    }
}

fn exports(e: &Env) -> Vec<PathBuf> {
    std::fs::read_dir(&e.paths.exports)
        .unwrap()
        .map(|d| d.unwrap().path())
        .collect()
}

#[test]
fn success_renames_and_marks_ok() {
    let e = env(Duration::from_secs(5));
    let c = clip(&e, "ok_clip.mp4");
    let id = e.jobs.start(c.clone(), 0, 2_000_000, ExportProfile::Mix);
    let (progress, done) = finish(&e);
    let JobEvent::Completed {
        job_id,
        clip_id,
        path,
    } = done
    else {
        panic!("{done:?}")
    };
    assert_eq!(job_id, id);
    assert!(!progress.is_empty());
    assert!(path.ends_with("ok_clip_trim.mp4"));
    assert_eq!(std::fs::read(&path).unwrap(), b"export");
    let row = e.index.get(clip_id).unwrap().unwrap();
    assert_eq!(row.status, ClipStatus::Ok);
    assert_eq!(row.folder, FolderKind::Exports);
    assert_eq!(row.source_row_id, Some(c.row_id));
    // Zweiter Export desselben Clips: Name mit _2.
    e.jobs.start(c, 0, 1_000_000, ExportProfile::Mix);
    let (_, done) = finish(&e);
    assert!(
        matches!(done, JobEvent::Completed { ref path, .. } if path.ends_with("ok_clip_trim_2.mp4")),
        "{done:?}"
    );
}

#[test]
fn failure_removes_part_and_entry() {
    let e = env(Duration::from_secs(5));
    let c = clip(&e, "fail_clip.mp4");
    e.jobs.start(c, 0, 1_000_000, ExportProfile::Mix);
    let (_, done) = finish(&e);
    assert!(
        matches!(
            done,
            JobEvent::Failed {
                code: ErrorCode::EncoderFailed,
                ..
            }
        ),
        "{done:?}"
    );
    assert!(exports(&e).is_empty());
    assert!(e.index.list(FolderKind::Exports).unwrap().is_empty());
}

#[test]
fn silence_hits_the_timeout() {
    let e = env(Duration::from_secs(1));
    let c = clip(&e, "silent_clip.mp4");
    let started = std::time::Instant::now();
    e.jobs.start(c, 0, 1_000_000, ExportProfile::Mix);
    let (_, done) = finish(&e);
    assert!(
        matches!(
            done,
            JobEvent::Failed {
                code: ErrorCode::Timeout,
                ..
            }
        ),
        "{done:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "Worker beendet statt 60 s gewartet"
    );
    assert!(exports(&e).is_empty());
}

#[test]
fn crash_is_reported() {
    let e = env(Duration::from_secs(5));
    let c = clip(&e, "crash_clip.mp4");
    e.jobs.start(c, 0, 1_000_000, ExportProfile::Mix);
    let (_, done) = finish(&e);
    assert!(
        matches!(
            done,
            JobEvent::Failed {
                code: ErrorCode::Internal,
                ..
            }
        ),
        "{done:?}"
    );
}

#[test]
fn cancel_stops_a_running_job() {
    let e = env(Duration::from_secs(5));
    let c = clip(&e, "slow_clip.mp4");
    let id = e.jobs.start(c, 0, 1_000_000, ExportProfile::Mix);
    // Warten, bis er läuft, dann abbrechen.
    let first = e.events.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(matches!(first, JobEvent::Progress { .. }));
    e.jobs.cancel(id);
    let (_, done) = finish(&e);
    assert!(
        matches!(
            done,
            JobEvent::Failed {
                code: ErrorCode::Busy,
                ..
            }
        ),
        "{done:?}"
    );
    assert!(exports(&e).is_empty());
    assert!(e.index.list(FolderKind::Exports).unwrap().is_empty());
}

#[test]
fn too_small_target_fails_without_worker() {
    let e = env(Duration::from_secs(5));
    // Echte kleine MP4, damit die Videodaten für den Plan lesbar sind.
    let path = e.paths.clips.join("fail_but_never_started.mp4");
    write_tiny_mp4(&path);
    let row = e
        .index
        .insert_pending(&NewClip {
            tag_id: "t",
            folder: FolderKind::Clips,
            path: &path,
            created_at: OffsetDateTime::now_utc(),
            source_row_id: None,
        })
        .unwrap();
    e.index.mark_ok(row, &path, 0, 1_000_000, false).unwrap();
    let c = e.index.get(row).unwrap().unwrap();
    e.jobs.start(
        c,
        0,
        60_000_000,
        ExportProfile::TargetSize { bytes: 100_000 },
    );
    let (progress, done) = finish(&e);
    // TooLarge statt EncoderFailed zeigt: der Worker (der „fail“ melden würde) lief nie.
    assert!(
        matches!(
            done,
            JobEvent::Failed {
                code: ErrorCode::TooLarge,
                ..
            }
        ),
        "{done:?}"
    );
    assert!(progress.is_empty());
    let _ = &e.dir;
}

fn write_tiny_mp4(path: &Path) {
    use gameclip_media::mux::{Track, write_mp4};
    use gameclip_media::{EncodeProfile, EncoderConfig, VideoEncoder};
    let cfg = EncoderConfig {
        width: 320,
        height: 240,
        fps: 30,
        bitrate_bps: 500_000,
        profile: EncodeProfile::Capture,
    };
    let mut enc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let mut pk = Vec::new();
    for i in 0..30 {
        let mut f = gameclip_testsupport::synth::nv12_frame(i, 320, 240);
        enc.send(&mut f, i64::from(i) * 33_333, false).unwrap();
        while let Some(p) = enc.receive().unwrap() {
            pk.push(p);
        }
    }
    enc.flush().unwrap();
    while let Some(p) = enc.receive().unwrap() {
        pk.push(p);
    }
    let par = enc.codec_parameters();
    write_mp4(
        path,
        &Track {
            params: &par,
            time_base: rsmpeg::ffi::AVRational {
                num: 1,
                den: 1_000_000,
            },
            packets: &pk,
            title: "Video",
            default: true,
        },
        &[],
        &[],
        0,
        false,
    )
    .unwrap();
}
