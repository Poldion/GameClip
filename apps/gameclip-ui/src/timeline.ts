// SPDX-License-Identifier: GPL-3.0-or-later
// Bildgenaue Zeitachse des Editors (Design, Datei 65, „Kürzen und Export“ → Zeitachse).
//
// Alle Zeiten sind Mikrosekunden Anzeigezeit wie `<video>.currentTime × 10^6`. `frames_us` kommt
// von `GetFrameTimes` (aufsteigend). Ein Schnitt umfasst die Bilder mit in_us ≤ t < out_us.

/** Spielraum für Rundung zwischen `currentTime` (Gleitkomma) und den ganzzahligen Bildzeiten. */
const SLACK_US = 1000;

/** Index des Bildes, das zur Zeit `currentTimeS` angezeigt wird (Binärsuche). */
export function toFrameIndex(currentTimeS: number, framesUs: readonly number[]): number {
  if (framesUs.length === 0) return 0;
  const t = Math.round(currentTimeS * 1_000_000) + SLACK_US;
  // Größtes i mit frames[i] ≤ t.
  let lo = 0;
  let hi = framesUs.length - 1;
  if (t < framesUs[0]) return 0;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (framesUs[mid] <= t) lo = mid;
    else hi = mid - 1;
  }
  return lo;
}

/** `currentTime` für eine Zeit in µs. */
export function toCurrentTime(us: number): number {
  return us / 1_000_000;
}

/**
 * `currentTime`, die sicher Bild `index` zeigt: etwas hinter seinem Beginn, aber vor dem nächsten
 * (Browser runden beim Spulen unterschiedlich).
 */
export function seekTimeForFrame(index: number, framesUs: readonly number[], durationUs: number): number {
  const i = Math.max(0, Math.min(index, framesUs.length - 1));
  const start = framesUs[i] ?? 0;
  const next = framesUs[i + 1] ?? durationUs;
  return toCurrentTime(start + Math.min(SLACK_US, Math.max(0, (next - start) / 2)));
}

/** Start eines Schnitts, der mit Bild `firstIndex` beginnt. */
export function inUs(firstIndex: number, framesUs: readonly number[]): number {
  return framesUs[Math.max(0, Math.min(firstIndex, framesUs.length - 1))] ?? 0;
}

/**
 * Ausschließliches Ende eines Schnitts, dessen letztes Bild `lastIndex` ist: die Zeit des Bildes
 * danach, beim letzten Bild des Clips `durationUs`.
 */
export function outUs(lastIndex: number, framesUs: readonly number[], durationUs: number): number {
  const next = lastIndex + 1;
  return next < framesUs.length ? framesUs[next] : durationUs;
}

/** Bildindex für eine Position 0…1 auf der Leiste (Ziehen eines Griffs). */
export function indexAtFraction(fraction: number, framesUs: readonly number[], durationUs: number): number {
  const f = Math.max(0, Math.min(1, fraction));
  return toFrameIndex((f * durationUs) / 1_000_000, framesUs);
}

/** Position 0…1 eines Bildes auf der Leiste. */
export function fractionOf(us: number, durationUs: number): number {
  return durationUs > 0 ? Math.max(0, Math.min(1, us / durationUs)) : 0;
}
