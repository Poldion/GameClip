// SPDX-License-Identifier: GPL-3.0-or-later
// Statuszeile: „Puffert 1:00 in 1080p60 über NVENC“, Einschränkungen und „Clip speichern“.

import type { ServiceStatus } from "../generated/ServiceStatus";
import type { Settings } from "../generated/Settings";
import { duration } from "../format";
import { useSelect, useStore } from "../store";
import css from "./StatusBar.module.css";

export function encoderName(encoder: string | null): string {
  if (!encoder) return "–";
  if (encoder.includes("nvenc")) return "NVENC";
  if (encoder.includes("amf")) return "AMF";
  if (encoder.includes("qsv")) return "Quick Sync";
  if (encoder.includes("x264")) return "CPU";
  return encoder;
}

const RESTRICTION: Record<string, string> = {
  hotkey_inactive: "Hotkey nicht aktiv",
  cpu_encoder: "Aufnahme über CPU",
  hdr_active: "HDR wird als SDR aufgenommen",
};

export function statusText(s: ServiceStatus, settings: Settings | null): string {
  switch (s.state.kind) {
    case "starting":
      return "Startet …";
    case "paused":
      return "Pausiert";
    case "reconfiguring":
      return "Konfiguriert neu …";
    case "recovering":
      return `Erholt sich (Versuch ${s.state.attempt}) …`;
    case "failed":
      return "Fehler: Aufnahme läuft nicht";
    case "buffering": {
      const fps = settings?.capture.fps ?? 60;
      const res = s.capture_resolution ? `${s.capture_resolution[1]}p${fps}` : settings?.capture.resolution;
      return `Puffert ${duration(s.buffered_us)} in ${res} über ${encoderName(s.encoder)}`;
    }
  }
}

export function StatusBar() {
  const store = useStore();
  const status = useSelect((s) => s.status);
  const settings = useSelect((s) => s.settings);
  if (!status) return <div className={css.bar}>Verbinde …</div>;
  const failed = status.state.kind === "failed";
  return (
    <div className={css.bar} data-state={status.state.kind}>
      <span className={css.dot} aria-hidden />
      <span className={css.text}>{statusText(status, settings)}</span>
      {status.restrictions.map((r) => (
        <span key={r} className={css.badge}>
          Eingeschränkt: {RESTRICTION[r] ?? r}
        </span>
      ))}
      {status.mic_paused && <span className={css.badge}>Mikrofon pausiert</span>}
      <span className={css.spacer} />
      {failed ? (
        <button onClick={() => void store.ipc.restart()}>Neu starten</button>
      ) : (
        <button
          className="primary"
          disabled={status.state.kind !== "buffering"}
          onClick={() => void store.ipc.saveClip().catch((e) => store.notify(String(e.message ?? e), "error"))}
        >
          {status.saves_in_flight > 0 ? "Speichert …" : "Clip speichern"}
        </button>
      )}
    </div>
  );
}
