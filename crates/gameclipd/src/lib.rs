// SPDX-License-Identifier: GPL-3.0-or-later
//! GameClip-Dienst (Design, Dateien 40–55). Alle Module sind öffentlich, damit die
//! Integrationstests sie nutzen können; `main.rs` ruft nur `run()`.

#![cfg(windows)]

pub mod capture;
pub mod foreground;
pub mod index;
pub mod paths;
pub mod save;
