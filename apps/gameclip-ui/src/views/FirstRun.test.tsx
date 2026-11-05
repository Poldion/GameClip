// SPDX-License-Identifier: GPL-3.0-or-later
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithStore } from "../test/render";
import { FirstRun } from "./FirstRun";

describe("FirstRun", () => {
  it("durchläuft Mikrofon, Hotkey, Ordner und Test-Clip", async () => {
    const onDone = vi.fn();
    const { ipc, calls } = await renderWithStore(<FirstRun onDone={onDone} />);

    // Mikrofon: standardmäßig an, mit Hinweis und Pegel.
    expect(await screen.findByRole("heading", { name: "Mikrofon" })).toBeTruthy();
    expect(screen.getByText(/dauerhaft im Hintergrund aufgenommen/)).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: "Mikrofon aufnehmen" })).toHaveProperty("checked", true);
    expect(calls).toContainEqual({ type: "SubscribeMicLevel", body: { on: true } });
    fireEvent.change(screen.getByRole("combobox", { name: "Mikrofon" }), { target: { value: "{mic-2}" } });
    await waitFor(() => expect(ipc.state.settings.audio.mic_device).toBe("{mic-2}"));
    fireEvent.click(screen.getByRole("button", { name: "Weiter" }));

    // Hotkey: Testdruck mit der Standardtaste.
    expect(await screen.findByRole("heading", { name: "Hotkey" })).toBeTruthy();
    expect(calls).toContainEqual({ type: "SubscribeMicLevel", body: { on: false } });
    fireEvent.keyDown(screen.getByTestId("hotkey-capture"), { code: "F8", key: "F8" });
    expect(await screen.findByText("F8 ist frei.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Übernehmen" }));

    // Ordner.
    expect(await screen.findByRole("heading", { name: "Clip-Ordner" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Anderen Ordner wählen …" }));
    await waitFor(() => expect(ipc.state.settings.storage.clip_folder).toBe("C:\\Users\\Spieler\\Videos\\Neu"));
    fireEvent.click(screen.getByRole("button", { name: "Weiter" }));

    // Test-Clip.
    expect(await screen.findByRole("heading", { name: "Test-Clip" })).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Jetzt Test-Clip erstellen" }));
    });
    expect(await screen.findByText(/Clip gespeichert/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Fertig" }));
    await waitFor(() => expect(onDone).toHaveBeenCalled());
    expect(ipc.state.settings.first_run_done).toBe(true);
  });

  it("zeigt den Hinweis zum Media Feature Pack zuerst", async () => {
    await renderWithStore(<FirstRun onDone={() => {}} />, { mediaFoundation: false });
    expect(await screen.findByRole("heading", { name: "Media Feature Pack fehlt" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Weiter" }));
    expect(await screen.findByRole("heading", { name: "Mikrofon" })).toBeTruthy();
  });
});
