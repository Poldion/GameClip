// SPDX-License-Identifier: GPL-3.0-or-later
//! Worker für Kürzen und Export (Design, „Kürzen und Export“ und Dateien 56–57).
//! `main.rs` liest den Auftrag von stdin und ruft `trim::run`.

#![cfg(windows)]

pub mod size_target;
pub mod timing;
pub mod trim;
