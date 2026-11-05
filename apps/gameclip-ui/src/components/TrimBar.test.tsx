// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { TrimBar } from "./TrimBar";

const frames = Array.from({ length: 60 }, (_, i) => Math.round((i * 1_000_000) / 60));
const duration = 1_000_000;

function setup(first = 0, last = 59) {
  const onChange = vi.fn();
  const onSeek = vi.fn();
  const video = createRef<HTMLVideoElement>();
  render(
    <>
      <video ref={video} />
      <TrimBar framesUs={frames} durationUs={duration} first={first} last={last} onChange={onChange} onSeek={onSeek} video={video} />
    </>,
  );
  const bar = screen.getByTestId("trim-bar");
  // jsdom hat kein Layout: Leiste 600 px breit ab x = 100.
  bar.getBoundingClientRect = () => ({ left: 100, width: 600, top: 0, height: 36, right: 700, bottom: 36, x: 100, y: 0, toJSON: () => ({}) });
  return { onChange, onSeek, bar };
}

describe("TrimBar", () => {
  it("rastet beim Ziehen des Start-Griffs auf ein Bild ein", () => {
    const { onChange, onSeek } = setup();
    const start = screen.getByRole("slider", { name: "Start" });
    fireEvent.pointerDown(start, { clientX: 100, pointerId: 1 });
    // 40 % der Leiste = 400 000 µs = Bild 24 (24 × 16 667 = 400 000).
    fireEvent.pointerMove(start, { clientX: 340, pointerId: 1 });
    expect(onChange).toHaveBeenLastCalledWith(24, 59);
    expect(onSeek).toHaveBeenLastCalledWith(24);
    // Zwischen zwei Bildern: 41 % = 410 000 µs liegt in Bild 24 (Bild 25 beginnt bei 416 667).
    fireEvent.pointerMove(start, { clientX: 346, pointerId: 1 });
    expect(onChange).toHaveBeenLastCalledWith(24, 59);
    fireEvent.pointerUp(start, { pointerId: 1 });
  });

  it("Start kann nicht hinter das Ende und Ende nicht vor den Start", () => {
    const { onChange } = setup(10, 20);
    const start = screen.getByRole("slider", { name: "Start" });
    fireEvent.pointerDown(start, { clientX: 0, pointerId: 1 });
    fireEvent.pointerMove(start, { clientX: 650, pointerId: 1 });
    expect(onChange).toHaveBeenLastCalledWith(20, 20);
    const end = screen.getByRole("slider", { name: "Ende" });
    fireEvent.pointerDown(end, { clientX: 0, pointerId: 2 });
    fireEvent.pointerMove(end, { clientX: 110, pointerId: 2 });
    expect(onChange).toHaveBeenLastCalledWith(10, 10);
  });

  it("Pfeiltasten am Griff verschieben um ein Bild, mit Umschalt um zehn", () => {
    const { onChange } = setup(10, 50);
    const end = screen.getByRole("slider", { name: "Ende" });
    fireEvent.keyDown(end, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenLastCalledWith(10, 49);
    fireEvent.keyDown(end, { key: "ArrowRight", shiftKey: true });
    // Über das letzte Bild hinaus bleibt es beim letzten.
    expect(onChange).toHaveBeenLastCalledWith(10, 59);
  });

  it("zeigt Start, Ende (ausschließlich) und Länge", () => {
    setup(0, 59);
    expect(screen.getByText("Länge 1 s")).toBeTruthy();
    expect(screen.getByText(/Ende 0:01 \(Bild 60\)/)).toBeTruthy();
  });
});
