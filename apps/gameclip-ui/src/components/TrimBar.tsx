// SPDX-License-Identifier: GPL-3.0-or-later
// Kürzen-Leiste (Design, Datei 65): Griffe für erstes und letztes Bild, rasten auf Bilder ein.
// Die Wiedergabeposition läuft über requestVideoFrameCallback direkt ins DOM; React rendert nur,
// wenn ein Griff bewegt wird.

import { type RefObject, useEffect, useRef } from "react";
import { duration as fmtDuration } from "../format";
import { fractionOf, inUs, indexAtFraction, outUs } from "../timeline";
import css from "./TrimBar.module.css";

type VideoWithFrames = HTMLVideoElement & {
  requestVideoFrameCallback?: (cb: (now: number, meta: { mediaTime: number }) => void) => number;
  cancelVideoFrameCallback?: (id: number) => void;
};

export function TrimBar(props: {
  framesUs: readonly number[];
  durationUs: number;
  /** Erstes Bild des Schnitts. */
  first: number;
  /** Letztes Bild des Schnitts (einschließlich). */
  last: number;
  onChange: (first: number, last: number) => void;
  onSeek: (index: number) => void;
  video: RefObject<HTMLVideoElement | null>;
}) {
  const bar = useRef<HTMLDivElement>(null);
  const head = useRef<HTMLDivElement>(null);
  const { framesUs, durationUs, first, last } = props;
  const lastIndex = Math.max(0, framesUs.length - 1);

  // Wiedergabeposition ohne React.
  useEffect(() => {
    const v = props.video.current as VideoWithFrames | null;
    if (!v) return;
    const place = (seconds: number) => {
      if (head.current) head.current.style.left = `${fractionOf(seconds * 1e6, durationUs) * 100}%`;
    };
    if (v.requestVideoFrameCallback) {
      let id = 0;
      const tick = (_: number, meta: { mediaTime: number }) => {
        place(meta.mediaTime);
        id = v.requestVideoFrameCallback!(tick);
      };
      id = v.requestVideoFrameCallback(tick);
      const onSeeked = () => place(v.currentTime);
      v.addEventListener("seeked", onSeeked);
      return () => {
        v.cancelVideoFrameCallback?.(id);
        v.removeEventListener("seeked", onSeeked);
      };
    }
    const onTime = () => place(v.currentTime);
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("seeked", onTime);
    return () => {
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("seeked", onTime);
    };
  }, [props.video, durationUs]);

  const indexAt = (clientX: number) => {
    const r = bar.current?.getBoundingClientRect();
    if (!r || r.width === 0) return 0;
    return indexAtFraction((clientX - r.left) / r.width, framesUs, durationUs);
  };

  const set = (which: "first" | "last", index: number) => {
    const i = Math.max(0, Math.min(lastIndex, index));
    if (which === "first") props.onChange(Math.min(i, last), last);
    else props.onChange(first, Math.max(i, first));
  };

  const drag = (which: "first" | "last") => (e: React.PointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    const el = e.currentTarget;
    el.setPointerCapture?.(e.pointerId);
    const move = (ev: PointerEvent) => {
      const i = indexAt(ev.clientX);
      set(which, i);
      props.onSeek(which === "first" ? Math.min(i, last) : Math.max(i, first));
    };
    const up = () => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  };

  const keys = (which: "first" | "last") => (e: React.KeyboardEvent) => {
    const step = e.shiftKey ? 10 : 1;
    const cur = which === "first" ? first : last;
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      e.stopPropagation();
      const next = cur + (e.key === "ArrowRight" ? step : -step);
      set(which, next);
      props.onSeek(Math.max(0, Math.min(lastIndex, next)));
    }
  };

  const start = inUs(first, framesUs);
  const end = outUs(last, framesUs, durationUs);
  const left = fractionOf(start, durationUs) * 100;
  const right = fractionOf(end, durationUs) * 100;

  return (
    <div className={css.wrap}>
      <div className={css.bar} ref={bar} onPointerDown={(e) => props.onSeek(indexAt(e.clientX))} data-testid="trim-bar">
        <div className={css.selection} style={{ left: `${left}%`, width: `${right - left}%` }} />
        <div className={css.head} ref={head} aria-hidden />
        <div
          className={css.handle}
          style={{ left: `${left}%` }}
          role="slider"
          tabIndex={0}
          aria-label="Start"
          aria-valuemin={0}
          aria-valuemax={lastIndex}
          aria-valuenow={first}
          onPointerDown={drag("first")}
          onKeyDown={keys("first")}
        />
        <div
          className={css.handle}
          style={{ left: `${right}%` }}
          role="slider"
          tabIndex={0}
          aria-label="Ende"
          aria-valuemin={0}
          aria-valuemax={lastIndex}
          aria-valuenow={last}
          onPointerDown={drag("last")}
          onKeyDown={keys("last")}
        />
      </div>
      <div className={css.labels}>
        <span>Start {fmtDuration(start)} (Bild {first + 1})</span>
        <span className={css.length}>Länge {((end - start) / 1e6).toLocaleString("de-DE", { maximumFractionDigits: 2 })} s</span>
        <span>Ende {fmtDuration(end)} (Bild {last + 1})</span>
      </div>
    </div>
  );
}
