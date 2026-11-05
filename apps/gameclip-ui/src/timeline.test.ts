// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { fractionOf, inUs, indexAtFraction, outUs, seekTimeForFrame, toFrameIndex } from "./timeline";

// 60 fps: Bild i beginnt bei round(i × 16 666,67 µs); 10 Bilder, Clip 166 667 µs lang.
const frames = Array.from({ length: 10 }, (_, i) => Math.round((i * 1_000_000) / 60));
const duration = Math.round((10 * 1_000_000) / 60);

describe("timeline", () => {
  it("bildet currentTime auf das angezeigte Bild ab", () => {
    expect(toFrameIndex(0, frames)).toBe(0);
    expect(toFrameIndex(frames[3] / 1e6, frames)).toBe(3);
    // Mitten im Bild.
    expect(toFrameIndex((frames[3] + 8000) / 1e6, frames)).toBe(3);
    // Gleitkomma knapp vor dem Bildbeginn zählt noch zum Bild.
    expect(toFrameIndex(frames[4] / 1e6 - 1e-7, frames)).toBe(4);
    // Kurz vor dem nächsten Bild.
    expect(toFrameIndex((frames[5] - 2000) / 1e6, frames)).toBe(4);
  });

  it("Rand am Ende: letztes Bild bis zur Dauer und darüber hinaus", () => {
    expect(toFrameIndex(frames[9] / 1e6, frames)).toBe(9);
    expect(toFrameIndex(duration / 1e6, frames)).toBe(9);
    expect(toFrameIndex(10, frames)).toBe(9);
    expect(toFrameIndex(-1, frames)).toBe(0);
    expect(toFrameIndex(1, [])).toBe(0);
  });

  it("Spulen auf ein Bild landet in diesem Bild", () => {
    for (let i = 0; i < frames.length; i++) {
      expect(toFrameIndex(seekTimeForFrame(i, frames, duration), frames)).toBe(i);
    }
  });

  it("Schnittgrenzen: Start auf dem Bild, Ende auf dem Bild danach (ausschließlich)", () => {
    expect(inUs(2, frames)).toBe(frames[2]);
    expect(outUs(5, frames, duration)).toBe(frames[6]);
    // Letztes gewünschtes Bild ist das letzte des Clips: Ende = Dauer.
    expect(outUs(9, frames, duration)).toBe(duration);
  });

  it("Leistenposition rastet auf Bilder ein", () => {
    expect(indexAtFraction(0, frames, duration)).toBe(0);
    expect(indexAtFraction(1, frames, duration)).toBe(9);
    expect(indexAtFraction(0.5, frames, duration)).toBe(5);
    expect(fractionOf(duration / 2, duration)).toBeCloseTo(0.5);
    expect(fractionOf(5, 0)).toBe(0);
  });
});
