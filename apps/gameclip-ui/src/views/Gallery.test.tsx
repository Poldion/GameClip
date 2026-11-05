// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { sampleClips } from "../ipc.mock";
import { renderWithStore } from "../test/render";
import { Gallery } from "./Gallery";

describe("Gallery", () => {
  it("markiert Fremd-Clips und rechnet sie nicht in die Platzanzeige", async () => {
    await renderWithStore(<Gallery />);
    const own = sampleClips().filter((c) => c.status === "ok" && c.in_current_folder);
    const used = own.reduce((a, c) => a + c.size_bytes, 0);
    const expected = `${(used / 1e9).toLocaleString("de-DE", { minimumFractionDigits: 1, maximumFractionDigits: 1 })} / 10 GB`;
    expect(screen.getByTestId("usage").textContent).toBe(expected);

    const foreign = screen.getByRole("article", { name: "kopie.mp4" });
    expect(within(foreign).getByText("Fremd")).toBeTruthy();
    expect(within(foreign).getByRole("button", { name: "Als eigenen Clip übernehmen" })).toBeTruthy();
  });

  it("zeigt Clips aus früheren Ordnern abgesetzt", async () => {
    await renderWithStore(<Gallery />);
    const old = screen.getByRole("article", { name: /2026-10-01/ });
    expect(old.getAttribute("data-old")).toBe("true");
    expect(within(old).getByText(/früherer Ordner/)).toBeTruthy();
  });

  it("übernimmt einen Fremd-Clip", async () => {
    const { ipc } = await renderWithStore(<Gallery />);
    fireEvent.click(screen.getByRole("button", { name: "Als eigenen Clip übernehmen" }));
    await waitFor(() => expect(ipc.state.clips.find((c) => c.clip_id === 3)?.status).toBe("ok"));
  });

  it("überwacht die Ordner nur, solange sie offen ist", async () => {
    const { calls, unmount } = await renderWithStore(<Gallery />);
    expect(calls).toContainEqual({ type: "WatchFolders", body: { on: true } });
    unmount();
    expect(calls.at(-1)).toEqual({ type: "WatchFolders", body: { on: false } });
  });

  it("löscht erst nach Rückfrage", async () => {
    const { ipc } = await renderWithStore(<Gallery />);
    const card = screen.getByRole("article", { name: /cs2_2026-11-02/ });
    fireEvent.click(within(card).getByRole("button", { name: "Mehr" }));
    fireEvent.click(await screen.findByRole("menuitem", { name: "Löschen …" }));
    expect(ipc.state.clips.some((c) => c.clip_id === 2)).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Endgültig löschen" }));
    await waitFor(() => expect(ipc.state.clips.some((c) => c.clip_id === 2)).toBe(false));
  });

  it("schützt mit dem Stern", async () => {
    const { ipc } = await renderWithStore(<Gallery />);
    const card = screen.getByRole("article", { name: /cs2_2026-11-02/ });
    fireEvent.click(within(card).getByRole("button", { name: "Mit Stern schützen" }));
    await waitFor(() => expect(ipc.state.clips.find((c) => c.clip_id === 2)?.starred).toBe(true));
  });

  it("gruppiert nach Spiel, das neueste Spiel zuerst, ohne Spiel zuletzt", async () => {
    await renderWithStore(<Gallery />);
    const titles = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent);
    // Neuester Clip: Valorant (11-03 Fremd ist „Ohne Spiel“), dann cs2 (11-02).
    expect(titles).toEqual(["Valorant 2 Clips", "cs2 1 Clip", "Ohne Spiel 1 Clip"]);
    expect(within(screen.getByRole("region", { name: "cs2" })).getAllByRole("article")).toHaveLength(1);
  });

  it("sortiert auf Wunsch nur nach Datum", async () => {
    await renderWithStore(<Gallery />);
    fireEvent.change(screen.getByRole("combobox", { name: "Sortieren" }), { target: { value: "date" } });
    expect(screen.queryAllByRole("heading", { level: 2 })).toHaveLength(0);
    expect(screen.getAllByRole("article")).toHaveLength(4);
  });

  it("öffnet den Teilen-Dialog", async () => {
    const { ipc } = await renderWithStore(<Gallery />);
    const card = screen.getByRole("article", { name: /cs2_2026-11-02/ });
    fireEvent.click(within(card).getByRole("button", { name: "Teilen" }));
    await waitFor(() => expect(ipc.state.shared).toEqual([2]));
  });
});
