// SPDX-License-Identifier: GPL-3.0-or-later
//! „Nach Updates suchen“ (Design, Datei 64; Entscheidung 2026-11-05).
//!
//! GameClip geht nie von selbst ins Internet (Non-Goal). Nur wenn der Benutzer in den
//! Einstellungen auf „Nach Updates suchen“ klickt, fragt [`check`] die Releases auf GitHub ab.
//! [`download`] lädt das Paket nach `Downloads\GameClip-Update`, [`open_installer`] öffnet es im
//! App Installer von Windows, der die laufende App beendet und das Update einspielt. Die Signatur
//! prüft Windows; ein Paket mit fremdem Zertifikat lehnt der App Installer ab.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use windows::Foundation::Uri;
use windows::Storage::Streams::DataReader;
use windows::Web::Http::HttpClient;
use windows::Win32::UI::Shell::{
    FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows_core::{HSTRING, PCWSTR, w};

/// Alle Releases (auch Vorabversionen, die das Testzertifikat erzeugt), neueste zuerst.
const RELEASES_URL: &str = "https://api.github.com/repos/Poldion/GameClip/releases?per_page=20";

/// Paketversion `a.b.c.d`.
pub type Version = [u16; 4];

pub fn version_string(v: Version) -> String {
    format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3])
}

/// Version des installierten Pakets; `None` ohne Paket (Entwicklung).
pub fn current_version() -> Option<Version> {
    let v = windows::ApplicationModel::Package::Current()
        .and_then(|p| p.Id())
        .and_then(|id| id.Version())
        .ok()?;
    Some([v.Major, v.Minor, v.Build, v.Revision])
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

/// Neueste Version auf GitHub mit einem x64-Paket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Available {
    pub version: Version,
    pub page_url: String,
    pub file_name: String,
    pub download_url: String,
    pub size_bytes: u64,
}

/// `v0.2.0` → `0.2.0.0` (so baut der Release-Workflow das Paket).
pub fn parse_tag(tag: &str) -> Option<Version> {
    let mut parts = tag.strip_prefix('v').unwrap_or(tag).split('.');
    let mut v = [0u16; 4];
    for slot in v.iter_mut().take(3) {
        *slot = parts.next()?.parse().ok()?;
    }
    if let Some(rev) = parts.next() {
        v[3] = rev.parse().ok()?;
    }
    parts.next().is_none().then_some(v)
}

/// Wählt aus der Antwort von GitHub das Release mit der höchsten Version, das ein Paket hat.
pub fn newest(releases: &[Release]) -> Option<Available> {
    releases
        .iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let version = parse_tag(&r.tag_name)?;
            let asset = r
                .assets
                .iter()
                .find(|a| a.name.starts_with("GameClip_") && a.name.ends_with("_x64.msix"))?;
            Some(Available {
                version,
                page_url: r.html_url.clone(),
                file_name: asset.name.clone(),
                download_url: asset.browser_download_url.clone(),
                size_bytes: asset.size,
            })
        })
        .max_by_key(|a| a.version)
}

fn err(msg: impl Into<String>) -> windows::core::Error {
    windows::core::Error::new(windows::Win32::Foundation::E_FAIL, msg.into())
}

fn client() -> windows::core::Result<HttpClient> {
    let client = HttpClient::new()?;
    let headers = client.DefaultRequestHeaders()?;
    // GitHub verlangt einen User-Agent; keine weiteren Angaben über den Rechner.
    headers.TryAppendWithoutValidation(
        &HSTRING::from("User-Agent"),
        &HSTRING::from(format!("GameClip/{}", env!("CARGO_PKG_VERSION"))),
    )?;
    headers.TryAppendWithoutValidation(
        &HSTRING::from("Accept"),
        &HSTRING::from("application/vnd.github+json"),
    )?;
    Ok(client)
}

/// Fragt GitHub nach Releases (blockiert; nicht auf dem Fenster-Thread aufrufen).
pub fn check() -> windows::core::Result<Option<Available>> {
    let text = client()?
        .GetStringAsync(&Uri::CreateUri(&HSTRING::from(RELEASES_URL))?)?
        .join()
        .map_err(|e| {
            // HTTP 404 als HRESULT: Ohne Anmeldung antwortet GitHub so auch bei privaten Repositorys.
            if e.code().0 as u32 == 0x8019_0194 {
                err("Keine Releases gefunden (Repository nicht öffentlich?)")
            } else {
                e
            }
        })?;
    let releases: Vec<Release> = serde_json::from_str(&text.to_string_lossy())
        .map_err(|e| err(format!("Antwort von GitHub nicht lesbar: {e}")))?;
    Ok(newest(&releases))
}

fn downloads_dir() -> windows::core::Result<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None)?;
        let path = PathBuf::from(p.to_string().map_err(|e| err(e.to_string()))?);
        windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
        Ok(path)
    }
}

/// Lädt das Paket nach `Downloads\GameClip-Update\<Name>` (blockiert).
pub fn download(update: &Available) -> windows::core::Result<PathBuf> {
    // Nur Dateinamen aus dem erwarteten Muster, damit kein Pfad aus der Antwort entsteht.
    let name = &update.file_name;
    if !(name.starts_with("GameClip_") && name.ends_with("_x64.msix"))
        || name.contains(['/', '\\', ':'])
    {
        return Err(err(format!("unerwarteter Dateiname {name}")));
    }
    let buffer = client()?
        .GetBufferAsync(&Uri::CreateUri(&HSTRING::from(&update.download_url))?)?
        .join()?;
    let reader = DataReader::FromBuffer(&buffer)?;
    let mut bytes = vec![0u8; buffer.Length()? as usize];
    reader.ReadBytes(&mut bytes)?;
    let dir = downloads_dir()?.join("GameClip-Update");
    std::fs::create_dir_all(&dir).map_err(|e| err(e.to_string()))?;
    let path = dir.join(name);
    std::fs::write(&path, &bytes).map_err(|e| err(e.to_string()))?;
    Ok(path)
}

/// Öffnet das Paket im App Installer von Windows.
pub fn open_installer(path: &Path) -> windows::core::Result<()> {
    let file = HSTRING::from(path.as_os_str());
    let r = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &file,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Rückgabe > 32 bedeutet Erfolg.
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(err(format!(
            "App Installer nicht gestartet ({})",
            r.0 as isize
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &[&str]) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("https://github.com/Poldion/GameClip/releases/tag/{tag}"),
            draft: false,
            assets: assets
                .iter()
                .map(|n| Asset {
                    name: (*n).into(),
                    browser_download_url: format!("https://example.invalid/{n}"),
                    size: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn parses_tags() {
        assert_eq!(parse_tag("v0.2.0"), Some([0, 2, 0, 0]));
        assert_eq!(parse_tag("1.10.3"), Some([1, 10, 3, 0]));
        assert_eq!(parse_tag("v0.1.0.6"), Some([0, 1, 0, 6]));
        assert_eq!(parse_tag("v0.2"), None);
        assert_eq!(parse_tag("v0.2.0-beta"), None);
        assert_eq!(parse_tag("v0.2.0.0.1"), None);
    }

    #[test]
    fn picks_highest_version_with_package() {
        let list = vec![
            release("v0.3.0", &["SHA256SUMS.txt"]),
            release(
                "v0.2.0",
                &["GameClip_0.2.0.0_x64.msix", "GameClip-Release.cer"],
            ),
            release("v0.10.0", &["GameClip_0.10.0.0_x64.msix"]),
            release("nightly", &["GameClip_x_x64.msix"]),
        ];
        let a = newest(&list).unwrap();
        assert_eq!(a.version, [0, 10, 0, 0]);
        assert_eq!(a.file_name, "GameClip_0.10.0.0_x64.msix");
    }

    #[test]
    fn ignores_drafts_and_empty_lists() {
        let mut r = release("v0.2.0", &["GameClip_0.2.0.0_x64.msix"]);
        r.draft = true;
        assert_eq!(newest(&[r]), None);
        assert_eq!(newest(&[]), None);
    }

    #[test]
    fn reads_github_json() {
        let json = r#"[{"tag_name":"v0.2.0","html_url":"https://github.com/x","draft":false,"prerelease":true,
            "assets":[{"name":"GameClip_0.2.0.0_x64.msix","browser_download_url":"https://github.com/d","size":42}]}]"#;
        let list: Vec<Release> = serde_json::from_str(json).unwrap();
        assert_eq!(newest(&list).unwrap().size_bytes, 42);
    }

    /// Echte Abfrage bei GitHub (Netz): `cargo test -p gameclip-ui-platform -- --ignored live`.
    #[test]
    #[ignore]
    fn live_check_against_github() {
        let r = check().expect("GitHub erreichbar");
        println!(
            "neueste Version: {:?}",
            r.map(|a| version_string(a.version))
        );
    }
}
