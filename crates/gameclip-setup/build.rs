// SPDX-License-Identifier: GPL-3.0-or-later
//! Bettet das Paket ein, das `GAMECLIP_SETUP_MSIX` nennt, und das Zertifikat aus
//! `GAMECLIP_SETUP_CER`. Ohne die Variablen (normaler Workspace-Build, CI) entsteht ein Setup
//! ohne Inhalt, das nur eine Fehlermeldung zeigt.
use std::path::{Path, PathBuf};

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rerun-if-env-changed=GAMECLIP_SETUP_MSIX");
    println!("cargo:rerun-if-env-changed=GAMECLIP_SETUP_CER");
    println!("cargo:rerun-if-changed=setup.manifest");

    let msix = std::env::var_os("GAMECLIP_SETUP_MSIX").map(PathBuf::from);
    let cer = std::env::var_os("GAMECLIP_SETUP_CER").map(PathBuf::from);
    let name = match (&msix, &cer) {
        (Some(m), Some(c)) => {
            copy(m, &out.join("payload.msix"));
            copy(c, &out.join("payload.cer"));
            let name = m.file_name().unwrap().to_string_lossy().into_owned();
            assert!(
                name.starts_with("GameClip_") && name.ends_with("_x64.msix"),
                "GAMECLIP_SETUP_MSIX muss auf GameClip_<Version>_x64.msix zeigen, nicht {name}"
            );
            name
        }
        (None, None) => {
            std::fs::write(out.join("payload.msix"), b"").unwrap();
            std::fs::write(out.join("payload.cer"), b"").unwrap();
            String::new()
        }
        _ => panic!("GAMECLIP_SETUP_MSIX und GAMECLIP_SETUP_CER nur zusammen setzen"),
    };
    println!("cargo:rustc-env=GAMECLIP_SETUP_NAME={name}");

    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let manifest =
            Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("setup.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}

fn copy(from: &Path, to: &Path) {
    println!("cargo:rerun-if-changed={}", from.display());
    std::fs::copy(from, to).unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", from.display()));
}
