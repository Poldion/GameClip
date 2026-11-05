// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { bytes, duration, fileName, usage } from "./format";
import { hotkeyError, hotkeyLabel } from "./hotkey";

describe("format", () => {
  it("Größen dezimal und deutsch", () => {
    expect(bytes(142_000_000)).toBe("142 MB");
    expect(bytes(7_200_000_000)).toBe("7,2 GB");
    expect(usage(7_200_000_000, 10_000_000_000)).toBe("7,2 / 10 GB");
  });

  it("Dauer", () => {
    expect(duration(60_000_000)).toBe("1:00");
    expect(duration(725_000_000)).toBe("12:05");
    expect(duration(3_723_000_000)).toBe("1:02:03");
  });

  it("Dateiname aus Windows-Pfad", () => {
    expect(fileName("C:\\a\\b\\clip.mp4")).toBe("clip.mp4");
  });
});

describe("hotkey", () => {
  it("folgt der Regel aus gameclip-core::keymap", () => {
    expect(hotkeyError({ code: "F8", ctrl: false, alt: false, shift: false })).toBeNull();
    expect(hotkeyError({ code: "F24", ctrl: false, alt: false, shift: false })).toBeNull();
    expect(hotkeyError({ code: "F12", ctrl: true, alt: false, shift: false })).toMatch(/F12/);
    expect(hotkeyError({ code: "KeyC", ctrl: false, alt: false, shift: true })).toMatch(/Strg oder Alt/);
    expect(hotkeyError({ code: "Digit5", ctrl: false, alt: true, shift: false })).toBeNull();
    expect(hotkeyError({ code: "Space", ctrl: true, alt: false, shift: false })).toMatch(/unbekannt/);
  });

  it("Anzeige", () => {
    expect(hotkeyLabel({ code: "KeyK", ctrl: true, alt: true, shift: false })).toBe("Strg+Alt+K");
  });
});
