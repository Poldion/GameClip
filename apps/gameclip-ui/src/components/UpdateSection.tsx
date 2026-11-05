// SPDX-License-Identifier: GPL-3.0-or-later
// „Updates“ in den Einstellungen. GameClip geht nie von selbst ins Internet (Design, Non-Goals):
// Erst der Klick auf „Nach Updates suchen“ fragt GitHub. „Herunterladen und installieren“ lädt
// das Paket nach Downloads\GameClip-Update und öffnet den App Installer von Windows, der GameClip
// dafür beendet.

import { useState } from "react";
import { bytes } from "../format";
import type { UpdateCheck } from "../ipc";
import { useStore } from "../store";

type Phase =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "checked"; result: UpdateCheck }
  | { kind: "downloading"; result: UpdateCheck }
  | { kind: "opened" }
  | { kind: "error"; message: string };

const message = (e: unknown) => String((e as Error)?.message ?? e);

function describe(phase: Phase): string {
  switch (phase.kind) {
    case "idle":
      return "GameClip sucht nur, wenn du hier klickst, und fragt dann die Releases auf GitHub ab.";
    case "checking":
      return "Suche …";
    case "checked":
    case "downloading": {
      const r = phase.result;
      if (r.newer && r.latest) {
        const size = r.size_bytes ? ` (${bytes(r.size_bytes)})` : "";
        return phase.kind === "downloading"
          ? `Lade Version ${r.latest}${size} …`
          : `Version ${r.latest} ist verfügbar${size}. Installiert: ${r.current}.`;
      }
      if (!r.current) return `Ohne installiertes Paket (Entwicklung) gibt es keine Updates. Neueste Version: ${r.latest ?? "keine"}.`;
      return `GameClip ist aktuell (${r.current}).`;
    }
    case "opened":
      return "Der App Installer von Windows ist offen. Dort „Aktualisieren“ wählen; GameClip wird dafür beendet.";
    case "error":
      return `Fehler: ${phase.message}`;
  }
}

export function UpdateSection(props: { className?: string; hintClass?: string }) {
  const store = useStore();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });

  const check = () => {
    setPhase({ kind: "checking" });
    store.ipc.checkUpdate().then(
      (result) => setPhase({ kind: "checked", result }),
      (e) => setPhase({ kind: "error", message: message(e) }),
    );
  };

  const install = (result: UpdateCheck) => {
    setPhase({ kind: "downloading", result });
    store.ipc.installUpdate().then(
      () => setPhase({ kind: "opened" }),
      (e) => setPhase({ kind: "error", message: message(e) }),
    );
  };

  const busy = phase.kind === "checking" || phase.kind === "downloading";
  const offer = phase.kind === "checked" && phase.result.newer ? phase.result : null;

  return (
    <fieldset className={props.className}>
      <h2>Updates</h2>
      <p className={props.hintClass} role="status">
        {describe(phase)}
      </p>
      <div>
        {offer ? (
          <button className="primary" onClick={() => install(offer)}>
            Herunterladen und installieren
          </button>
        ) : (
          <button onClick={check} disabled={busy}>
            Nach Updates suchen
          </button>
        )}
      </div>
    </fieldset>
  );
}
