// SPDX-License-Identifier: GPL-3.0-or-later
//! Zeitmessung je Abschnitt (nur zur Analyse): mit `GAMECLIP_WORKER_TIMING=1` schreibt
//! `main.rs` die Summen nach stderr. GPU-Befehle laufen asynchron; ihre Kosten zeigen sich dort,
//! wo auf sie gewartet wird.

use std::cell::RefCell;
use std::time::{Duration, Instant};

thread_local! {
    static SUMS: RefCell<Vec<(&'static str, Duration, u64)>> = const { RefCell::new(Vec::new()) };
}

/// Misst `f` unter `name`.
pub fn time<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    add(name, start.elapsed());
    out
}

pub fn add(name: &'static str, d: Duration) {
    SUMS.with(|s| {
        let mut s = s.borrow_mut();
        match s.iter_mut().find(|(n, _, _)| *n == name) {
            Some((_, sum, count)) => {
                *sum += d;
                *count += 1;
            }
            None => s.push((name, d, 1)),
        }
    });
}

/// Summen als Text, z. B. für stderr.
pub fn report() -> String {
    SUMS.with(|s| {
        s.borrow()
            .iter()
            .map(|(n, d, c)| format!("{n:<16} {:>8.1} ms  ({c}×)", d.as_secs_f64() * 1000.0))
            .collect::<Vec<_>>()
            .join("\n")
    })
}
