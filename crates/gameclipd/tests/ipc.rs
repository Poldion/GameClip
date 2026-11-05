// SPDX-License-Identifier: GPL-3.0-or-later
//! Pipe-Server ohne Hauptschleife (Design, Datei 55): Handshake, Versionsprüfung, unbekannte
//! Befehle, Weiterleitung an die Hauptschleife, Ereignisse und Trennen.

use std::fs::{File, OpenOptions};
use std::time::Duration;

use crossbeam_channel::unbounded;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::framing::{read_json, write_frame, write_json};
use gameclip_protocol::messages::{Command, Request, Response, ServiceEvent};
use gameclip_protocol::version::PROTOCOL_VERSION;
use gameclipd::ipc_server::{Broadcaster, IpcRequest, IpcServer};

fn pipe(tag: &str) -> String {
    format!(r"\\.\pipe\gameclip-test-{tag}-{}", std::process::id())
}

fn connect(name: &str) -> File {
    for _ in 0..100 {
        if let Ok(f) = OpenOptions::new().read(true).write(true).open(name) {
            return f;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("Pipe {name} nicht erreichbar");
}

fn hello(f: &mut File, protocol: u32) -> Response {
    let req = Request {
        id: 1,
        cmd: Command::Hello {
            protocol,
            client: "test".into(),
        },
    };
    write_json(f, &req).unwrap();
    read_json(f).unwrap()
}

#[test]
fn version_mismatch_is_refused() {
    let name = pipe("version");
    let (tx, _rx) = unbounded();
    IpcServer::start_named(name.clone(), tx, Broadcaster::default()).unwrap();
    let mut f = connect(&name);
    let r = hello(&mut f, PROTOCOL_VERSION + 1);
    assert!(!r.ok);
    assert_eq!(r.error.unwrap().code, ErrorCode::VersionMismatch);
}

#[test]
fn first_message_must_be_hello() {
    let name = pipe("nohello");
    let (tx, _rx) = unbounded();
    IpcServer::start_named(name.clone(), tx, Broadcaster::default()).unwrap();
    let mut f = connect(&name);
    write_json(
        &mut f,
        &Request {
            id: 4,
            cmd: Command::GetStatus,
        },
    )
    .unwrap();
    let r: Response = read_json(&mut f).unwrap();
    assert_eq!(r.id, 4);
    assert_eq!(r.error.unwrap().code, ErrorCode::InvalidArgument);
}

#[test]
fn calls_unknown_commands_events_and_disconnect() {
    let name = pipe("calls");
    let (tx, rx) = unbounded();
    let events = Broadcaster::default();
    IpcServer::start_named(name.clone(), tx, events.clone()).unwrap();
    // Hauptschleife als Attrappe: beantwortet jeden Befehl mit seinem Typnamen.
    let handler = std::thread::spawn(move || {
        let mut conns = Vec::new();
        while let Ok(req) = rx.recv_timeout(Duration::from_secs(10)) {
            match req {
                IpcRequest::Call {
                    conn,
                    id,
                    cmd,
                    reply,
                } => {
                    conns.push(conn);
                    let _ = reply.send(Response::ok(id, &format!("{cmd:?}")));
                }
                IpcRequest::Disconnected(conn) => return (conns, conn),
            }
        }
        panic!("keine Trennung gemeldet");
    });

    let mut f = connect(&name);
    let r = hello(&mut f, PROTOCOL_VERSION);
    assert!(r.ok, "{r:?}");

    write_json(
        &mut f,
        &Request {
            id: 2,
            cmd: Command::GetStatus,
        },
    )
    .unwrap();
    let r: Response = read_json(&mut f).unwrap();
    assert_eq!((r.id, r.ok), (2, true));
    assert_eq!(r.body.unwrap(), serde_json::json!("GetStatus"));

    // Unbekannter Typ: Antwort mit derselben id, Verbindung bleibt offen.
    write_frame(&mut f, br#"{"id":3,"type":"Fliegen"}"#).unwrap();
    let r: Response = read_json(&mut f).unwrap();
    assert_eq!(r.id, 3);
    assert_eq!(r.error.unwrap().code, ErrorCode::UnknownCommand);

    // Ereignis an alle Verbindungen.
    events.send(ServiceEvent::MicLevel { dbfs: -12.0 });
    let ev: ServiceEvent = read_json(&mut f).unwrap();
    assert_eq!(ev, ServiceEvent::MicLevel { dbfs: -12.0 });

    write_json(
        &mut f,
        &Request {
            id: 5,
            cmd: Command::ListJobs,
        },
    )
    .unwrap();
    let r: Response = read_json(&mut f).unwrap();
    assert_eq!(r.id, 5);

    drop(f);
    let (conns, gone) = handler.join().unwrap();
    assert!(conns.iter().all(|c| *c == gone));
}
