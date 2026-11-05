// SPDX-License-Identifier: GPL-3.0-or-later
// Ansichten ohne Router (Design, Datei 63): Erster Start, solange `first_run_done` falsch ist,
// sonst Galerie, Editor und Einstellungen. Vollbild-Meldungen, wenn der Dienst fehlt oder die
// Version nicht passt.

import { useEffect, useState } from "react";
import type { ClipEntry } from "./generated/ClipEntry";
import { StatusBar } from "./components/StatusBar";
import type { StartView } from "./ipc";
import { useSelect, useStore } from "./store";
import { Editor } from "./views/Editor";
import { FirstRun } from "./views/FirstRun";
import { Gallery } from "./views/Gallery";
import { Settings } from "./views/Settings";
import css from "./App.module.css";

type View = { name: "gallery" } | { name: "settings" } | { name: "editor"; clip: ClipEntry };

export function App() {
  const store = useStore();
  const connection = useSelect((s) => s.connection);
  const settings = useSelect((s) => s.settings);
  const notices = useSelect((s) => s.notices);
  const [view, setView] = useState<View>({ name: "gallery" });

  // Tray: „Galerie öffnen“ / „Einstellungen“ beim Start und bei offenem Fenster.
  useEffect(() => {
    const open = (v: StartView) => setView(v === "settings" ? { name: "settings" } : { name: "gallery" });
    void store.ipc.initialView().then((v) => v && open(v));
    return store.ipc.onOpenView(open);
  }, [store]);

  if (connection.state === "connecting") {
    return <Full title="Dienst wird gestartet …" />;
  }
  if (connection.state === "service_unreachable") {
    return (
      <Full title="GameClip-Dienst nicht erreichbar">
        <p>Der Hintergrunddienst startet nicht. Starte GameClip neu; hilft das nicht, installiere die App erneut.</p>
      </Full>
    );
  }
  if (connection.state === "version_mismatch") {
    return (
      <Full title="Versionen passen nicht">
        <p>Oberfläche und Dienst stammen aus verschiedenen Versionen ({connection.message}). Beende GameClip im Tray und starte es neu.</p>
      </Full>
    );
  }
  if (!settings) return <Full title="Lade …" />;

  return (
    <div className={css.app}>
      {settings.first_run_done ? (
        <>
          <nav className={css.nav}>
            <span className={css.brand}>GameClip</span>
            <button aria-current={view.name === "gallery" || view.name === "editor"} onClick={() => setView({ name: "gallery" })}>
              Galerie
            </button>
            <button aria-current={view.name === "settings"} onClick={() => setView({ name: "settings" })}>
              Einstellungen
            </button>
          </nav>
          <main className={css.main}>
            {view.name === "gallery" && <Gallery onOpen={(clip) => setView({ name: "editor", clip })} />}
            {view.name === "settings" && <Settings />}
            {view.name === "editor" && <Editor clip={view.clip} onBack={() => setView({ name: "gallery" })} />}
          </main>
        </>
      ) : (
        <main className={css.main}>
          <FirstRun onDone={() => setView({ name: "gallery" })} />
        </main>
      )}
      <StatusBar />
      <div className={css.notices} aria-live="polite">
        {notices.map((n) => (
          <div key={n.id} className={css.notice} data-kind={n.kind} onClick={() => store.dismiss(n.id)}>
            {n.text}
          </div>
        ))}
      </div>
    </div>
  );
}

function Full(props: { title: string; children?: React.ReactNode }) {
  return (
    <div className={css.full}>
      <h1>{props.title}</h1>
      {props.children}
    </div>
  );
}
