// SPDX-License-Identifier: GPL-3.0-or-later
//! Eigenes Protokoll `clip` (Design, Datei 60, T8). WebView2 erreicht es als
//! `http://clip.localhost/<clip_id>` (Datei) und `http://clip.localhost/thumb/<clip_id>`
//! (Vorschaubild). Den Pfad liefert nur der Dienst (`GetClip`); das Frontend nennt nie Pfade.
//!
//! Range: `bytes=a-b`, `bytes=a-`, `bytes=-n`. Antwort 206 mit höchstens 1 MiB und
//! `Content-Range`; der Player fordert den Rest selbst nach. Ohne Range 200 mit der ganzen Datei.
//! Unbekannte ID 404, 416 nur wenn `a ≥ size`.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder};

use crate::commands::clip_path;
use crate::ipc::Ipc;

pub const MAX_CHUNK: u64 = 1024 * 1024;
const THUMB_SIZE: u32 = 320;

#[derive(Debug, PartialEq, Eq)]
pub enum Range {
    /// Kein (verwertbarer) Range-Kopf: ganze Datei.
    Full,
    /// Inklusive Grenzen, schon auf `MAX_CHUNK` und die Dateigröße gekürzt.
    Part {
        start: u64,
        end: u64,
    },
    Unsatisfiable,
}

/// Wertet den Range-Kopf aus. Mehrere Bereiche oder kaputte Angaben gelten als „ganze Datei“.
pub fn parse_range(header: Option<&str>, size: u64) -> Range {
    let Some(spec) = header.and_then(|h| h.trim().strip_prefix("bytes=")) else {
        return Range::Full;
    };
    if spec.contains(',') {
        return Range::Full;
    }
    let Some((a, b)) = spec.split_once('-') else {
        return Range::Full;
    };
    let (a, b) = (a.trim(), b.trim());
    let (start, end) = match (a.is_empty(), b.is_empty()) {
        // bytes=-n: die letzten n Byte.
        (true, false) => match b.parse::<u64>() {
            Ok(0) => return Range::Unsatisfiable,
            Ok(n) => (size.saturating_sub(n), size.saturating_sub(1)),
            Err(_) => return Range::Full,
        },
        (false, _) => {
            let Ok(start) = a.parse::<u64>() else {
                return Range::Full;
            };
            let end = if b.is_empty() {
                size.saturating_sub(1)
            } else {
                match b.parse::<u64>() {
                    Ok(e) if e >= start => e,
                    _ => return Range::Full,
                }
            };
            (start, end)
        }
        (true, true) => return Range::Full,
    };
    if start >= size {
        return Range::Unsatisfiable;
    }
    let end = end.min(size - 1).min(start + MAX_CHUNK - 1);
    Range::Part { start, end }
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .body(Vec::new())
        .expect("gültige Antwort")
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4") => "video/mp4",
        Some("m4a") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

pub fn serve_file(path: &Path, range_header: Option<&str>) -> std::io::Result<Response<Vec<u8>>> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let builder = Response::builder()
        .header(header::CONTENT_TYPE, content_type(path))
        .header(header::ACCEPT_RANGES, "bytes");
    Ok(match parse_range(range_header, size) {
        Range::Full => {
            let mut body = Vec::with_capacity(size as usize);
            file.read_to_end(&mut body)?;
            builder.status(StatusCode::OK).body(body)
        }
        Range::Part { start, end } => {
            file.seek(SeekFrom::Start(start))?;
            let mut body = vec![0u8; (end - start + 1) as usize];
            file.read_exact(&mut body)?;
            builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
                .body(body)
        }
        Range::Unsatisfiable => builder
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{size}"))
            .body(Vec::new()),
    }
    .expect("gültige Antwort"))
}

fn handle(ipc: &Ipc, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().trim_start_matches('/');
    let (thumb, id) = match path.strip_prefix("thumb/") {
        Some(id) => (true, id),
        None => (false, path),
    };
    let Ok(clip_id) = id.parse::<u32>() else {
        return status(StatusCode::NOT_FOUND);
    };
    let Ok(file) = clip_path(ipc, clip_id) else {
        return status(StatusCode::NOT_FOUND);
    };
    if thumb {
        return match gameclip_ui_platform::thumbnail::png(&file, THUMB_SIZE) {
            Ok(png) => Response::builder()
                .header(header::CONTENT_TYPE, "image/png")
                .header(header::CACHE_CONTROL, "max-age=3600")
                .body(png)
                .expect("gültige Antwort"),
            Err(_) => status(StatusCode::NOT_FOUND),
        };
    }
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok());
    serve_file(&file, range).unwrap_or_else(|_| status(StatusCode::NOT_FOUND))
}

/// Für `register_asynchronous_uri_scheme_protocol("clip", …)`.
pub fn protocol<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let ipc = ctx.app_handle().state::<Ipc>().inner().clone();
    // Pipe und Datei blockieren; nicht auf dem Thread der WebView.
    std::thread::spawn(move || responder.respond(handle(&ipc, &request)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_forms() {
        let size = 10 * MAX_CHUNK;
        assert_eq!(parse_range(None, size), Range::Full);
        assert_eq!(
            parse_range(Some("bytes=0-99"), size),
            Range::Part { start: 0, end: 99 }
        );
        assert_eq!(
            parse_range(Some("bytes=100-"), size),
            Range::Part {
                start: 100,
                end: 100 + MAX_CHUNK - 1
            }
        );
        assert_eq!(
            parse_range(Some("bytes=-500"), size),
            Range::Part {
                start: size - 500,
                end: size - 1
            }
        );
        // Ende jenseits der Datei wird gekürzt.
        assert_eq!(
            parse_range(Some("bytes=5-1000"), 50),
            Range::Part { start: 5, end: 49 }
        );
        assert_eq!(
            parse_range(Some("bytes=-1000"), 50),
            Range::Part { start: 0, end: 49 }
        );
    }

    #[test]
    fn unsatisfiable_only_past_the_end() {
        assert_eq!(parse_range(Some("bytes=50-"), 50), Range::Unsatisfiable);
        assert_eq!(
            parse_range(Some("bytes=49-"), 50),
            Range::Part { start: 49, end: 49 }
        );
        assert_eq!(parse_range(Some("bytes=0-"), 0), Range::Unsatisfiable);
        // Kaputte oder mehrfache Bereiche: ganze Datei statt Fehler.
        assert_eq!(parse_range(Some("bytes=9-3"), 50), Range::Full);
        assert_eq!(parse_range(Some("bytes=0-1,4-5"), 50), Range::Full);
        assert_eq!(parse_range(Some("items=0-1"), 50), Range::Full);
    }

    #[test]
    fn serves_partial_content_with_headers() {
        let dir = std::env::temp_dir().join(format!("gameclip-range-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.mp4");
        std::fs::write(&file, (0..=255u8).collect::<Vec<_>>()).unwrap();
        let r = serve_file(&file, Some("bytes=10-19")).unwrap();
        assert_eq!(r.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(r.headers()[header::CONTENT_RANGE], "bytes 10-19/256");
        assert_eq!(r.headers()[header::CONTENT_TYPE], "video/mp4");
        assert_eq!(r.body(), &(10..20u8).collect::<Vec<_>>());
        let r = serve_file(&file, None).unwrap();
        assert_eq!((r.status(), r.body().len()), (StatusCode::OK, 256));
        let r = serve_file(&file, Some("bytes=256-")).unwrap();
        assert_eq!(r.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
