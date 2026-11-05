// SPDX-License-Identifier: GPL-3.0-or-later
// Kachel in der Galerie: Vorschaubild, Name, Datum, Dauer, Größe, Stern, Markierung „Fremd“.

import type { ClipEntry } from "../generated/ClipEntry";
import { bytes, dateTime, duration, fileName } from "../format";
import css from "./ClipCard.module.css";

export function ClipCard(props: {
  clip: ClipEntry;
  thumbnail: string;
  onStar: (on: boolean) => void;
  onAdopt: () => void;
  onOpen: () => void;
  onMenu: (x: number, y: number) => void;
}) {
  const c = props.clip;
  const foreign = c.status === "foreign";
  return (
    <article
      className={css.card}
      data-old={!c.in_current_folder || undefined}
      data-foreign={foreign || undefined}
      onContextMenu={(e) => {
        e.preventDefault();
        props.onMenu(e.clientX, e.clientY);
      }}
      onDoubleClick={props.onOpen}
      aria-label={fileName(c.path)}
    >
      <div className={css.thumb}>
        <img src={props.thumbnail} alt="" loading="lazy" draggable={false} />
        <span className={css.duration}>{duration(c.duration_us)}</span>
        {foreign && <span className={css.foreign}>Fremd</span>}
      </div>
      <div className={css.meta}>
        <div className={css.title} title={c.path}>
          {fileName(c.path)}
        </div>
        <div className={css.sub}>
          {dateTime(c.created_at)} · {bytes(c.size_bytes)}
          {!c.in_current_folder && " · früherer Ordner"}
        </div>
      </div>
      <div className={css.actions}>
        {foreign ? (
          <button
            onClick={props.onAdopt}
            aria-label="Als eigenen Clip übernehmen"
            title="Nicht von GameClip markiert (z. B. kopiert). Übernehmen schreibt eine eigene Kennung in die Datei; danach zählt sie zum Limit."
          >
            Übernehmen
          </button>
        ) : (
          <button
            className={css.star}
            data-on={c.starred || undefined}
            aria-pressed={c.starred}
            aria-label={c.starred ? "Stern entfernen" : "Mit Stern schützen"}
            title={c.starred ? "Geschützt: wird nicht automatisch gelöscht" : "Mit Stern vor dem automatischen Löschen schützen"}
            onClick={() => props.onStar(!c.starred)}
          >
            {c.starred ? "★" : "☆"}
          </button>
        )}
        <button aria-label="Mehr" onClick={(e) => props.onMenu(e.clientX, e.clientY)}>
          ⋯
        </button>
      </div>
    </article>
  );
}
