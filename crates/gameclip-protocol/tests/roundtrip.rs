// SPDX-License-Identifier: GPL-3.0-or-later
use std::io::Cursor;

use gameclip_core::retention::FolderKind;
use gameclip_core::settings::Hotkey;
use gameclip_protocol::error::ErrorCode;
use gameclip_protocol::framing::{FrameError, read_frame, read_json, write_frame, write_json};
use gameclip_protocol::messages::{
    Command, ExportProfile, ListClipsReply, Request, Response, ServiceEvent,
};
use gameclip_protocol::version::MAX_MESSAGE_BYTES;
use gameclip_protocol::worker::WorkerMessage;
use serde_json::json;

fn roundtrip_request(req: &Request) {
    let text = serde_json::to_string(req).unwrap();
    let back: Request = serde_json::from_str(&text).unwrap();
    assert_eq!(&back, req, "{text}");
}

#[test]
fn every_command_shape_roundtrips() {
    let cmds = vec![
        Command::Hello {
            protocol: 1,
            client: "gameclip-ui".into(),
        },
        Command::GetStatus,
        Command::SetSettings {
            partial: json!({"capture": {"fps": 30}}),
        },
        Command::TestRegister {
            hotkey: Hotkey::default(),
        },
        Command::ListClips {
            folder: FolderKind::Exports,
        },
        Command::SetStar {
            clip_id: 7,
            on: true,
        },
        Command::GetExportEstimate {
            clip_id: 7,
            in_us: 1_000_000,
            out_us: 9_000_000,
            target_bytes: 10_000_000,
        },
        Command::StartExportJob {
            clip_id: 7,
            in_us: 0,
            out_us: 5,
            profile: ExportProfile::TargetSize { bytes: 10 },
        },
        Command::CancelJob { job_id: 3 },
    ];
    for (i, cmd) in cmds.into_iter().enumerate() {
        roundtrip_request(&Request { id: i as u32, cmd });
    }
}

#[test]
fn envelope_matches_documented_shape() {
    let req = Request {
        id: 4,
        cmd: Command::SetStar {
            clip_id: 9,
            on: false,
        },
    };
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        json!({"id": 4, "type": "SetStar", "body": {"clip_id": 9, "on": false}})
    );
    let unit = Request {
        id: 5,
        cmd: Command::GetStatus,
    };
    assert_eq!(
        serde_json::to_value(&unit).unwrap(),
        json!({"id": 5, "type": "GetStatus"})
    );
    let parsed: Request = serde_json::from_value(json!({"id": 6, "type": "SaveClip"})).unwrap();
    assert_eq!(parsed.cmd, Command::SaveClip);
}

#[test]
fn unknown_fields_are_ignored() {
    let parsed: Request = serde_json::from_value(json!({
        "id": 1, "type": "SetStar", "body": {"clip_id": 2, "on": true, "future": 1}, "extra": "x"
    }))
    .unwrap();
    assert_eq!(
        parsed.cmd,
        Command::SetStar {
            clip_id: 2,
            on: true
        }
    );
}

#[test]
fn unknown_command_is_an_error() {
    let parsed = serde_json::from_value::<Request>(json!({"id": 1, "type": "Teleport"}));
    assert!(parsed.is_err());
}

#[test]
fn events_use_event_tag() {
    let e = ServiceEvent::NewClip {
        clip_id: 3,
        folder: FolderKind::Clips,
    };
    assert_eq!(
        serde_json::to_value(&e).unwrap(),
        json!({"event": "NewClip", "body": {"clip_id": 3, "folder": "clips"}})
    );
}

#[test]
fn responses() {
    let reply = ListClipsReply {
        clips: vec![],
        used_bytes: 1,
        limit_bytes: 2,
    };
    let ok = Response::ok(8, &reply);
    assert_eq!(
        serde_json::to_value(&ok).unwrap(),
        json!({"id": 8, "ok": true, "body": {"clips": [], "used_bytes": 1, "limit_bytes": 2}})
    );
    let err = Response::err(9, ErrorCode::UnknownCommand, "unbekannt");
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({"id": 9, "ok": false, "error": {"code": "UnknownCommand", "message": "unbekannt"}})
    );
}

#[test]
fn framing_roundtrip_and_limit() {
    let mut buf = Vec::new();
    write_json(
        &mut buf,
        &Request {
            id: 1,
            cmd: Command::GetStatus,
        },
    )
    .unwrap();
    assert_eq!(&buf[..4], &(buf.len() as u32 - 4).to_le_bytes());
    let back: Request = read_json(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(back.cmd, Command::GetStatus);

    let mut huge = Vec::new();
    huge.extend_from_slice(&((MAX_MESSAGE_BYTES as u32) + 1).to_le_bytes());
    assert!(matches!(
        read_frame(&mut Cursor::new(huge)),
        Err(FrameError::TooLarge(_))
    ));
    assert!(matches!(
        write_frame(&mut Vec::new(), &vec![0; MAX_MESSAGE_BYTES + 1]),
        Err(FrameError::TooLarge(_))
    ));
}

#[test]
fn worker_messages_are_json_lines() {
    let line = serde_json::to_string(&WorkerMessage::Progress { percent: 42.0 }).unwrap();
    assert_eq!(line, r#"{"progress":{"percent":42.0}}"#);
}

/// Jedes ```json-Beispiel aus docs/protocol.md muss sich einlesen lassen. Nach `json` darf der
/// Name eines Antworttyps stehen; dann wird auch `body` als dieser Typ gelesen.
#[test]
fn every_example_in_protocol_md_parses() {
    use gameclip_protocol::messages::*;
    let doc = include_str!("../../../docs/protocol.md");
    let mut count = 0;
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        let Some(info) = line.strip_prefix("```json") else {
            continue;
        };
        let block: Vec<&str> = lines
            .by_ref()
            .take_while(|l| !l.starts_with("```"))
            .collect();
        let text = block.join("\n");
        let value: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("kein JSON ({e}): {text}"));
        if value.get("event").is_some() {
            serde_json::from_value::<ServiceEvent>(value.clone())
                .unwrap_or_else(|e| panic!("Ereignis ({e}): {text}"));
        } else if value.get("ok").is_some() {
            let r: Response = serde_json::from_value(value.clone())
                .unwrap_or_else(|e| panic!("Antwort ({e}): {text}"));
            let body = r.body.clone().unwrap_or(serde_json::Value::Null);
            let check = |res: Result<(), serde_json::Error>| {
                res.unwrap_or_else(|e| panic!("Body ({e}): {text}"))
            };
            match info.trim() {
                "" => {}
                "HelloReply" => check(serde_json::from_value::<HelloReply>(body).map(drop)),
                "GetStatusReply" => check(serde_json::from_value::<GetStatusReply>(body).map(drop)),
                "ListClipsReply" => check(serde_json::from_value::<ListClipsReply>(body).map(drop)),
                "ListJobsReply" => check(serde_json::from_value::<ListJobsReply>(body).map(drop)),
                "StartExportJobReply" => {
                    check(serde_json::from_value::<StartExportJobReply>(body).map(drop))
                }
                "TestRegisterReply" => {
                    check(serde_json::from_value::<TestRegisterReply>(body).map(drop))
                }
                "AutostartReply" => check(serde_json::from_value::<AutostartReply>(body).map(drop)),
                other => panic!("unbekannter Antworttyp {other}"),
            }
        } else {
            let req: Request = serde_json::from_value(value.clone())
                .unwrap_or_else(|e| panic!("Anfrage ({e}): {text}"));
            assert_eq!(serde_json::to_value(&req).unwrap()["type"], value["type"]);
        }
        count += 1;
    }
    assert!(count >= 20, "nur {count} Beispiele gefunden");
}
