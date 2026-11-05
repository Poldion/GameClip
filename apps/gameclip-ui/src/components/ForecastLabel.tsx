// SPDX-License-Identifier: GPL-3.0-or-later
// „ca. N MB RAM“ für die gewählten Aufnahme-Einstellungen, mit Warnstufe (Design, Datei 65).

import { useEffect, useState } from "react";
import type { Forecast } from "../generated/Forecast";
import type { Settings } from "../generated/Settings";
import { bytes } from "../format";
import { useStore } from "../store";
import css from "./ForecastLabel.module.css";

export function ForecastLabel(props: { settings: Settings }) {
  const store = useStore();
  const [f, setF] = useState<Forecast | null>(null);
  const key = JSON.stringify(props.settings.capture);
  useEffect(() => {
    let alive = true;
    // Kurz warten, damit Schieberegler nicht bei jedem Schritt fragen.
    const t = setTimeout(() => {
      void store.ipc.getForecast(props.settings).then((r) => alive && setF(r)).catch(() => alive && setF(null));
    }, 150);
    return () => {
      alive = false;
      clearTimeout(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, store]);
  if (!f) return <span className={css.label}>&nbsp;</span>;
  return (
    <span className={css.label} data-level={f.level}>
      ca. {bytes(f.total_bytes)} RAM
      {f.level === "warning" && " – viel für diesen Rechner"}
      {f.level === "rejected" && " – zu viel für diesen Rechner, bitte kürzer oder niedriger einstellen"}
    </span>
  );
}
