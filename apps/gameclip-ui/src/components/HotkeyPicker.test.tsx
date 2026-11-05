// SPDX-License-Identifier: GPL-3.0-or-later
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithStore } from "../test/render";
import { HotkeyPicker } from "./HotkeyPicker";

const F8 = { code: "F8", ctrl: false, alt: false, shift: false };

describe("HotkeyPicker", () => {
  it("lehnt F12 ab, ohne den Dienst zu fragen", async () => {
    const { calls } = await renderWithStore(<HotkeyPicker value={F8} onDone={() => {}} />);
    fireEvent.keyDown(screen.getByTestId("hotkey-capture"), { code: "F12", key: "F12" });
    expect(await screen.findByText("F12 ist für Debugger reserviert")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Übernehmen" })).toHaveProperty("disabled", true);
    expect(calls.some((c) => c.type === "TestRegister")).toBe(false);
  });

  it("verlangt Strg oder Alt bei Buchstaben", async () => {
    await renderWithStore(<HotkeyPicker value={F8} onDone={() => {}} />);
    fireEvent.keyDown(screen.getByTestId("hotkey-capture"), { code: "KeyK", key: "k" });
    expect(await screen.findByText(/nur zusammen mit Strg oder Alt/)).toBeTruthy();
  });

  it("meldet eine belegte Taste und speichert eine freie", async () => {
    const onDone = vi.fn();
    const { calls, ipc } = await renderWithStore(<HotkeyPicker value={F8} onDone={onDone} />);
    const box = screen.getByTestId("hotkey-capture");
    fireEvent.keyDown(box, { code: "F9", key: "F9" });
    expect(await screen.findByText(/von einem anderen Programm belegt/)).toBeTruthy();
    fireEvent.keyDown(box, { code: "KeyK", key: "k", ctrlKey: true, altKey: true });
    expect(await screen.findByText("Strg+Alt+K ist frei.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Übernehmen" }));
    await waitFor(() => expect(onDone).toHaveBeenCalled());
    expect(ipc.state.settings.hotkey).toEqual({ code: "KeyK", ctrl: true, alt: true, shift: false });
    expect(calls.find((c) => c.type === "SetSettings")).toBeTruthy();
  });

  it("gibt die Taste frei, solange offen, und hebt das beim Schließen auf", async () => {
    const { calls, unmount } = await renderWithStore(<HotkeyPicker value={F8} onDone={() => {}} />);
    expect(calls).toContainEqual({ type: "SuspendHotkey", body: { on: true } });
    act(() => unmount());
    expect(calls.at(-1)).toEqual({ type: "SuspendHotkey", body: { on: false } });
  });

  it("bietet F13 bis F24 an", async () => {
    await renderWithStore(<HotkeyPicker value={F8} onDone={() => {}} />);
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "F13" } });
    expect(await screen.findByText("F13 ist frei.")).toBeTruthy();
  });
});
