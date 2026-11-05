// SPDX-License-Identifier: GPL-3.0-or-later
// Editor (Design, Datei 64): Player, Kürzen-Leiste, Bildschritt mit Pfeiltasten, Export-Dialog.
// Start und Ende rasten auf die Bildzeiten aus `GetFrameTimes` ein.
//
// Tasten: Leertaste Wiedergabe, ←/→ ein Bild (mit Umschalt zehn), I/O Start/Ende hier,
// Pos1/Ende an den Anfang/das Ende.

import { useEffect, useRef, useState } from "react";
import type { ClipEntry } from "../generated/ClipEntry";
import { ExportDialog } from "../components/ExportDialog";
import { TrimBar } from "../components/TrimBar";
import { fileName } from "../format";
import { inUs, outUs, seekTimeForFrame, toFrameIndex } from "../timeline";
import { useStore } from "../store";
import css from "./Editor.module.css";

type Frames = { framesUs: number[]; durationUs: number };

export function Editor(props: { clip: ClipEntry; onBack: () => void }) {
  const store = useStore();
  const video = useRef<HTMLVideoElement>(null);
  const frameLabel = useRef<HTMLSpanElement>(null);
  const [frames, setFrames] = useState<Frames | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [first, setFirst] = useState(0);
  const [last, setLast] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [exporting, setExporting] = useState(false);
  const { clip } = props;

  useEffect(() => {
    let alive = true;
    void store.ipc
      .getFrameTimes(clip.clip_id)
      .then((r) => {
        if (!alive) return;
        setFrames({ framesUs: r.frames_us, durationUs: r.duration_us });
        setFirst(0);
        setLast(Math.max(0, r.frames_us.length - 1));
      })
      .catch((e) => alive && setError(String((e as Error).message ?? e)));
    return () => {
      alive = false;
    };
  }, [clip.clip_id, store]);

  // Aktuelle Bildnummer ohne React-Rendern.
  useEffect(() => {
    const v = video.current;
    if (!v || !frames) return;
    const show = () => {
      if (frameLabel.current) frameLabel.current.textContent = `Bild ${toFrameIndex(v.currentTime, frames.framesUs) + 1} / ${frames.framesUs.length}`;
    };
    show();
    v.addEventListener("timeupdate", show);
    v.addEventListener("seeked", show);
    return () => {
      v.removeEventListener("timeupdate", show);
      v.removeEventListener("seeked", show);
    };
  }, [frames]);

  const current = () => (video.current && frames ? toFrameIndex(video.current.currentTime, frames.framesUs) : 0);

  const seek = (index: number) => {
    const v = video.current;
    if (!v || !frames) return;
    v.currentTime = seekTimeForFrame(index, frames.framesUs, frames.durationUs);
  };

  const togglePlay = () => {
    const v = video.current;
    if (!v) return;
    if (v.paused) void v.play().catch(() => undefined);
    else v.pause();
  };

  useEffect(() => {
    if (!frames || exporting) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === "INPUT" || target.getAttribute("role") === "slider")) return;
      const n = frames.framesUs.length;
      switch (e.key) {
        case " ":
          e.preventDefault();
          togglePlay();
          break;
        case "ArrowLeft":
        case "ArrowRight": {
          e.preventDefault();
          video.current?.pause();
          const step = (e.shiftKey ? 10 : 1) * (e.key === "ArrowRight" ? 1 : -1);
          seek(Math.max(0, Math.min(n - 1, current() + step)));
          break;
        }
        case "Home":
          seek(0);
          break;
        case "End":
          seek(n - 1);
          break;
        case "i":
        case "I":
          setFirst(Math.min(current(), last));
          break;
        case "o":
        case "O":
          setLast(Math.max(current(), first));
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [frames, first, last, exporting]);

  return (
    <section className={css.editor}>
      <header className={css.header}>
        <button onClick={props.onBack}>← Galerie</button>
        <h2 title={clip.path}>{fileName(clip.path)}</h2>
        <span className={css.frame} ref={frameLabel} />
      </header>

      <div className={css.stage}>
        <video
          ref={video}
          className={css.video}
          src={store.ipc.clipUrl(clip.clip_id) || undefined}
          preload="auto"
          onPlay={() => setPlaying(true)}
          onPause={() => setPlaying(false)}
          onClick={togglePlay}
          data-testid="player"
        />
      </div>

      {error && <p className={css.error}>Bildzeiten nicht lesbar: {error}</p>}

      {frames && (
        <div className={css.controls}>
          <TrimBar
            framesUs={frames.framesUs}
            durationUs={frames.durationUs}
            first={first}
            last={last}
            onChange={(f, l) => {
              setFirst(f);
              setLast(l);
            }}
            onSeek={seek}
            video={video}
          />
          <div className={css.buttons}>
            <button onClick={togglePlay} aria-label={playing ? "Pause" : "Abspielen"}>
              {playing ? "⏸" : "▶"}
            </button>
            <button onClick={() => setFirst(Math.min(current(), last))} title="Taste I">
              Start hier
            </button>
            <button onClick={() => setLast(Math.max(current(), first))} title="Taste O">
              Ende hier
            </button>
            <span className={css.spacer} />
            <button className="primary" onClick={() => setExporting(true)}>
              Exportieren …
            </button>
          </div>
        </div>
      )}

      {exporting && frames && (
        <ExportDialog
          clip={clip}
          inUs={inUs(first, frames.framesUs)}
          outUs={outUs(last, frames.framesUs, frames.durationUs)}
          onClose={() => setExporting(false)}
        />
      )}
    </section>
  );
}
