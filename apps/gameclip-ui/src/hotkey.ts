// SPDX-License-Identifier: GPL-3.0-or-later
// Dieselbe Regel wie gameclip-core::keymap::to_vk, damit die Oberfläche sofort warnen kann.
// Der Dienst prüft beim Speichern noch einmal.

import type { Hotkey } from "./generated/Hotkey";

/** Fehlertext oder `null`, wenn die Taste erlaubt ist. */
export function hotkeyError(h: Hotkey): string | null {
  const f = /^F(\d+)$/.exec(h.code);
  if (f) {
    const n = Number(f[1]);
    if (n === 12) return "F12 ist für Debugger reserviert";
    if (n >= 1 && n <= 24) return null;
    return `unbekannte Taste: ${h.code}`;
  }
  if (/^Key[A-Z]$/.test(h.code) || /^Digit[0-9]$/.test(h.code)) {
    return h.ctrl || h.alt ? null : "Buchstaben und Ziffern nur zusammen mit Strg oder Alt";
  }
  return `unbekannte Taste: ${h.code}`;
}

/** Anzeige wie „Strg+Alt+K“ oder „F8“. */
export function hotkeyLabel(h: Hotkey): string {
  const key = h.code.replace(/^Key/, "").replace(/^Digit/, "");
  return [h.ctrl && "Strg", h.alt && "Alt", h.shift && "Umschalt", key].filter(Boolean).join("+");
}

/** Taste aus einem Tastendruck; reine Modifier ergeben `null`. */
export function hotkeyFromEvent(e: Pick<KeyboardEvent, "code" | "ctrlKey" | "altKey" | "shiftKey">): Hotkey | null {
  if (/^(Control|Alt|Shift|Meta)(Left|Right)$/.test(e.code)) return null;
  return { code: e.code, ctrl: e.ctrlKey, alt: e.altKey, shift: e.shiftKey };
}
