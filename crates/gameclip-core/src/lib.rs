// SPDX-License-Identifier: GPL-3.0-or-later
//! Plattformneutrale Kernlogik von GameClip.
//!
//! Dieses Crate enthält keinen Windows-Code. Alles hier ist reine Logik, die ohne
//! Hardware getestet werden kann und später von der Linux-Version wiederverwendet wird.

pub mod export_plan;
pub mod forecast;
pub mod ids;
pub mod keymap;
pub mod naming;
pub mod retention;
pub mod settings;
pub mod state;
