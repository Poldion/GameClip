// SPDX-License-Identifier: GPL-3.0-or-later
//! `gameclipd.exe --save-clip` (Design, Datei 41; intern, nicht öffentlich dokumentiert).
//!
//! Verbindet sich mit der Pipe des laufenden Dienstes, Handshake, `SaveClip`, wartet höchstens
//! 10 s auf `SaveCompleted`/`SaveFailed`. Exit 0 bei Erfolg, 1 bei Fehler oder ohne Dienst.
//! Wegen des GUI-Subsystems ist der Exit-Code nur mit `start /wait` bzw.
//! `Start-Process -Wait` sichtbar; Meldungen gehen nach `AttachConsole` auf stderr.

use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::time::Duration;

use gameclip_core::ids::pipe_name;
use gameclip_protocol::framing::{read_json, write_json};
use gameclip_protocol::messages::{Command, Request};
use gameclip_protocol::version::PROTOCOL_VERSION;
use windows::Win32::Foundation::{ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_NONE, OPEN_EXISTING,
};
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
use windows::Win32::System::Pipes::WaitNamedPipeW;
use windows_core::HSTRING;

use crate::ipc_server::user_sid;

const WAIT: Duration = Duration::from_secs(10);

pub fn save_clip() -> i32 {
    unsafe {
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }
    match run() {
        Ok(path) => {
            eprintln!("Clip gespeichert: {path}");
            0
        }
        Err(e) => {
            eprintln!("Speichern fehlgeschlagen: {e}");
            1
        }
    }
}

fn connect() -> Result<File, String> {
    let name = HSTRING::from(pipe_name(&user_sid().map_err(|e| e.to_string())?));
    for _ in 0..5 {
        let h = unsafe {
            CreateFileW(
                &name,
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };
        match h {
            Ok(h) => return Ok(unsafe { File::from_raw_handle(h.0) }),
            Err(e) if e.code() == ERROR_PIPE_BUSY.to_hresult() => unsafe {
                let _ = WaitNamedPipeW(&name, 2000);
            },
            Err(_) => return Err("GameClip-Dienst läuft nicht".into()),
        }
    }
    Err("Pipe belegt".into())
}

fn run() -> Result<String, String> {
    let mut pipe = connect()?;
    let err = |e: gameclip_protocol::framing::FrameError| e.to_string();
    let hello = Request {
        id: 1,
        cmd: Command::Hello {
            protocol: PROTOCOL_VERSION,
            client: "gameclipd --save-clip".into(),
        },
    };
    write_json(&mut pipe, &hello).map_err(err)?;
    let reply: serde_json::Value = read_json(&mut pipe).map_err(err)?;
    if reply["ok"] != true {
        return Err(format!("Handshake abgelehnt: {}", reply["error"]));
    }
    write_json(
        &mut pipe,
        &Request {
            id: 2,
            cmd: Command::SaveClip,
        },
    )
    .map_err(err)?;

    // Antworten und Ereignisse kommen gemischt; das Lesen blockiert, deshalb auf einem Thread.
    let (tx, rx) = crossbeam_channel::unbounded();
    std::thread::spawn(move || {
        while let Ok(v) = read_json::<serde_json::Value>(&mut pipe) {
            if tx.send(v).is_err() {
                break;
            }
        }
    });
    let deadline = std::time::Instant::now() + WAIT;
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        let msg = rx
            .recv_timeout(left)
            .map_err(|_| "keine Rückmeldung innerhalb von 10 s".to_string())?;
        match msg["event"].as_str() {
            Some("SaveCompleted") => {
                return Ok(msg["body"]["path"].as_str().unwrap_or_default().to_string());
            }
            Some("SaveFailed") => return Err(msg["body"]["message"].to_string()),
            _ if msg["id"] == 2 && msg["ok"] != true => {
                return Err(format!("SaveClip abgelehnt: {}", msg["error"]));
            }
            _ => {}
        }
    }
}
