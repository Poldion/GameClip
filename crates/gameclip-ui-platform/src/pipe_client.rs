// SPDX-License-Identifier: GPL-3.0-or-later
//! Verbindung der Oberfläche zum Dienst (Design, Datei 58, Protokoll in `docs/protocol.md`).
//!
//! Eine synchrone Pipe serialisiert Lesen und Schreiben auf demselben Handle; ein blockierendes
//! `ReadFile` würde jedes Schreiben aufhalten. Deshalb besitzt ein einziger I/O-Thread die Pipe:
//! er schreibt wartende Anfragen, liest, sobald `PeekNamedPipe` Daten meldet, und verteilt
//! Antworten (nach `id`) und Ereignisse (Feld `event`).

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded, unbounded};
use gameclip_core::ids::pipe_name;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::framing::{read_json, write_json};
use gameclip_protocol::messages::{Command, HelloReply, Request, Response, ServiceEvent};
use gameclip_protocol::version::PROTOCOL_VERSION;
use thiserror::Error;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows::Win32::System::Pipes::{GetNamedPipeServerProcessId, PeekNamedPipe};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_core::PWSTR;

/// `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION`: Der Server darf den Client nur
/// identifizieren, nicht in seinem Namen handeln.
const SQOS_IDENTIFICATION: u32 = 0x0010_0000 | (1 << 16);
const POLL: Duration = Duration::from_millis(10);

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("Dienst läuft nicht")]
    NotRunning,
    #[error("Pipe gehört nicht zu GameClip ({0})")]
    WrongServer(String),
    #[error("Version passt nicht: {0}")]
    VersionMismatch(String),
    #[error("{0}")]
    Io(String),
}

#[derive(Debug, Error)]
pub enum CallError {
    #[error("Verbindung zum Dienst getrennt")]
    Disconnected,
    #[error("Dienst antwortet nicht")]
    Timeout,
}

struct Pending {
    request: Request,
    reply: Sender<Response>,
}

/// Offene Verbindung. Wird sie fallen gelassen, endet der I/O-Thread und die Pipe schließt.
pub struct Client {
    tx: Sender<Pending>,
    next_id: AtomicU32,
    pub hello: HelloReply,
}

/// Wie die Pipe des Servers geprüft wird.
#[derive(Debug, Clone)]
pub enum ServerCheck {
    /// Der Server muss diese Exe sein (im Paket: `InstalledLocation\gameclipd.exe`).
    Exe(PathBuf),
    /// Keine Prüfung (Entwicklung ohne Paket, Tests).
    None,
}

impl Client {
    /// Verbindet sich mit dem Dienst des angemeldeten Benutzers, prüft den Server und führt den
    /// Handshake. Ereignisse kommen über den zurückgegebenen Kanal; er schließt mit der Pipe.
    pub fn connect(
        client_name: &str,
        check: &ServerCheck,
    ) -> Result<(Client, Receiver<ServiceEvent>), ConnectError> {
        let sid = user_sid().map_err(|e| ConnectError::Io(e.to_string()))?;
        Self::connect_to(&pipe_name(&sid), client_name, check)
    }

    /// Wie `connect`, mit eigenem Pipe-Namen (Tests).
    pub fn connect_to(
        name: &str,
        client_name: &str,
        check: &ServerCheck,
    ) -> Result<(Client, Receiver<ServiceEvent>), ConnectError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(SQOS_IDENTIFICATION)
            .open(name)
            .map_err(|e| match e.raw_os_error() {
                // ERROR_FILE_NOT_FOUND: keine Pipe; ERROR_PIPE_BUSY: alle Instanzen belegt.
                Some(2) | Some(231) => ConnectError::NotRunning,
                _ => ConnectError::Io(e.to_string()),
            })?;
        let handle = HANDLE(file.as_raw_handle());
        if let ServerCheck::Exe(expected) = check {
            let actual = server_exe(handle).map_err(|e| ConnectError::Io(e.to_string()))?;
            if !same_path(&actual, expected) {
                return Err(ConnectError::WrongServer(actual.display().to_string()));
            }
        }

        let hello = Request {
            id: 1,
            cmd: Command::Hello {
                protocol: PROTOCOL_VERSION,
                client: client_name.into(),
            },
        };
        write_json(&mut file, &hello).map_err(|e| ConnectError::Io(e.to_string()))?;
        let reply: Response = read_json(&mut file).map_err(|e| ConnectError::Io(e.to_string()))?;
        let hello: HelloReply = match (reply.ok, reply.body, reply.error) {
            (true, Some(body), _) => {
                serde_json::from_value(body).map_err(|e| ConnectError::Io(e.to_string()))?
            }
            (_, _, Some(err)) if err.code == ErrorCode::VersionMismatch => {
                return Err(ConnectError::VersionMismatch(err.message));
            }
            (_, _, err) => {
                return Err(ConnectError::Io(
                    err.map(|e| e.message)
                        .unwrap_or_else(|| "leere Antwort".into()),
                ));
            }
        };

        let (tx, rx) = unbounded::<Pending>();
        let (ev_tx, ev_rx) = unbounded();
        std::thread::Builder::new()
            .name("gameclip-pipe".into())
            .spawn(move || {
                if let Err(e) = io_loop(file, &rx, &ev_tx) {
                    tracing::info!("Pipe zum Dienst getrennt: {e}");
                }
            })
            .map_err(|e| ConnectError::Io(e.to_string()))?;
        Ok((
            Client {
                tx,
                next_id: AtomicU32::new(2),
                hello,
            },
            ev_rx,
        ))
    }

    /// Schickt einen Befehl und wartet auf die Antwort.
    pub fn call(&self, cmd: Command, timeout: Duration) -> Result<Response, CallError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply, rx) = bounded(1);
        self.tx
            .send(Pending {
                request: Request { id, cmd },
                reply,
            })
            .map_err(|_| CallError::Disconnected)?;
        rx.recv_timeout(timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => CallError::Timeout,
            RecvTimeoutError::Disconnected => CallError::Disconnected,
        })
    }
}

fn io_loop(
    mut file: File,
    requests: &Receiver<Pending>,
    events: &Sender<ServiceEvent>,
) -> Result<(), String> {
    let handle = HANDLE(file.as_raw_handle());
    let mut waiting: HashMap<u32, Sender<Response>> = HashMap::new();
    loop {
        // Anfragen schreiben; ist der Client weg, endet die Verbindung.
        loop {
            match requests.try_recv() {
                Ok(p) => {
                    write_json(&mut file, &p.request).map_err(|e| e.to_string())?;
                    waiting.insert(p.request.id, p.reply);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => return Ok(()),
            }
        }
        let mut avail = 0u32;
        unsafe { PeekNamedPipe(handle, None, 0, None, Some(&mut avail), None) }
            .map_err(|e| e.to_string())?;
        if avail == 0 {
            // Auf neue Anfragen warten statt fest zu schlafen, damit Befehle sofort rausgehen.
            match requests.recv_timeout(POLL) {
                Ok(p) => {
                    write_json(&mut file, &p.request).map_err(|e| e.to_string())?;
                    waiting.insert(p.request.id, p.reply);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
            continue;
        }
        let msg: serde_json::Value = read_json(&mut file).map_err(|e| e.to_string())?;
        if msg.get("event").is_some() {
            match serde_json::from_value::<ServiceEvent>(msg) {
                Ok(ev) => {
                    let _ = events.send(ev);
                }
                Err(e) => tracing::warn!("Unbekanntes Ereignis: {e}"),
            }
        } else {
            match serde_json::from_value::<Response>(msg) {
                Ok(r) => {
                    if let Some(tx) = waiting.remove(&r.id) {
                        let _ = tx.send(r);
                    }
                }
                Err(e) => tracing::warn!("Unlesbare Antwort: {e}"),
            }
        }
        let _ = file.flush();
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().to_lowercase();
        s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
    };
    norm(a) == norm(b)
}

/// Programmpfad des Prozesses, der die Pipe anbietet.
fn server_exe(pipe: HANDLE) -> windows::core::Result<PathBuf> {
    unsafe {
        let mut pid = 0u32;
        GetNamedPipeServerProcessId(pipe, &mut pid)?;
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result?;
        Ok(PathBuf::from(String::from_utf16_lossy(
            &buf[..len as usize],
        )))
    }
}

/// SID des angemeldeten Benutzers (`S-1-5-21-…`), Teil des Pipe-Namens.
pub fn user_sid() -> windows::core::Result<String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
        let mut buf = vec![0u8; len as usize];
        let result = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            len,
            &mut len,
        );
        let _ = CloseHandle(token);
        result?;
        let user = &*(buf.as_ptr() as *const TOKEN_USER);
        let mut s = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut s)?;
        let sid = s.to_string().unwrap_or_default();
        let _ = LocalFree(Some(HLOCAL(s.0.cast())));
        Ok(sid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_compare_without_case_and_prefix() {
        assert!(same_path(
            Path::new(r"\\?\C:\Program Files\WindowsApps\GameClip\gameclipd.exe"),
            Path::new(r"c:\program files\windowsapps\gameclip\GAMECLIPD.exe"),
        ));
        assert!(!same_path(
            Path::new(r"C:\a\gameclipd.exe"),
            Path::new(r"C:\b\gameclipd.exe")
        ));
    }

    #[test]
    fn missing_pipe_means_not_running() {
        let name = format!(r"\\.\pipe\gameclip-test-missing-{}", std::process::id());
        assert!(matches!(
            Client::connect_to(&name, "test", &ServerCheck::None),
            Err(ConnectError::NotRunning)
        ));
    }
}
