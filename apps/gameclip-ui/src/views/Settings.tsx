// SPDX-License-Identifier: GPL-3.0-or-later
// Einstellungen (Design, Datei 64): Aufnahme, Audio, Hotkey, Speicher, Töne, Autostart, Diagnose.
// Jede Änderung geht sofort als Merge-Patch an den Dienst; Schieberegler erst beim Loslassen.
// Lehnt der Dienst ab, steht der Grund am Feld.

import { useEffect, useState } from "react";
import type { ListDevicesReply } from "../generated/ListDevicesReply";
import type { Settings as SettingsT } from "../generated/Settings";
import { ForecastLabel } from "../components/ForecastLabel";
import { HotkeyPicker } from "../components/HotkeyPicker";
import { VolumeSlider } from "../components/VolumeSlider";
import { duration } from "../format";
import { hotkeyLabel } from "../hotkey";
import { type Autostart, IpcError } from "../ipc";
import { useSelect, useStore } from "../store";
import css from "./Settings.module.css";

type Patch = Record<string, unknown>;

/** Setzt `value` unter `path` (z. B. "audio.gain_db.mic") in einem neuen Patch-Objekt. */
export function patchAt(path: string, value: unknown): Patch {
  const keys = path.split(".");
  const root: Patch = {};
  let cur = root;
  keys.slice(0, -1).forEach((k) => {
    cur[k] = {};
    cur = cur[k] as Patch;
  });
  cur[keys[keys.length - 1]] = value;
  return root;
}

function setAt(s: SettingsT, path: string, value: unknown): SettingsT {
  const copy = structuredClone(s) as unknown as Patch;
  const keys = path.split(".");
  let cur = copy;
  keys.slice(0, -1).forEach((k) => (cur = cur[k] as Patch));
  cur[keys[keys.length - 1]] = value;
  return copy as unknown as SettingsT;
}

const AUTOSTART_TEXT: Record<Autostart, string> = {
  enabled: "GameClip startet mit Windows.",
  disabled: "GameClip startet nicht mit Windows.",
  disabled_by_user: "Im Task-Manager unter „Autostart“ abgeschaltet; dort wieder einschalten.",
  disabled_by_policy: "Durch eine Richtlinie gesperrt.",
  unavailable: "Nur in der installierten App verfügbar.",
};

export function Settings() {
  const store = useStore();
  const saved = useSelect((s) => s.settings);
  const readOnly = useSelect((s) => s.readOnly);
  const [draft, setDraft] = useState<SettingsT | null>(saved);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [devices, setDevices] = useState<ListDevicesReply | null>(null);
  const [autostart, setAutostartState] = useState<Autostart | null>(null);
  const [pickHotkey, setPickHotkey] = useState(false);

  useEffect(() => setDraft(saved), [saved]);
  useEffect(() => {
    void store.ipc.listDevices().then(setDevices).catch(() => setDevices({ monitors: [], mics: [] }));
    void store.ipc.getAutostart().then(setAutostartState);
  }, [store]);

  if (!draft) return null;

  /** Ändert die Anzeige; `commit` schickt es an den Dienst. */
  const change = (path: string, value: unknown, commit = true) => {
    setDraft((d) => (d ? setAt(d, path, value) : d));
    if (commit) void save(path, value);
  };

  async function save(path: string, value: unknown) {
    try {
      const r = await store.ipc.setSettings(patchAt(path, value) as never);
      store.setSettings(r.settings);
      setErrors((e) => {
        const { [path]: _, ...rest } = e;
        return rest;
      });
    } catch (e) {
      if (e instanceof IpcError && e.fields.length) {
        setErrors((old) => ({ ...old, ...Object.fromEntries(e.fields.map((f) => [f.key, f.reason])) }));
      } else {
        store.notify(String((e as Error).message ?? e), "error");
      }
      setDraft(store.get().settings);
    }
  }

  const err = (key: string) => errors[key] && <span className={css.error}>{errors[key]}</span>;
  const c = draft.capture;
  const a = draft.audio;
  const st = draft.storage;
  const commitOnRelease = (path: string) => ({
    onPointerUp: (e: React.PointerEvent<HTMLInputElement>) => void save(path, Number(e.currentTarget.value)),
    onKeyUp: (e: React.KeyboardEvent<HTMLInputElement>) => void save(path, Number(e.currentTarget.value)),
  });

  return (
    <div className={css.page}>
      {readOnly && (
        <p className={css.banner}>
          Die Einstellungen stammen von einer neueren GameClip-Version und werden nur gelesen.
        </p>
      )}
      <fieldset className={css.section} disabled={readOnly}>
        <h2>Aufnahme</h2>
        <label className={css.row}>
          <span>Länge</span>
          <input
            type="range"
            min={15}
            max={300}
            step={5}
            value={c.buffer_seconds}
            onChange={(e) => change("capture.buffer_seconds", Number(e.target.value), false)}
            {...commitOnRelease("capture.buffer_seconds")}
            aria-label="Länge"
          />
          <span className={css.value}>{duration(c.buffer_seconds * 1_000_000)}</span>
        </label>
        {err("capture.buffer_seconds")}
        <label className={css.row}>
          <span>Auflösung</span>
          <select value={c.resolution} onChange={(e) => change("capture.resolution", e.target.value)}>
            {(["720p", "1080p", "1440p", "2160p"] as const).map((r) => (
              <option key={r}>{r}</option>
            ))}
          </select>
        </label>
        <label className={css.row}>
          <span>Bildrate</span>
          <select value={c.fps} onChange={(e) => change("capture.fps", Number(e.target.value))}>
            <option value={30}>30 fps</option>
            <option value={60}>60 fps</option>
          </select>
        </label>
        <label className={css.row}>
          <span>Qualität</span>
          <select value={c.quality} onChange={(e) => change("capture.quality", e.target.value)}>
            <option value="low">Niedrig</option>
            <option value="medium">Mittel</option>
            <option value="high">Hoch</option>
          </select>
        </label>
        <label className={css.row}>
          <span>Monitor</span>
          <select value={c.monitor ?? ""} onChange={(e) => change("capture.monitor", e.target.value || null)}>
            <option value="">Hauptmonitor</option>
            {devices?.monitors.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name} ({m.width}×{m.height})
              </option>
            ))}
          </select>
        </label>
        <label className={css.check}>
          <input type="checkbox" checked={c.cursor} onChange={(e) => change("capture.cursor", e.target.checked)} />
          Mauszeiger aufnehmen
        </label>
        <ForecastLabel settings={draft} />
      </fieldset>

      <fieldset className={css.section} disabled={readOnly}>
        <h2>Audio</h2>
        <label className={css.check}>
          <input type="checkbox" checked={a.mic_enabled} onChange={(e) => change("audio.mic_enabled", e.target.checked)} />
          Mikrofon aufnehmen
        </label>
        <label className={css.row}>
          <span>Mikrofon</span>
          <select value={a.mic_device ?? ""} disabled={!a.mic_enabled} onChange={(e) => change("audio.mic_device", e.target.value || null)}>
            <option value="">Windows-Standard</option>
            {devices?.mics.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        </label>
        <label className={css.check}>
          <input type="checkbox" checked={a.discord_enabled} onChange={(e) => change("audio.discord_enabled", e.target.checked)} />
          Discord als eigene Spur aufnehmen
        </label>
        <VolumeSlider label="Spiel und System" value={a.gain_db.system} onChange={(v) => change("audio.gain_db.system", v)} />
        <VolumeSlider label="Mikrofon" value={a.gain_db.mic} disabled={!a.mic_enabled} onChange={(v) => change("audio.gain_db.mic", v)} />
        <VolumeSlider label="Discord" value={a.gain_db.discord} disabled={!a.discord_enabled} onChange={(v) => change("audio.gain_db.discord", v)} />
        {err("audio.gain_db.system")}
      </fieldset>

      <fieldset className={css.section} disabled={readOnly}>
        <h2>Hotkey</h2>
        {pickHotkey ? (
          <HotkeyPicker inline value={draft.hotkey} onDone={() => setPickHotkey(false)} />
        ) : (
          <div className={css.row}>
            <span>Clip speichern</span>
            <span className={css.key}>{hotkeyLabel(draft.hotkey)}</span>
            <button onClick={() => setPickHotkey(true)}>Ändern …</button>
          </div>
        )}
        {err("hotkey")}
      </fieldset>

      <fieldset className={css.section} disabled={readOnly}>
        <h2>Speicher</h2>
        {(
          [
            ["storage.clip_folder", "Clip-Ordner", st.clip_folder],
            ["storage.export_folder", "Export-Ordner", st.export_folder],
          ] as const
        ).map(([key, label, value]) => (
          <div key={key}>
            <div className={css.row}>
              <span>{label}</span>
              <span className={css.path} title={value ?? undefined}>
                {value ?? (key === "storage.clip_folder" ? "Standard (Videos\\GameClip)" : "Standard (Videos\\GameClip Export)")}
              </span>
              <button
                onClick={() =>
                  void store.ipc.pickFolder(value).then((dir) => {
                    if (dir) change(key, dir);
                  })
                }
              >
                Ändern …
              </button>
              {value && <button onClick={() => change(key, null)}>Standard</button>}
            </div>
            {err(key)}
          </div>
        ))}
        {(
          [
            ["storage.clip_limit_bytes", "Limit Clips", st.clip_limit_bytes],
            ["storage.export_limit_bytes", "Limit Exporte", st.export_limit_bytes],
          ] as const
        ).map(([key, label, value]) => (
          <div key={key}>
            <label className={css.row}>
              <span>{label}</span>
              <input
                type="number"
                min={1}
                max={2000}
                step={1}
                defaultValue={Math.round(value / 1e9)}
                key={value}
                onBlur={(e) => {
                  const gb = Number(e.target.value);
                  if (gb * 1e9 !== value) change(key, Math.round(gb * 1e9));
                }}
                aria-label={label}
              />
              <span className={css.value}>GB</span>
            </label>
            {err(key)}
          </div>
        ))}
        <p className={css.hint}>
          Ist ein Limit erreicht, löscht GameClip die ältesten eigenen Clips ohne Stern endgültig. Andere Dateien im Ordner bleiben unangetastet.
        </p>
      </fieldset>

      <fieldset className={css.section} disabled={readOnly}>
        <h2>Töne</h2>
        <label className={css.check}>
          <input type="checkbox" checked={draft.sound.enabled} onChange={(e) => change("sound.enabled", e.target.checked)} />
          Ton beim Speichern
        </label>
        <label className={css.row}>
          <span>Lautstärke</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={draft.sound.volume}
            disabled={!draft.sound.enabled}
            onChange={(e) => change("sound.volume", Number(e.target.value), false)}
            {...commitOnRelease("sound.volume")}
            aria-label="Lautstärke"
          />
          <span className={css.value}>{Math.round(draft.sound.volume * 100)} %</span>
        </label>
      </fieldset>

      <fieldset className={css.section}>
        <h2>Autostart</h2>
        <label className={css.check}>
          <input
            type="checkbox"
            checked={autostart === "enabled"}
            disabled={autostart === null || autostart === "unavailable" || autostart === "disabled_by_policy"}
            onChange={(e) => void store.ipc.setAutostart(e.target.checked).then(setAutostartState)}
          />
          Mit Windows starten
        </label>
        {autostart && <p className={css.hint}>{AUTOSTART_TEXT[autostart]}</p>}
      </fieldset>

      <fieldset className={css.section}>
        <h2>Diagnose</h2>
        <p className={css.hint}>
          Kopiert Windows-Version, Grafikkarten, Encoder, Geräte, Einstellungen und die letzten Logzeilen in die Zwischenablage. Dein Benutzername wird dabei ersetzt.
        </p>
        <div>
          <button
            onClick={() =>
              void store.ipc
                .copyDiagnostics()
                .then((r) => navigator.clipboard.writeText(r.text))
                .then(() => store.notify("Diagnose kopiert."))
                .catch((e) => store.notify(String((e as Error).message ?? e), "error"))
            }
          >
            Diagnose kopieren
          </button>
        </div>
      </fieldset>
    </div>
  );
}
