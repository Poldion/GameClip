// SPDX-License-Identifier: GPL-3.0-or-later
//! GameClip-Dienst (Design, Dateien 40–55). Alle Module sind öffentlich, damit die
//! Integrationstests sie nutzen können; `main.rs` ruft nur `run()`.

#![cfg(windows)]

pub mod capture;
pub mod cli;
pub mod clipboard;
pub mod diagnostics;
pub mod foreground;
pub mod frames;
pub mod index;
pub mod ipc_server;
pub mod jobs;
pub mod logging;
pub mod msgthread;
pub mod paths;
pub mod retention_exec;
pub mod save;
pub mod service;
pub mod sound;
pub mod toast;
pub mod tray;
pub mod watcher;
