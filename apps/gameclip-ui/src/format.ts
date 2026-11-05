// SPDX-License-Identifier: GPL-3.0-or-later
// Zahlen und Zeiten für die Anzeige, deutsch und dezimal (1 GB = 10^9 Byte, wie im Dienst).

const nf1 = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 1, minimumFractionDigits: 1 });
const nf0 = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 0 });

/** „142 MB“, „7,2 GB“. */
export function bytes(n: number): string {
  if (n >= 1e9) return `${nf1.format(n / 1e9)} GB`;
  if (n >= 1e6) return `${nf0.format(n / 1e6)} MB`;
  if (n >= 1e3) return `${nf0.format(n / 1e3)} kB`;
  return `${n} B`;
}

/** „7,2 / 10 GB“: belegter Platz im Verhältnis zum Limit, in der Einheit des Limits. */
export function usage(used: number, limit: number): string {
  if (limit >= 1e9) return `${nf1.format(used / 1e9)} / ${nf0.format(limit / 1e9)} GB`;
  return `${nf0.format(used / 1e6)} / ${nf0.format(limit / 1e6)} MB`;
}

/** „1:00“, „12:05“, „1:02:03“. */
export function duration(us: number): string {
  const total = Math.round(us / 1_000_000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const ss = s.toString().padStart(2, "0");
  return h > 0 ? `${h}:${m.toString().padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Datum und Uhrzeit in Ortszeit, „05.11.2026, 21:14“. */
export function dateTime(rfc3339: string): string {
  const d = new Date(rfc3339);
  return Number.isNaN(d.getTime())
    ? rfc3339
    : d.toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" });
}

/** Dateiname aus einem Windows-Pfad. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
