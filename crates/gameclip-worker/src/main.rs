// SPDX-License-Identifier: GPL-3.0-or-later
//! `gameclip-worker.exe`: Auftrag als JSON über stdin, Meldungen als JSON-Zeilen über stdout,
//! Exit 0 bei Erfolg, 1 bei Fehler (Details in der letzten Meldung).

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    use std::io::{Read, Write};

    use gameclip_protocol::worker::{WorkerError, WorkerJob, WorkerMessage};

    let mut emit = |m: WorkerMessage| {
        let mut out = std::io::stdout().lock();
        if let Ok(line) = serde_json::to_string(&m) {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        }
    };
    let mut input = String::new();
    let job: WorkerJob = match std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| e.to_string())
        .and_then(|_| serde_json::from_str(&input).map_err(|e| e.to_string()))
    {
        Ok(j) => j,
        Err(e) => {
            emit(WorkerMessage::Error {
                code: WorkerError::InputMissing,
                message: format!("Auftrag unlesbar: {e}"),
            });
            std::process::exit(1);
        }
    };
    let started = std::time::Instant::now();
    let result = gameclip_worker::trim::run(&job, &mut emit);
    if std::env::var_os("GAMECLIP_WORKER_TIMING").is_some() {
        eprintln!("{}", gameclip_worker::timing::report());
        eprintln!(
            "gesamt           {:>8.1} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    match result {
        Ok(out) => {
            emit(WorkerMessage::Done {
                bytes: out.bytes,
                duration_us: out.duration_us,
            });
            std::process::exit(0);
        }
        Err(gameclip_worker::trim::Fail(code, message)) => {
            let _ = std::fs::remove_file(&job.target_part_path);
            emit(WorkerMessage::Error { code, message });
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {}
