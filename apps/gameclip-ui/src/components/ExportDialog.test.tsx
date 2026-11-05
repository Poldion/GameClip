// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { sampleClips } from "../ipc.mock";
import { renderWithStore } from "../test/render";
import { ExportDialog } from "./ExportDialog";

const clip = sampleClips()[1];

describe("ExportDialog", () => {
  it("schickt standardmäßig die Mischung", async () => {
    const { calls } = await renderWithStore(<ExportDialog clip={clip} inUs={1_000_000} outUs={31_000_000} onClose={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Exportieren" }));
    await waitFor(() => expect(calls.some((c) => c.type === "StartExportJob")).toBe(true));
    expect(calls.find((c) => c.type === "StartExportJob")).toEqual({
      type: "StartExportJob",
      body: { clip_id: clip.clip_id, in_us: 1_000_000, out_us: 31_000_000, profile: { kind: "Mix" } },
    });
  });

  it("schickt getrennte Spuren", async () => {
    const { calls } = await renderWithStore(<ExportDialog clip={clip} inUs={0} outUs={5_000_000} onClose={() => {}} />);
    fireEvent.click(screen.getByRole("radio", { name: /Mit getrennten Spuren/ }));
    fireEvent.click(screen.getByRole("button", { name: "Exportieren" }));
    await waitFor(() =>
      expect(calls.find((c) => c.type === "StartExportJob")).toMatchObject({ body: { profile: { kind: "SeparateTracks" } } }),
    );
  });

  it("Zielgröße: Vorab-Warnung bei zu kleinem Ziel, dann richtiges Profil", async () => {
    const { calls } = await renderWithStore(<ExportDialog clip={clip} inUs={0} outUs={60_000_000} onClose={() => {}} />);
    fireEvent.click(screen.getByRole("radio", { name: /Zielgröße/ }));
    // Standard aus den Einstellungen: 10 MB.
    expect(screen.getByRole("spinbutton", { name: "Zielgröße in MB" })).toHaveProperty("value", "10");
    fireEvent.change(screen.getByRole("spinbutton", { name: "Zielgröße in MB" }), { target: { value: "1" } });
    expect(await screen.findByText(/Zu klein für diese Länge\. Mindestens 3 MB/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Exportieren" })).toHaveProperty("disabled", true);
    expect(calls.find((c) => c.type === "GetExportEstimate")).toMatchObject({
      body: { clip_id: clip.clip_id, in_us: 0, out_us: 60_000_000, target_bytes: 1_000_000 },
    });

    fireEvent.change(screen.getByRole("spinbutton", { name: "Zielgröße in MB" }), { target: { value: "8" } });
    expect(await screen.findByText(/720p60, ca\./)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Exportieren" }));
    await waitFor(() =>
      expect(calls.find((c) => c.type === "StartExportJob")).toMatchObject({
        body: { profile: { kind: "TargetSize", bytes: 8_000_000 } },
      }),
    );
  });

  // Das Mock meldet 10 × 200 ms Fortschritt; unter Last (parallele Builds) großzügig warten.
  it("zeigt Fortschritt und am Ende Kopieren", { timeout: 20_000 }, async () => {
    await renderWithStore(<ExportDialog clip={clip} inUs={0} outUs={5_000_000} onClose={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Exportieren" }));
    expect(await screen.findByRole("progressbar")).toBeTruthy();
    expect(await screen.findByText(/NVENC/, {}, { timeout: 8000 })).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Kopieren" }, { timeout: 15_000 })).toBeTruthy();
  });

  it("ohne Einzelspuren ist „getrennte Spuren“ gesperrt", async () => {
    await renderWithStore(<ExportDialog clip={{ ...clip, has_tracks: false }} inUs={0} outUs={5_000_000} onClose={() => {}} />);
    expect(screen.getByRole("radio", { name: /Mit getrennten Spuren/ })).toHaveProperty("disabled", true);
  });
});
