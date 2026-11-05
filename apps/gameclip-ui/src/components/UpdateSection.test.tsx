// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderWithStore } from "../test/render";
import { UpdateSection } from "./UpdateSection";

describe("UpdateSection", () => {
  it("fragt erst auf Klick und bietet dann die neue Version an", async () => {
    const { ipc } = await renderWithStore(<UpdateSection />);
    expect(screen.getByRole("status").textContent).toMatch(/nur, wenn du hier klickst/);
    fireEvent.click(screen.getByRole("button", { name: "Nach Updates suchen" }));
    expect(await screen.findByText(/Version 0\.2\.0\.0 ist verfügbar .*Installiert: 0\.1\.0\.6\./)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Herunterladen und installieren" }));
    expect(await screen.findByText(/App Installer von Windows ist offen/)).toBeTruthy();
    expect(ipc.state.updateInstalled).toBe(true);
  });

  it("meldet „aktuell“", async () => {
    const { ipc } = await renderWithStore(<UpdateSection />);
    ipc.state.update = { current: "0.2.0.0", latest: "0.2.0.0", newer: false, page_url: null, size_bytes: null };
    fireEvent.click(screen.getByRole("button", { name: "Nach Updates suchen" }));
    expect(await screen.findByText("GameClip ist aktuell (0.2.0.0).")).toBeTruthy();
  });

  it("zeigt Fehler", async () => {
    const { ipc } = await renderWithStore(<UpdateSection />);
    ipc.state.update = new Error("GitHub nicht erreichbar: Zeitüberschreitung");
    fireEvent.click(screen.getByRole("button", { name: "Nach Updates suchen" }));
    expect(await screen.findByText(/Fehler: GitHub nicht erreichbar/)).toBeTruthy();
  });
});
