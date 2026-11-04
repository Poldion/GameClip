// SPDX-License-Identifier: GPL-3.0-or-later
//! Attrappe für `gameclip-worker` in den Auftrags-Tests. Das Verhalten steuert der Dateiname
//! der Quelle (die `output_tag_id` vergibt der Dienst selbst): `fail…` meldet einen Fehler,
//! `silent…` schweigt, `crash…` endet mit Exit 3, `slow…` meldet 4 s lang Fortschritt, alles
//! andere schreibt eine kleine Datei und meldet `Done`.

use std::io::{Read, Write};
use std::path::Path;

use gameclip_protocol::worker::{WorkerError, WorkerJob, WorkerMessage};

fn emit(m: &WorkerMessage) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", serde_json::to_string(m).unwrap());
    let _ = out.flush();
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let job: WorkerJob = serde_json::from_str(&input).unwrap();
    let name = Path::new(&job.source)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    emit(&WorkerMessage::Encoder {
        name: "fake".into(),
    });
    if name.starts_with("fail") {
        emit(&WorkerMessage::Error {
            code: WorkerError::EncoderFailed,
            message: "absichtlich".into(),
        });
        std::process::exit(1);
    }
    if name.starts_with("silent") {
        std::thread::sleep(std::time::Duration::from_secs(60));
        std::process::exit(0);
    }
    if name.starts_with("crash") {
        std::process::exit(3);
    }
    let steps = if name.starts_with("slow") { 40 } else { 3 };
    for i in 0..steps {
        emit(&WorkerMessage::Progress {
            percent: (i * 100 / steps) as f32,
        });
        if name.starts_with("slow") {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    std::fs::write(&job.target_part_path, b"export").unwrap();
    emit(&WorkerMessage::Done {
        bytes: 6,
        duration_us: job.out_us - job.in_us,
    });
}
