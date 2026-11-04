// SPDX-License-Identifier: GPL-3.0-or-later
//! `gameclipd.exe`: GUI-Subsystem ohne Konsolenfenster; alle Logik in der Bibliothek.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    std::process::exit(gameclipd::service::run());
}

#[cfg(not(windows))]
fn main() {}
