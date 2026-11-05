// SPDX-License-Identifier: GPL-3.0-or-later
//! Alle Windows-Aufrufe der Oberfläche an einer Stelle (Design, Datei 58). `gameclip-ui` selbst
//! nennt `windows` nicht. Freie Funktionen, keine Traits.

#[cfg(windows)]
pub mod clipboard;
#[cfg(windows)]
pub mod drag;
#[cfg(windows)]
pub mod media;
#[cfg(windows)]
pub mod package;
#[cfg(windows)]
pub mod pipe_client;
#[cfg(windows)]
pub mod reveal;
#[cfg(windows)]
pub mod service;
#[cfg(windows)]
pub mod share;
#[cfg(windows)]
pub mod thumbnail;
