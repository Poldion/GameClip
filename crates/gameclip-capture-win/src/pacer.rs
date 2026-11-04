// SPDX-License-Identifier: GPL-3.0-or-later
//! Taktgeber mit festem Takt nach QPC (Design, Datei 33).
//!
//! Vor jedem Takt wird die Wartezeit neu aus `start + n / fps − jetzt` berechnet, damit sich
//! keine Rundungsfehler aufsummieren. Liegt der Thread mehr als einen Takt zurück, werden die
//! verpassten Takte übersprungen und gezählt.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateWaitableTimerExW, INFINITE, SetWaitableTimer,
    TIMER_ALL_ACCESS, WaitForSingleObject,
};

use crate::Result;

/// Aktuelle QPC-Zeit in µs. Dieselbe Zeitbasis wie `SystemRelativeTime` von WGC und
/// `pu64QPCPosition` von WASAPI.
pub fn qpc_us() -> i64 {
    let (mut count, mut freq) = (0i64, 0i64);
    unsafe {
        let _ = QueryPerformanceCounter(&mut count);
        let _ = QueryPerformanceFrequency(&mut freq);
    }
    (i128::from(count) * 1_000_000 / i128::from(freq)) as i64
}

/// Zeit des Takts `n` in µs.
pub fn tick_time_us(start_us: i64, n: u64, fps: u32) -> i64 {
    start_us + (i128::from(n) * 1_000_000 / i128::from(fps)) as i64
}

/// Nächster Takt nach `n` zur Zeit `now_us`: (neuer Takt, übersprungene Takte).
/// Übersprungen wird nur, wenn der nächste Takt mehr als eine Taktlänge zurückliegt.
pub fn next_tick(start_us: i64, n: u64, fps: u32, now_us: i64) -> (u64, u64) {
    let next = n + 1;
    let frame_us = 1_000_000 / i64::from(fps);
    let late = now_us - tick_time_us(start_us, next, fps);
    if late > frame_us {
        let skip = (late / frame_us) as u64;
        (next + skip, skip)
    } else {
        (next, 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tick {
    pub n: u64,
    pub time_us: i64,
    /// Seit dem letzten Takt übersprungene Takte (Thread lag zurück).
    pub skipped: u64,
}

pub struct Pacer {
    timer: HANDLE,
    fps: u32,
    start_us: i64,
    n: u64,
}

impl Pacer {
    /// Takt 0 liegt bei „jetzt“.
    pub fn new(fps: u32) -> Result<Self> {
        let timer = unsafe {
            CreateWaitableTimerExW(
                None,
                None,
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )?
        };
        Ok(Pacer {
            timer,
            fps,
            start_us: qpc_us(),
            n: 0,
        })
    }

    pub fn start_us(&self) -> i64 {
        self.start_us
    }

    /// Wartet bis zum nächsten Takt.
    pub fn wait(&mut self) -> Result<Tick> {
        let (n, skipped) = next_tick(self.start_us, self.n, self.fps, qpc_us());
        self.n = n;
        let target = tick_time_us(self.start_us, n, self.fps);
        let wait_us = target - qpc_us();
        if wait_us > 0 {
            // Negativ = relativ, in 100 ns.
            let due = -(wait_us * 10);
            unsafe {
                SetWaitableTimer(self.timer, &due, 0, None, None, false)?;
                WaitForSingleObject(self.timer, INFINITE);
            }
        }
        Ok(Tick {
            n,
            time_us: target,
            skipped,
        })
    }
}

impl Drop for Pacer {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.timer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_times_do_not_drift() {
        // 60 fps: 16 666,67 µs je Takt; nach 60 Takten genau 1 s.
        assert_eq!(tick_time_us(0, 60, 60), 1_000_000);
        assert_eq!(tick_time_us(0, 1, 60), 16_666);
        assert_eq!(tick_time_us(0, 3_600 * 60, 60), 3_600_000_000);
    }

    #[test]
    fn on_time_takes_next_tick() {
        assert_eq!(next_tick(0, 0, 60, 10_000), (1, 0));
        // Knapp eine Taktlänge zu spät: noch kein Überspringen.
        assert_eq!(next_tick(0, 0, 60, 16_666 + 16_000), (1, 0));
    }

    #[test]
    fn late_skips_missed_ticks() {
        // Takt 1 wäre bei 16 666 µs; jetzt 100 ms → 5 Takte übersprungen, weiter bei Takt 6.
        assert_eq!(next_tick(0, 0, 60, 100_000), (6, 5));
    }
}
