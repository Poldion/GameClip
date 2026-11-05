// SPDX-License-Identifier: GPL-3.0-or-later
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { VolumeSlider, gainLabel } from "./VolumeSlider";

describe("VolumeSlider", () => {
  it("speichert erst beim Loslassen, Doppelklick setzt 0 dB", () => {
    const onChange = vi.fn();
    const onCommit = vi.fn();
    render(<VolumeSlider label="Mikrofon" value={-6} onChange={onChange} onCommit={onCommit} />);
    const slider = screen.getByRole("slider", { name: "Mikrofon" });
    fireEvent.change(slider, { target: { value: "3" } });
    expect(onChange).toHaveBeenLastCalledWith(3);
    expect(onCommit).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider);
    expect(onCommit).toHaveBeenCalledTimes(1);
    fireEvent.doubleClick(slider);
    expect(onChange).toHaveBeenLastCalledWith(0);
    expect(onCommit).toHaveBeenLastCalledWith(0);
  });

  it("Bereich und Anzeige", () => {
    render(<VolumeSlider label="System" value={0} onChange={() => {}} />);
    const slider = screen.getByRole("slider", { name: "System" });
    expect(slider.getAttribute("min")).toBe("-30");
    expect(slider.getAttribute("max")).toBe("12");
    expect(gainLabel(-4.5)).toBe("−4,5 dB");
    expect(gainLabel(12)).toBe("+12 dB");
  });
});
