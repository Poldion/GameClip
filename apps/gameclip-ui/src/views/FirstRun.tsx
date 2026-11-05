// SPDX-License-Identifier: GPL-3.0-or-later
// Erster Start (Design, Datei 64, CUJ 1): Mikrofon bestätigen oder wechseln, Hotkey mit Testdruck,
// Clip-Ordner, dann Test-Clip. Fehlt das Media Feature Pack, steht der Hinweis vorne.

import { useEffect, useState } from "react";
import type { ListDevicesReply } from "../generated/ListDevicesReply";
import { HotkeyPicker } from "../components/HotkeyPicker";
import { hotkeyLabel } from "../hotkey";
import { useSelect, useStore } from "../store";
import css from "./FirstRun.module.css";

type Step = "media" | "mic" | "hotkey" | "folder" | "test";

export function FirstRun(props: { onDone: () => void }) {
  const store = useStore();
  const settings = useSelect((s) => s.settings);
  const [steps, setSteps] = useState<Step[] | null>(null);
  const [index, setIndex] = useState(0);
  const [devices, setDevices] = useState<ListDevicesReply | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void store.ipc.hasMediaFoundation().then((mf) => setSteps([...(mf ? [] : ["media" as const]), "mic", "hotkey", "folder", "test"]));
    void store.ipc.listDevices().then(setDevices).catch(() => setDevices({ monitors: [], mics: [] }));
  }, [store]);

  const step = steps?.[index];

  // Pegel nur, solange der Mikrofon-Schritt offen ist.
  useEffect(() => {
    if (step !== "mic") return;
    void store.ipc.subscribeMicLevel(true);
    return () => {
      void store.ipc.subscribeMicLevel(false);
    };
  }, [step, store]);

  // Test-Clip: auf die Meldung des Dienstes warten.
  useEffect(() => {
    if (step !== "test") return;
    return store.ipc.onEvent((ev) => {
      if (ev.event === "SaveCompleted") setSaved(true);
    });
  }, [step, store]);

  if (!steps || !settings || !step) return null;
  const next = () => setIndex((i) => Math.min(i + 1, steps.length - 1));

  async function patch(p: Record<string, unknown>) {
    const r = await store.ipc.setSettings(p as never);
    store.setSettings(r.settings);
  }

  async function finish() {
    setBusy(true);
    try {
      await patch({ first_run_done: true });
      // Autostart erst nach dem ersten Start einschalten; Windows kann nachfragen.
      await store.ipc.setAutostart(true).catch(() => undefined);
      props.onDone();
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={css.page}>
      <ol className={css.progress} aria-label="Schritte">
        {steps.map((s, i) => (
          <li key={s} data-active={i === index || undefined} data-done={i < index || undefined}>
            {{ media: "Wiedergabe", mic: "Mikrofon", hotkey: "Hotkey", folder: "Ordner", test: "Test-Clip" }[s]}
          </li>
        ))}
      </ol>

      <section className={css.card}>
        {step === "media" && (
          <>
            <h1>Media Feature Pack fehlt</h1>
            <p>
              Diese Windows-Edition (N oder KN) enthält keine Videowiedergabe. GameClip nimmt trotzdem auf, aber die Galerie kann Clips nicht abspielen.
            </p>
            <p>
              Installieren: Einstellungen → System → Optionale Features → „Feature hinzufügen“ → „Media Feature Pack“. Danach Windows neu starten.
            </p>
            <div className={css.buttons}>
              <button className="primary" onClick={next}>
                Weiter
              </button>
            </div>
          </>
        )}

        {step === "mic" && (
          <>
            <h1>Mikrofon</h1>
            <label className={css.check}>
              <input
                type="checkbox"
                checked={settings.audio.mic_enabled}
                onChange={(e) => void patch({ audio: { mic_enabled: e.target.checked } })}
              />
              Mikrofon aufnehmen
            </label>
            <p className={css.hint}>
              Das Mikrofon wird dauerhaft im Hintergrund aufgenommen, solange GameClip läuft. Gespeichert wird nur, was im Clip landet.
            </p>
            <select
              value={settings.audio.mic_device ?? ""}
              disabled={!settings.audio.mic_enabled}
              onChange={(e) => void patch({ audio: { mic_device: e.target.value || null } })}
              aria-label="Mikrofon"
            >
              <option value="">Windows-Standard</option>
              {devices?.mics.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name}
                </option>
              ))}
            </select>
            {settings.audio.mic_enabled && <MicMeter />}
            <div className={css.buttons}>
              <button className="primary" onClick={next}>
                Weiter
              </button>
            </div>
          </>
        )}

        {step === "hotkey" && (
          <>
            <h1>Hotkey</h1>
            <p>
              Mit dieser Taste speicherst du die letzten {settings.capture.buffer_seconds} Sekunden. Drücke sie einmal zum Test. Standard ist{" "}
              {hotkeyLabel(settings.hotkey)}.
            </p>
            <HotkeyPicker inline value={settings.hotkey} onDone={next} />
          </>
        )}

        {step === "folder" && (
          <>
            <h1>Clip-Ordner</h1>
            <p className={css.path}>{settings.storage.clip_folder ?? "Videos\\GameClip (Standard)"}</p>
            <p className={css.hint}>
              Ist das Limit von {Math.round(settings.storage.clip_limit_bytes / 1e9)} GB erreicht, löscht GameClip die ältesten eigenen Clips ohne Stern.
            </p>
            <div className={css.buttons}>
              <button
                onClick={() =>
                  void store.ipc.pickFolder(settings.storage.clip_folder).then((dir) => {
                    if (dir) void patch({ storage: { clip_folder: dir } }).catch((e) => store.notify(String(e.message ?? e), "error"));
                  })
                }
              >
                Anderen Ordner wählen …
              </button>
              <button className="primary" onClick={next}>
                Weiter
              </button>
            </div>
          </>
        )}

        {step === "test" && (
          <>
            <h1>Test-Clip</h1>
            <p>Erstelle jetzt einen Clip, um zu prüfen, dass alles funktioniert.</p>
            {saved && <p className={css.ok}>Clip gespeichert. Du findest ihn in der Galerie.</p>}
            <div className={css.buttons}>
              <button
                onClick={() => void store.ipc.saveClip().catch((e) => store.notify(String(e.message ?? e), "error"))}
              >
                Jetzt Test-Clip erstellen
              </button>
              <button className="primary" disabled={busy} onClick={() => void finish()}>
                Fertig
              </button>
            </div>
          </>
        )}
      </section>
    </div>
  );
}

function MicMeter() {
  const level = useSelect((s) => s.micLevel);
  // −60 dBFS = leer, 0 dBFS = voll.
  const fill = level === null ? 0 : Math.max(0, Math.min(1, (level + 60) / 60));
  return (
    <div className={css.meter} role="meter" aria-label="Mikrofonpegel" aria-valuenow={Math.round(fill * 100)}>
      <span style={{ width: `${fill * 100}%` }} />
    </div>
  );
}
