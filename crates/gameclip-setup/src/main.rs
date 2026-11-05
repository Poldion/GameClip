// SPDX-License-Identifier: GPL-3.0-or-later
//! `GameClip-Setup.exe`: eine Datei statt Paket, Zertifikat und Skript.
//!
//! Macht dasselbe wie `packaging/Install-GameClip.ps1`: Ist das Release-Zertifikat noch nicht
//! unter „Lokaler Computer\Vertrauenswürdige Personen“ eingetragen, startet sich das Setup einmal
//! mit `--import-cert` als Administrator (UAC) und trägt es ein. Danach installiert es das
//! eingebettete MSIX für den angemeldeten Benutzer. Mehr ändert es nicht.

use std::io::{BufRead, Write};
use std::path::Path;

use windows::Foundation::Uri;
use windows::Management::Deployment::{DeploymentOptions, PackageManager};
use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
use windows::Win32::Security::Cryptography::{
    CERT_FIND_EXISTING, CERT_OPEN_STORE_FLAGS, CERT_STORE_ADD_REPLACE_EXISTING,
    CERT_STORE_OPEN_EXISTING_FLAG, CERT_STORE_PROV_SYSTEM_W, CERT_STORE_READONLY_FLAG,
    CERT_SYSTEM_STORE_LOCAL_MACHINE, CertAddEncodedCertificateToStore, CertCloseStore,
    CertCreateCertificateContext, CertFindCertificateInStore, CertFreeCertificateContext,
    CertOpenStore, HCERTSTORE, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
};
use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows_core::{HSTRING, w};

const MSIX: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.msix"));
const CER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.cer"));
const MSIX_NAME: &str = env!("GAMECLIP_SETUP_NAME");

const IMPORT_ARG: &str = "--import-cert";

fn main() {
    if std::env::args().nth(1).as_deref() == Some(IMPORT_ARG) {
        // Erhöhter Aufruf: nur das Zertifikat eintragen, ohne Fenster und ohne Rückfrage.
        std::process::exit(match import_cert() {
            Ok(()) => 0,
            Err(e) => e.code().0,
        });
    }

    println!("GameClip-Setup {}", version_label());
    let result = run();
    match &result {
        Ok(()) => println!("\nFertig. GameClip im Startmenü öffnen."),
        Err(e) => println!("\nFehler: {e}"),
    }
    print!("Enter zum Schließen ");
    let _ = std::io::stdout().flush();
    let _ = std::io::stdin().lock().read_line(&mut String::new());
    std::process::exit(if result.is_ok() { 0 } else { 1 });
}

/// `GameClip_0.2.0.0_x64.msix` → `0.2.0.0`.
fn version_label() -> &'static str {
    MSIX_NAME
        .strip_prefix("GameClip_")
        .and_then(|s| s.strip_suffix("_x64.msix"))
        .unwrap_or("(ohne Paket)")
}

fn run() -> Result<(), String> {
    if MSIX.is_empty() || CER.is_empty() {
        return Err("Dieses Setup wurde ohne Paket gebaut.".into());
    }
    if !cert_trusted().map_err(|e| format!("Zertifikatsspeicher nicht lesbar: {e}"))? {
        println!(
            "Das Zertifikat von GameClip wird eingetragen – Windows fragt nach Administratorrechten …"
        );
        import_cert_elevated()?;
        if !cert_trusted().unwrap_or(false) {
            return Err("Das Zertifikat ist nach dem Eintragen nicht im Speicher.".into());
        }
    }

    let dir = std::env::temp_dir().join("GameClip-Setup");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(MSIX_NAME);
    std::fs::write(&path, MSIX).map_err(|e| format!("{}: {e}", path.display()))?;

    // Läuft GameClip, beendet Windows es für das Update.
    println!("Installiere {MSIX_NAME} …");
    let result = install(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
    result.map_err(|e| format!("Installation fehlgeschlagen: {}", e.message()))
}

fn install(path: &Path) -> windows::core::Result<()> {
    let uri = Uri::CreateUri(&HSTRING::from(file_uri(path)))?;
    PackageManager::new()?
        .AddPackageAsync(&uri, None, DeploymentOptions::ForceApplicationShutdown)?
        .join()?;
    Ok(())
}

/// `C:\a b\x.msix` → `file:///C:/a%20b/x.msix`.
fn file_uri(path: &Path) -> String {
    let mut s = String::from("file:///");
    for c in path.to_string_lossy().chars() {
        match c {
            '\\' => s.push('/'),
            ' ' => s.push_str("%20"),
            '#' => s.push_str("%23"),
            '%' => s.push_str("%25"),
            c => s.push(c),
        }
    }
    s
}

fn open_store(read_only: bool) -> windows::core::Result<HCERTSTORE> {
    let mut flags = CERT_SYSTEM_STORE_LOCAL_MACHINE | CERT_STORE_OPEN_EXISTING_FLAG.0;
    if read_only {
        flags |= CERT_STORE_READONLY_FLAG.0;
    }
    unsafe {
        CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            Default::default(),
            None,
            CERT_OPEN_STORE_FLAGS(flags),
            Some(w!("TrustedPeople").as_ptr() as *const _),
        )
    }
}

fn cert_trusted() -> windows::core::Result<bool> {
    cert_in_store(CER)
}

/// Steht `cer` (DER) unter „Lokaler Computer\Vertrauenswürdige Personen“?
fn cert_in_store(cer: &[u8]) -> windows::core::Result<bool> {
    unsafe {
        let cert = CertCreateCertificateContext(X509_ASN_ENCODING | PKCS_7_ASN_ENCODING, cer);
        if cert.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        let store = match open_store(true) {
            Ok(s) => s,
            Err(e) => {
                let _ = CertFreeCertificateContext(Some(cert));
                return Err(e);
            }
        };
        let found = CertFindCertificateInStore(
            store,
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            0,
            CERT_FIND_EXISTING,
            Some(cert as *const _),
            None,
        );
        let trusted = !found.is_null();
        if trusted {
            let _ = CertFreeCertificateContext(Some(found));
        }
        let _ = CertFreeCertificateContext(Some(cert));
        let _ = CertCloseStore(Some(store), 0);
        Ok(trusted)
    }
}

fn import_cert() -> windows::core::Result<()> {
    unsafe {
        let store = open_store(false)?;
        let r = CertAddEncodedCertificateToStore(
            Some(store),
            X509_ASN_ENCODING | PKCS_7_ASN_ENCODING,
            CER,
            CERT_STORE_ADD_REPLACE_EXISTING,
            None,
        );
        let _ = CertCloseStore(Some(store), 0);
        r
    }
}

/// Startet dieses Programm mit `--import-cert` als Administrator und wartet darauf.
fn import_cert_elevated() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = HSTRING::from(exe.as_os_str());
    let args = HSTRING::from(IMPORT_ARG);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: windows_core::PCWSTR(exe.as_ptr()),
        lpParameters: windows_core::PCWSTR(args.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        if let Err(e) = ShellExecuteExW(&mut info) {
            return Err(if e.code() == ERROR_CANCELLED.to_hresult() {
                "Administratorrechte abgelehnt; ohne Zertifikat installiert Windows GameClip nicht."
                    .into()
            } else {
                format!("Erhöhter Start fehlgeschlagen: {}", e.message())
            });
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 0u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        if code != 0 {
            return Err(format!(
                "Zertifikat nicht eingetragen: {}",
                windows::core::Error::from_hresult(windows_core::HRESULT(code as i32)).message()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris() {
        assert_eq!(
            file_uri(Path::new(r"C:\Users\A B\Temp\GameClip_0.2.0.0_x64.msix")),
            "file:///C:/Users/A%20B/Temp/GameClip_0.2.0.0_x64.msix"
        );
    }

    #[test]
    fn reads_release_certificate_and_store() {
        let cer = include_bytes!("../../../packaging/GameClip-Release.cer");
        cert_in_store(cer).expect("Zertifikat lesbar und Speicher offen");
        assert!(cert_in_store(b"kein Zertifikat").is_err());
    }

    #[test]
    fn version_without_payload() {
        // Im Workspace-Build ist kein Paket eingebettet.
        if MSIX_NAME.is_empty() {
            assert_eq!(version_label(), "(ohne Paket)");
            assert!(run().is_err());
        }
    }
}
