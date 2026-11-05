// SPDX-License-Identifier: GPL-3.0-or-later
// Galerie (Design, Datei 64): Reiter „Clips“ und „Exporte“, Kacheln nach Datum, Platzanzeige aus
// `used_bytes`/`limit_bytes` (nur eigene Clips im aktuellen Ordner, das rechnet der Dienst),
// Stern, „Fremd“ mit Übernehmen, Kontextmenü mit Kopieren, Im Ordner anzeigen, Löschen.

import { useEffect, useMemo, useState } from "react";
import type { ClipEntry } from "../generated/ClipEntry";
import type { FolderKind } from "../generated/FolderKind";
import { ClipCard } from "../components/ClipCard";
import { fileName, usage } from "../format";
import { useSelect, useStore } from "../store";
import css from "./Gallery.module.css";

type Menu = { clip: ClipEntry; x: number; y: number };

export function Gallery(props: { onOpen?: (clip: ClipEntry) => void }) {
  const store = useStore();
  const [tab, setTab] = useState<FolderKind>("clips");
  const [menu, setMenu] = useState<Menu | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<ClipEntry | null>(null);
  const list = useSelect((s) => (tab === "clips" ? s.clips : s.exports));

  // Ordner nur überwachen, solange die Galerie offen ist (Design, Datei 46).
  useEffect(() => {
    void store.ipc.watchFolders(true);
    return () => {
      void store.ipc.watchFolders(false);
    };
  }, [store]);

  useEffect(() => {
    void store.refreshFolder(tab);
  }, [store, tab]);

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("blur", close);
    };
  }, [menu]);

  const clips = useMemo(
    () => [...(list?.clips ?? [])].filter((c) => c.status !== "pending").sort((a, b) => b.created_at.localeCompare(a.created_at)),
    [list],
  );

  const act = (f: () => Promise<unknown>, after?: () => void) =>
    void f()
      .then(() => {
        after?.();
        return store.refreshFolder(tab);
      })
      .catch((e) => store.notify(String((e as Error).message ?? e), "error"));

  const full = list && list.limit_bytes > 0 ? Math.min(1, list.used_bytes / list.limit_bytes) : 0;

  return (
    <section className={css.gallery}>
      <header className={css.header}>
        <div className={css.tabs} role="tablist">
          {(["clips", "exports"] as const).map((t) => (
            <button key={t} role="tab" aria-selected={tab === t} className={css.tab} onClick={() => setTab(t)}>
              {t === "clips" ? "Clips" : "Exporte"}
            </button>
          ))}
        </div>
        {list && (
          <div className={css.usage} title="Belegt von GameClip-Clips im aktuellen Ordner">
            <span data-testid="usage">{usage(list.used_bytes, list.limit_bytes)}</span>
            <span className={css.meter}>
              <span style={{ width: `${full * 100}%` }} data-full={full >= 1 || undefined} />
            </span>
          </div>
        )}
      </header>

      {clips.length === 0 ? (
        <div className={css.empty}>
          {tab === "clips"
            ? `Noch keine Clips. Drücke den Hotkey im Spiel oder „Clip speichern“.`
            : "Noch keine Exporte. Öffne einen Clip, um ihn zu kürzen und zu exportieren."}
        </div>
      ) : (
        <div className={css.grid}>
          {clips.map((c) => (
            <ClipCard
              key={c.clip_id}
              clip={c}
              thumbnail={store.ipc.thumbnailUrl(c.clip_id)}
              onStar={(on) => act(() => store.ipc.setStar(c.clip_id, on))}
              onAdopt={() => act(() => store.ipc.adoptClip(c.clip_id))}
              onOpen={() => c.status === "ok" && props.onOpen?.(c)}
              onMenu={(x, y) => setTimeout(() => setMenu({ clip: c, x, y }))}
            />
          ))}
        </div>
      )}

      {menu && (
        <ul className={css.menu} style={{ left: menu.x, top: menu.y }} role="menu">
          <li role="menuitem" onClick={() => act(() => store.ipc.copyFile(menu.clip.clip_id), () => store.notify("Datei kopiert. Mit Strg+V z. B. in Discord einfügen."))}>
            Kopieren
          </li>
          <li role="menuitem" onClick={() => act(() => store.ipc.reveal(menu.clip.clip_id))}>
            Im Ordner anzeigen
          </li>
          <li role="menuitem" className={css.danger} onClick={() => setConfirmDelete(menu.clip)}>
            Löschen …
          </li>
        </ul>
      )}

      {confirmDelete && (
        <div className={css.backdrop}>
          <div className={css.confirm} role="alertdialog" aria-label="Löschen bestätigen">
            <h3>„{fileName(confirmDelete.path)}“ löschen?</h3>
            <p>Die Datei wird endgültig gelöscht, nicht in den Papierkorb verschoben.</p>
            <div className={css.buttons}>
              <button onClick={() => setConfirmDelete(null)}>Abbrechen</button>
              <button
                className="primary"
                onClick={() => {
                  const c = confirmDelete;
                  setConfirmDelete(null);
                  act(() => store.ipc.deleteClip(c.clip_id));
                }}
              >
                Endgültig löschen
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
