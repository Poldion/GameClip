// SPDX-License-Identifier: GPL-3.0-or-later
//! Dateinamen für Clips und Exporte.

use time::OffsetDateTime;
use time::macros::format_description;

const MAX_PROGRAM_CHARS: usize = 64;
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Export-Profil, soweit es den Dateinamen bestimmt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSuffix {
    Trim,
    Tracks,
    TargetSize { bytes: u64 },
}

/// `<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4`; `local_time` ist Ortszeit.
pub fn clip_file_name(program: Option<&str>, local_time: OffsetDateTime) -> String {
    let stamp = local_time
        .format(format_description!(
            "[year]-[month]-[day]_[hour]-[minute]-[second]"
        ))
        .expect("festes Format");
    format!("{}_{stamp}.mp4", sanitize_program(program))
}

/// Name einer Export-Datei aus dem Dateistamm des Originals.
pub fn export_file_name(original_stem: &str, suffix: ExportSuffix) -> String {
    let tail = match suffix {
        ExportSuffix::Trim => "_trim".to_string(),
        ExportSuffix::Tracks => "_tracks".to_string(),
        ExportSuffix::TargetSize { bytes } => format!("_{}MB", bytes.div_ceil(1_000_000)),
    };
    format!("{original_stem}{tail}.mp4")
}

/// Hängt `_2`, `_3`, … vor der Endung an, bis `exists` den Namen nicht mehr kennt.
pub fn dedupe(name: &str, exists: impl Fn(&str) -> bool) -> String {
    if !exists(name) {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    (2..)
        .map(|n| format!("{stem}_{n}{ext}"))
        .find(|candidate| !exists(candidate))
        .expect("unendliche Folge")
}

fn sanitize_program(program: Option<&str>) -> String {
    let raw = program.unwrap_or("");
    let raw = raw
        .strip_suffix(".exe")
        .or_else(|| raw.strip_suffix(".EXE"))
        .unwrap_or(raw);
    let mut name: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .take(MAX_PROGRAM_CHARS)
        .collect();
    while name.ends_with('.') || name.ends_with(' ') {
        name.pop();
    }
    if name.is_empty() {
        return "Clip".to_string();
    }
    if RESERVED.iter().any(|r| r.eq_ignore_ascii_case(&name)) {
        name.push('_');
    }
    name
}

/// Programm aus einem Dateinamen, den [`clip_file_name`] erzeugt hat, für die Gruppierung in der
/// Galerie. Erkennt auch Exporte (`_trim`, `_tracks`, `_10MB`) und Zähler (`_2`). `None` für
/// „Clip_…“ (kein Programm bekannt) und Namen ohne Zeitstempel.
pub fn program_from_file_name(file_name: &str) -> Option<String> {
    let stem = file_name.rsplit_once('.').map_or(file_name, |(s, _)| s);
    // Zeitstempel `_JJJJ-MM-TT_HH-MM-SS` suchen; davor steht das Programm.
    let bytes = stem.as_bytes();
    const LEN: usize = "_2026-11-05_21-14-03".len();
    let is_stamp = |i: usize| -> bool {
        let s = &bytes[i..i + LEN];
        s[0] == b'_'
            && s[5] == b'-'
            && s[8] == b'-'
            && s[11] == b'_'
            && s[14] == b'-'
            && s[17] == b'-'
            && [1, 2, 3, 4, 6, 7, 9, 10, 12, 13, 15, 16, 18, 19]
                .iter()
                .all(|&k| s[k].is_ascii_digit())
    };
    let start = (0..bytes.len().checked_sub(LEN)? + 1)
        .rev()
        .find(|&i| is_stamp(i))?;
    let program = &stem[..start];
    if program.is_empty() || program == "Clip" {
        return None;
    }
    Some(program_label(program))
}

/// Lesbarer Name aus dem Exe-Namen: Unreal-Endungen wie `-Win64-Shipping` fallen weg.
pub fn program_label(program: &str) -> String {
    let mut name = program;
    for suffix in [
        "-Win64-Shipping",
        "-WinGDK-Shipping",
        "-Shipping",
        "-Win64",
        "_x64",
        "_Data",
    ] {
        if let Some(s) = name.strip_suffix(suffix) {
            name = s;
        }
    }
    if name.is_empty() { program } else { name }.to_string()
}
