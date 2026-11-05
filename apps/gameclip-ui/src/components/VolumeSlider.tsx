// SPDX-License-Identifier: GPL-3.0-or-later
// Lautstärke −30 bis +12 dB, Doppelklick setzt 0 dB (Design, Datei 65).

import css from "./VolumeSlider.module.css";

export const GAIN_MIN = -30;
export const GAIN_MAX = 12;

export function gainLabel(db: number): string {
  if (db === 0) return "0 dB";
  return `${db > 0 ? "+" : "−"}${Math.abs(db).toLocaleString("de-DE", { maximumFractionDigits: 1 })} dB`;
}

export function VolumeSlider(props: { label: string; value: number; onChange: (db: number) => void; disabled?: boolean }) {
  return (
    <label className={css.row}>
      <span className={css.label}>{props.label}</span>
      <input
        type="range"
        min={GAIN_MIN}
        max={GAIN_MAX}
        step={0.5}
        value={props.value}
        disabled={props.disabled}
        aria-label={props.label}
        onChange={(e) => props.onChange(Number(e.target.value))}
        onDoubleClick={() => props.onChange(0)}
      />
      <span className={css.value}>{gainLabel(props.value)}</span>
    </label>
  );
}
