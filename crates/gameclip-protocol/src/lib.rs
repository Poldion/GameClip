// SPDX-License-Identifier: GPL-3.0-or-later
//! IPC-Vertrag von GameClip: Rahmung, Nachrichten, Fehlercodes und Worker-Aufträge.
//!
//! Kein Windows-Code. Die Nachrichtentabelle steht im Design (Datei 24) und in
//! `docs/protocol.md`; Feldnamen sind ab v1 fest.

pub mod error;
pub mod framing;
pub mod messages;
pub mod version;
pub mod worker;
