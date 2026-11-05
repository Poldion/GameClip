// SPDX-License-Identifier: GPL-3.0-or-later
// Galerie (Design, Datei 64): Reiter „Clips“ und „Exporte“, Kacheln nach Spiel gruppiert (oder nach
// Datum, die Wahl merkt sich localStorage), Teilen über Windows, Platzanzeige aus
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
type SortMode = "game" | "date";
type Group = { key: string; label: string; clips: ClipEntry[] };

const SORT_KEY = "gallery.sort";

function loadSort(): SortMode {
  try {
    return localStorage.getItem(SORT_KEY) === "date" ? "date" : "game";
  } catch {
    return "game";
  }
}

function saveSort(m: SortMode) {
  try {
    localStorage.setItem(SORT_KEY, m);
  } catch {
    // Ohne Speicher gilt die Wahl nur bis zum Schließen.
  }
}

/** Gruppen nach Spiel; das Spiel mit dem neuesten Clip zuerst, „Ohne Spiel“ zuletzt. */
export function groupByGame(newestFirst: ClipEntry[]): Group[] {
  const groups = new Map<string, Group>();
  for (const c of newestFirst) {
    const key = c.program?.toLowerCase() ?? "";
    let g = groups.get(key);
    if (!g) {
      g = { key, label: c.program ?? "Ohne Spiel", clips: [] };
      groups.set(key, g);
    }
    g.clips.push(c);
  }
  const all = [...groups.values()];
  return [...all.filter((g) => g.key !== ""), ...all.filter((g) => g.key === "")];
}

export function Gallery(props: { onOpen?: (clip: ClipEntry) => void }) {
  const store = useStore();
  const [tab, setTab] = useState<FolderKind>("clips");
  const [menu, setMenu] = useState<Menu | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<ClipEntry | null>(null);
  const [sort, setSort] = useState<SortMode>(loadSort);
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

  const groups = useMemo<Group[]>(
    () => (sort === "game" ? groupByGame(clips) : [{ key: "*", label: "", clips }]),
    [clips, sort],
  );

  const share = (c: ClipEntry) =>
    void store.ipc.share(c.clip_id).catch((e) => store.notify(String((e as Error).message ?? e), "error"));

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
        <label className={css.sort}>
          Sortieren
          <select
            value={sort}
            onChange={(e) => {
              const m = e.target.value as SortMode;
              setSort(m);
              saveSort(m);
            }}
          >
            <option value="game">nach Spiel</option>
            <option value="date">nach Datum</option>
          </select>
        </label>
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
        <div className={css.groups}>
          {groups.map((g) => (
            <section key={g.key} aria-label={g.label || undefined}>
              {g.label && (
                <h2 className={css.groupTitle}>
                  {g.label} <span>{g.clips.length === 1 ? "1 Clip" : `${g.clips.length} Clips`}</span>
                </h2>
              )}
              <div className={css.grid}>
                {g.clips.map((c) => (
                  <ClipCard
                    key={c.clip_id}
                    clip={c}
                    thumbnail={store.ipc.thumbnailUrl(c.clip_id)}
                    onStar={(on) => act(() => store.ipc.setStar(c.clip_id, on))}
                    onAdopt={() => act(() => store.ipc.adoptClip(c.clip_id))}
                    onOpen={() => c.status === "ok" && props.onOpen?.(c)}
                    onMenu={(x, y) => setTimeout(() => setMenu({ clip: c, x, y }))}
                    onDrag={() => void store.ipc.startDrag(c.clip_id).catch((e) => store.notify(String((e as Error).message ?? e), "error"))}
                    onShare={() => share(c)}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}

      {menu && (
        <ul className={css.menu} style={{ left: menu.x, top: menu.y }} role="menu">
          <li role="menuitem" onClick={() => share(menu.clip)}>
            Teilen …
          </li>
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
