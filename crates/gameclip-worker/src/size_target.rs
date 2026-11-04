// SPDX-License-Identifier: GPL-3.0-or-later
//! Export auf Zielgröße (Design, „Zielgröße“): Plan vorab, kodieren, Größe prüfen, höchstens ein
//! Korrekturversuch, dann `TooLarge`.

use gameclip_core::export_plan::{ExportPlan, PlanError, correct, plan};
use gameclip_protocol::worker::WorkerError;

use crate::trim::{Fail, Output};

/// Plan für die Zielgröße; `TooSmall` endet sofort mit `TooLarge` (ohne zu kodieren).
pub fn plan_for(
    duration_us: u64,
    target_bytes: u64,
    source: (u32, u32, u32),
) -> Result<ExportPlan, Fail> {
    plan(duration_us, target_bytes, source).map_err(|PlanError::TooSmall { min_target_bytes }| {
        Fail(
            WorkerError::TooLarge,
            format!("Zielgröße zu klein; nötig wären mindestens {min_target_bytes} Bytes"),
        )
    })
}

/// Kodiert mit `encode(video_bps)` und prüft die Größe; höchstens eine Korrektur.
pub fn with_correction(
    plan: &ExportPlan,
    target_bytes: u64,
    mut encode: impl FnMut(u64) -> Result<Output, Fail>,
) -> Result<Output, Fail> {
    let first = encode(plan.video_bps)?;
    if first.bytes <= target_bytes {
        return Ok(first);
    }
    let retry = encode(correct(plan.video_bps, target_bytes, first.bytes))?;
    if retry.bytes <= target_bytes {
        Ok(retry)
    } else {
        Err(Fail(
            WorkerError::TooLarge,
            format!("{} Bytes nach Korrektur, Ziel {target_bytes}", retry.bytes),
        ))
    }
}
