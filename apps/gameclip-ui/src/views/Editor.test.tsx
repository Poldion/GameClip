// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { sampleClips } from "../ipc.mock";
import { renderWithStore } from "../test/render";
import { Editor } from "./Editor";

const clip = sampleClips()[1];

describe("Editor", () => {
  it("setzt Start und Ende mit Pfeiltasten und I/O und exportiert genau diesen Bereich", async () => {
    const { calls } = await renderWithStore(<Editor clip={clip} onBack={() => {}} />);
    // 60 s × 60 fps = 3600 Bilder aus GetFrameTimes.
    expect(await screen.findByText(/Bild 3600\)/)).toBeTruthy();
    const player = screen.getByTestId("player") as HTMLVideoElement;
    player.pause = () => {};

    // 10 Bilder vor, Start hier.
    fireEvent.keyDown(window, { key: "ArrowRight", shiftKey: true });
    fireEvent.keyDown(window, { key: "i" });
    // Noch 20 Bilder weiter, Ende hier.
    fireEvent.keyDown(window, { key: "ArrowRight", shiftKey: true });
    fireEvent.keyDown(window, { key: "ArrowRight", shiftKey: true });
    fireEvent.keyDown(window, { key: "o" });
    expect(screen.getByRole("slider", { name: "Start" }).getAttribute("aria-valuenow")).toBe("10");
    expect(screen.getByRole("slider", { name: "Ende" }).getAttribute("aria-valuenow")).toBe("30");

    fireEvent.click(screen.getByRole("button", { name: "Exportieren …" }));
    fireEvent.click(screen.getByRole("button", { name: "Exportieren" }));
    await waitFor(() => expect(calls.some((c) => c.type === "StartExportJob")).toBe(true));
    // Bild 10 beginnt bei round(10 × 10^6 / 60); Ende ausschließlich = Beginn von Bild 31.
    expect(calls.find((c) => c.type === "StartExportJob")).toMatchObject({
      body: { in_us: Math.round(10e6 / 60), out_us: Math.round(31e6 / 60) },
    });
  });
});
