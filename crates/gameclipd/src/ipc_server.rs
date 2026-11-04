// SPDX-License-Identifier: GPL-3.0-or-later
//! Named-Pipe-Server (Design, Datei 53). M0-Umfang: Handshake, `GetStatus`, `SaveClip`,
//! Ereignisse `SaveCompleted`/`SaveFailed`; alle anderen Befehle antworten `UnknownCommand`.
//!
//! Sicherheit: SDDL `D:P(A;;GA;;;<SID>)` (nur der angemeldete Benutzer),
//! `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` für die erste Instanz, höchstens
//! 4 Verbindungen.
//!
//! Ein synchrones Pipe-Handle serialisiert Lesen und Schreiben; ein blockierendes `ReadFile`
//! würde also das Senden von Ereignissen aufhalten. Der Verbindungs-Thread prüft deshalb mit
//! `PeekNamedPipe`, ob eine Anfrage vorliegt, und sendet dazwischen Ereignisse (Takt 10 ms).

use std::fs::File;
use std::io::Write;
use std::os::windows::io::FromRawHandle;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use gameclip_core::ids::pipe_name;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::framing::{read_json, write_json};
use gameclip_protocol::messages::{
    Command, ExportProfile, HelloReply, Request, Response, ServiceEvent, StartExportJobReply,
};
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

/// Was der Server von der Hauptschleife will.
#[derive(Debug)]
pub enum IpcRequest {
    SaveClip,
    StartExport {
        clip_id: u32,
        in_us: u64,
        out_us: u64,
        profile: ExportProfile,
        reply: Sender<Result<u32, (ErrorCode, String)>>,
    },
    CancelJob(u32),
    /// Antwort auf `GetStatus` (JSON des `ServiceStatus`).
    GetStatus(Sender<serde_json::Value>),
}

/// Verteilt Ereignisse an alle offenen Verbindungen.
#[derive(Clone, Default)]
pub struct Broadcaster(Arc<Mutex<Vec<Sender<ServiceEvent>>>>);

impl Broadcaster {
    pub fn send(&self, ev: ServiceEvent) {
        if let Ok(mut list) = self.0.lock() {
            list.retain(|tx| tx.send(ev.clone()).is_ok());
        }
    }

    fn subscribe(&self) -> Receiver<ServiceEvent> {
        let (tx, rx) = unbounded();
        if let Ok(mut list) = self.0.lock() {
            list.push(tx);
        }
        rx
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
    /// Startet den Annahme-Thread. Anfragen gehen an `requests`, Ereignisse kommen über
    /// `events` (Broadcaster).
    pub fn start(requests: Sender<IpcRequest>, events: Broadcaster) -> windows::core::Result<()> {
        let sid = user_sid()?;
        let name = pipe_name(&sid);
        std::thread::Builder::new()
            .name("gameclip-ipc".into())
            .spawn(move || accept_loop(&name, &sid, &requests, &events))
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

fn accept_loop(name: &str, sid: &str, requests: &Sender<IpcRequest>, events: &Broadcaster) {
    let mut first = true;
    loop {
        let handle = match create_instance(name, sid, first) {
            Ok(h) => h,
            Err(e) => {
                warn!("Pipe lässt sich nicht anlegen: {e}");
                // Alle Instanzen belegt: kurz warten, bis eine Verbindung endet.
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
        };
        if first {
            info!("Pipe bereit: {name}");
        }
        first = false;
        let connected = unsafe { ConnectNamedPipe(handle, None) };
        if let Err(e) = connected
            && e.code() != ERROR_PIPE_CONNECTED.to_hresult()
        {
            unsafe {
                let _ = CloseHandle(handle);
            }
            continue;
        }
        let (requests, events) = (requests.clone(), events.subscribe());
        // HANDLE ist nicht Send; der Rohwert geht als Zahl an den Verbindungs-Thread.
        let raw = handle.0 as usize;
        let _ = std::thread::Builder::new()
            .name("gameclip-ipc-conn".into())
            .spawn(move || {
                let handle = HANDLE(raw as *mut std::ffi::c_void);
                // SAFETY: Das Handle gehört ab hier allein der Datei.
                let file = unsafe { File::from_raw_handle(handle.0) };
                if let Err(e) = connection(file, handle, &requests, &events) {
                    tracing::debug!("Verbindung beendet: {e}");
                }
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
        let req: Request = read_json(&mut file).map_err(err)?;
        let response = match req.cmd {
            Command::SaveClip => {
                let _ = requests.send(IpcRequest::SaveClip);
                Response::ok(req.id, &serde_json::Value::Null)
            }
            Command::GetStatus => {
                let (tx, rx) = crossbeam_channel::bounded(1);
                let _ = requests.send(IpcRequest::GetStatus(tx));
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(status) => Response::ok(req.id, &status),
                    Err(_) => Response::err(req.id, ErrorCode::Timeout, "Dienst antwortet nicht"),
                }
            }
            Command::StartExportJob {
                clip_id,
                in_us,
                out_us,
                profile,
            } => {
                let (tx, rx) = crossbeam_channel::bounded(1);
                let _ = requests.send(IpcRequest::StartExport {
                    clip_id,
                    in_us,
                    out_us,
                    profile,
                    reply: tx,
                });
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(Ok(job_id)) => Response::ok(req.id, &StartExportJobReply { job_id }),
                    Ok(Err((code, msg))) => Response::err(req.id, code, msg),
                    Err(_) => Response::err(req.id, ErrorCode::Timeout, "Dienst antwortet nicht"),
                }
            }
            Command::CancelJob { job_id } => {
                let _ = requests.send(IpcRequest::CancelJob(job_id));
                Response::ok(req.id, &serde_json::Value::Null)
            }
            Command::Hello { .. } => {
                Response::err(req.id, ErrorCode::InvalidArgument, "Hello nur einmal")
            }
            _ => Response::err(req.id, ErrorCode::UnknownCommand, "in M0 nicht umgesetzt"),
        };
        write_json(&mut file, &response).map_err(err)?;
        file.flush().map_err(|e| e.to_string())?;
    }
}
