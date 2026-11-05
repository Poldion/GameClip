// SPDX-License-Identifier: GPL-3.0-or-later
// Hotkey wählen (Design, Datei 65): Solange der Dialog offen ist, gibt der Dienst die Taste frei
// (`SuspendHotkey`), damit der Druck hier ankommt. Jede Taste wird mit `TestRegister` geprüft.
// Bestätigen speichert über `SetSettings`; Schließen hebt `SuspendHotkey` in jedem Fall auf.

import { useEffect, useRef, useState } from "react";
import type { Hotkey } from "../generated/Hotkey";
import { hotkeyError, hotkeyFromEvent, hotkeyLabel } from "../hotkey";
import { IpcError } from "../ipc";
import { useStore } from "../store";
import css from "./HotkeyPicker.module.css";

const EXTRA_KEYS = Array.from({ length: 12 }, (_, i) => `F${13 + i}`);

type Check = { kind: "idle" } | { kind: "checking" } | { kind: "free" } | { kind: "taken" } | { kind: "invalid"; reason: string };

export function HotkeyPicker(props: { value: Hotkey; onDone: (saved: Hotkey | null) => void; inline?: boolean }) {
  const store = useStore();
  const [key, setKey] = useState<Hotkey>(props.value);
  const [check, setCheck] = useState<Check>({ kind: "idle" });
  const [saving, setSaving] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void store.ipc.suspendHotkey(true);
    box.current?.focus();
    return () => {
      void store.ipc.suspendHotkey(false);
    };
  }, [store]);

  async function choose(h: Hotkey) {
    setKey(h);
    const reason = hotkeyError(h);
    if (reason) {
      setCheck({ kind: "invalid", reason });
      return;
    }
    setCheck({ kind: "checking" });
    try {
      const r = await store.ipc.testRegister(h);
      setCheck(r.free ? { kind: "free" } : { kind: "taken" });
    } catch (e) {
      setCheck({ kind: "invalid", reason: e instanceof Error ? e.message : String(e) });
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Tab") return;
    e.preventDefault();
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey) {
      props.onDone(null);
      return;
    }
    const h = hotkeyFromEvent(e.nativeEvent);
    if (h) void choose(h);
  }

  async function confirm() {
    setSaving(true);
    try {
      const r = await store.ipc.setSettings({ hotkey: key });
      store.setSettings(r.settings);
      props.onDone(key);
    } catch (e) {
      const reason = e instanceof IpcError && e.fields[0] ? e.fields[0].reason : String((e as Error).message ?? e);
      setCheck({ kind: "invalid", reason });
    } finally {
      setSaving(false);
    }
  }

  const blocked = check.kind === "invalid" || check.kind === "checking" || saving;
  return (
    <div className={props.inline ? css.inline : css.dialog} role="dialog" aria-label="Hotkey wählen">
      <div
        ref={box}
        className={css.capture}
        tabIndex={0}
        onKeyDown={onKeyDown}
        data-testid="hotkey-capture"
        aria-label="Taste drücken"
      >
        <span className={css.key}>{hotkeyLabel(key)}</span>
        <span className={css.hint}>Taste drücken (Strg/Alt nur mit Buchstaben oder Ziffern)</span>
      </div>
      <label className={css.row}>
        Weitere Tasten:
        <select
          value={EXTRA_KEYS.includes(key.code) ? key.code : ""}
          onChange={(e) => e.target.value && void choose({ code: e.target.value, ctrl: false, alt: false, shift: false })}
        >
          <option value="">–</option>
          {EXTRA_KEYS.map((k) => (
            <option key={k} value={k}>
              {k}
            </option>
          ))}
        </select>
      </label>
      <p className={css.status} data-kind={check.kind} role="status">
        {check.kind === "checking" && "Prüfe …"}
        {check.kind === "free" && `${hotkeyLabel(key)} ist frei.`}
        {check.kind === "taken" && `${hotkeyLabel(key)} ist von einem anderen Programm belegt. Speichern ist möglich, die Taste wirkt aber erst, wenn sie frei wird.`}
        {check.kind === "invalid" && check.reason}
      </p>
      <p className={css.warn}>
        Manche Spiele nutzen dieselbe Taste. Die Taste wirkt in allen Programmen, solange GameClip läuft.
      </p>
      <div className={css.buttons}>
        <button onClick={() => props.onDone(null)}>Abbrechen</button>
        <button className="primary" disabled={blocked} onClick={() => void confirm()}>
          Übernehmen
        </button>
      </div>
    </div>
  );
}
