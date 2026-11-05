// SPDX-License-Identifier: GPL-3.0-or-later
//! Named-Pipe-Server (Design, Datei 53).
//!
//! Jede Anfrage geht als `IpcRequest::Call` mit einem Antwortkanal an die Hauptschleife; dort
//! entsteht die `Response`. Beim Trennen meldet der Server `Disconnected(conn)`, damit der Dienst
//! die Anforderungen dieser Verbindung zurücknimmt (`SuspendHotkey`, `WatchFolders`,
//! `SubscribeMicLevel`). Ereignisse gehen über den `Broadcaster` an alle oder an ausgewählte
//! Verbindungen.
//!
//! Sicherheit: SDDL `D:P(A;;GA;;;<SID>)` (nur der angemeldete Benutzer),
//! `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` für die erste Instanz, höchstens
//! 4 Verbindungen.
//!
//! Ein synchrones Pipe-Handle serialisiert Lesen und Schreiben; ein blockierendes `ReadFile`
//! würde also das Senden von Ereignissen aufhalten. Der Verbindungs-Thread prüft deshalb mit
//! `PeekNamedPipe`, ob eine Anfrage vorliegt, und sendet dazwischen Ereignisse (Takt 10 ms).

use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::os::windows::io::FromRawHandle;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use gameclip_core::ids::pipe_name;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::framing::{read_json, write_json};
use gameclip_protocol::messages::{Command, HelloReply, Request, Response, ServiceEvent};
use gameclip_protocol::version::PROTOCOL_VERSION;
use tracing::{info, warn};
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT, PeekNamedPipe,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_core::{HSTRING, PWSTR};

const MAX_INSTANCES: u32 = 4;
const BUFFER: u32 = 64 * 1024;
const POLL: Duration = Duration::from_millis(10);
/// Längste Wartezeit auf die Hauptschleife (z. B. `AdoptClip` schreibt eine Datei um).
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Was der Server von der Hauptschleife will.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)] // wenige Anfragen je Sekunde; Box lohnt nicht
pub enum IpcRequest {
    Call {
        conn: u32,
        id: u32,
        cmd: Command,
        reply: Sender<Response>,
    },
    Disconnected(u32),
}

type Subscribers = Vec<(u32, Sender<ServiceEvent>)>;

/// Verteilt Ereignisse an offene Verbindungen.
#[derive(Clone, Default)]
pub struct Broadcaster(Arc<Mutex<Subscribers>>);

impl Broadcaster {
    /// An alle Verbindungen.
    pub fn send(&self, ev: ServiceEvent) {
        if let Ok(mut list) = self.0.lock() {
            list.retain(|(_, tx)| tx.send(ev.clone()).is_ok());
        }
    }

    /// Nur an die genannten Verbindungen (z. B. Abonnenten des Mikrofonpegels).
    pub fn send_to(&self, conns: &HashSet<u32>, ev: ServiceEvent) {
        if conns.is_empty() {
            return;
        }
        if let Ok(mut list) = self.0.lock() {
            list.retain(|(c, tx)| !conns.contains(c) || tx.send(ev.clone()).is_ok());
        }
    }

    fn subscribe(&self, conn: u32) -> Receiver<ServiceEvent> {
        let (tx, rx) = unbounded();
        if let Ok(mut list) = self.0.lock() {
            list.push((conn, tx));
        }
        rx
    }

    fn unsubscribe(&self, conn: u32) {
        if let Ok(mut list) = self.0.lock() {
            list.retain(|(c, _)| *c != conn);
        }
    }
}

/// SID des angemeldeten Benutzers als Zeichenkette (`S-1-5-21-…`).
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

pub struct IpcServer;

impl IpcServer {
    /// Startet den Server unter dem Pipe-Namen des Benutzers.
    pub fn start(requests: Sender<IpcRequest>, events: Broadcaster) -> windows::core::Result<()> {
        let sid = user_sid()?;
        Self::start_named(pipe_name(&sid), requests, events)
    }

    /// Wie `start`, mit eigenem Pipe-Namen (Tests).
    pub fn start_named(
        name: String,
        requests: Sender<IpcRequest>,
        events: Broadcaster,
    ) -> windows::core::Result<()> {
        let sid = user_sid()?;
        // Erste Instanz sofort anlegen, damit ein laufender zweiter Server sofort auffällt.
        let first = create_instance(&name, &sid, true)?;
        info!("Pipe bereit: {name}");
        let first_raw = first.0 as usize;
        std::thread::Builder::new()
            .name("gameclip-ipc".into())
            .spawn(move || {
                let first = HANDLE(first_raw as *mut std::ffi::c_void);
                accept_loop(&name, &sid, first, &requests, &events)
            })
            .map_err(|_| windows::core::Error::from_thread())?;
        Ok(())
    }
}

fn create_instance(name: &str, sid: &str, first: bool) -> windows::core::Result<HANDLE> {
    unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(format!("D:P(A;;GA;;;{sid})")),
            SDDL_REVISION_1,
            &mut sd,
            None,
        )?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: false.into(),
        };
        let mut open = PIPE_ACCESS_DUPLEX;
        if first {
            open |= FILE_FLAG_FIRST_PIPE_INSTANCE;
        }
        let h = CreateNamedPipeW(
            &HSTRING::from(name),
            open,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            MAX_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            Some(&sa),
        );
        let _ = LocalFree(Some(HLOCAL(sd.0)));
        if h.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }
        Ok(h)
    }
}

fn accept_loop(
    name: &str,
    sid: &str,
    first: HANDLE,
    requests: &Sender<IpcRequest>,
    events: &Broadcaster,
) {
    static NEXT_CONN: AtomicU32 = AtomicU32::new(1);
    let mut next = Some(first);
    loop {
        let handle = match next.take() {
            Some(h) => h,
            None => match create_instance(name, sid, false) {
                Ok(h) => h,
                Err(e) => {
                    warn!("Pipe lässt sich nicht anlegen: {e}");
                    // Alle Instanzen belegt: kurz warten, bis eine Verbindung endet.
                    std::thread::sleep(Duration::from_millis(200));
                    continue;
                }
            },
        };
        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if let Err(e) = connected
            && e.code() != ERROR_PIPE_CONNECTED.to_hresult()
        {
            unsafe {
                let _ = CloseHandle(handle);
            }
            continue;
        }
        let conn = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
        let (requests, ev_rx, events) = (requests.clone(), events.subscribe(conn), events.clone());
        // HANDLE ist nicht Send; der Rohwert geht als Zahl an den Verbindungs-Thread.
        let raw = handle.0 as usize;
        let _ = std::thread::Builder::new()
            .name("gameclip-ipc-conn".into())
            .spawn(move || {
                let handle = HANDLE(raw as *mut std::ffi::c_void);
                // SAFETY: Das Handle gehört ab hier allein der Datei.
                let file = unsafe { File::from_raw_handle(handle.0) };
                if let Err(e) = connection(conn, file, handle, &requests, &ev_rx) {
                    tracing::debug!("Verbindung {conn} beendet: {e}");
                }
                events.unsubscribe(conn);
                let _ = requests.send(IpcRequest::Disconnected(conn));
            });
    }
}

fn available(handle: HANDLE) -> Result<u32, String> {
    let mut avail = 0u32;
    unsafe { PeekNamedPipe(handle, None, 0, None, Some(&mut avail), None) }
        .map_err(|e| e.to_string())?;
    Ok(avail)
}

fn connection(
    conn: u32,
    mut file: File,
    handle: HANDLE,
    requests: &Sender<IpcRequest>,
    events: &Receiver<ServiceEvent>,
) -> Result<(), String> {
    let err = |e: gameclip_protocol::framing::FrameError| e.to_string();
    // Handshake: erste Nachricht muss Hello sein.
    let hello: Request = read_json(&mut file).map_err(err)?;
    match hello.cmd {
        Command::Hello { protocol, .. } if protocol == PROTOCOL_VERSION => {
            let reply = HelloReply {
                protocol: PROTOCOL_VERSION,
                service_version: env!("CARGO_PKG_VERSION").into(),
            };
            write_json(&mut file, &Response::ok(hello.id, &reply)).map_err(err)?;
        }
        Command::Hello { protocol, .. } => {
            let msg = format!("Protokoll {protocol}, Dienst {PROTOCOL_VERSION}");
            let _ = write_json(
                &mut file,
                &Response::err(hello.id, ErrorCode::VersionMismatch, msg),
            );
            return Ok(());
        }
        _ => {
            let _ = write_json(
                &mut file,
                &Response::err(hello.id, ErrorCode::InvalidArgument, "Hello erwartet"),
            );
            return Ok(());
        }
    }

    loop {
        for ev in events.try_iter() {
            write_json(&mut file, &ev).map_err(err)?;
        }
        if available(handle)? == 0 {
            std::thread::sleep(POLL);
            continue;
        }
        // Unlesbare Befehle (unbekannter Typ) beantworten statt die Verbindung zu beenden.
        let raw: serde_json::Value = read_json(&mut file).map_err(err)?;
        let id = raw.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let response = match serde_json::from_value::<Request>(raw) {
            Ok(Request {
                cmd: Command::Hello { .. },
                ..
            }) => Response::err(id, ErrorCode::InvalidArgument, "Hello nur einmal"),
            Ok(req) => {
                let (tx, rx) = bounded(1);
                if requests
                    .send(IpcRequest::Call {
                        conn,
                        id: req.id,
                        cmd: req.cmd,
                        reply: tx,
                    })
                    .is_err()
                {
                    return Ok(());
                }
                rx.recv_timeout(CALL_TIMEOUT).unwrap_or_else(|_| {
                    Response::err(id, ErrorCode::Timeout, "Dienst antwortet nicht")
                })
            }
            Err(e) => Response::err(id, ErrorCode::UnknownCommand, e.to_string()),
        };
        write_json(&mut file, &response).map_err(err)?;
        file.flush().map_err(|e| e.to_string())?;
    }
}
