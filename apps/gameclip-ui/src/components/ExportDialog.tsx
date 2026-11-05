// SPDX-License-Identifier: GPL-3.0-or-later
// Export-Dialog (Design, Datei 65): Mischung (Standard), getrennte Spuren oder Zielgröße. Bei
// Zielgröße fragt er vorab `GetExportEstimate` und warnt; danach Fortschritt mit Encoder
// („Export über CPU“), Abbrechen, und am Ende Kopieren bzw. Im Ordner anzeigen.

import { useEffect, useState } from "react";
import type { ClipEntry } from "../generated/ClipEntry";
import type { ExportProfile } from "../generated/ExportProfile";
import type { GetExportEstimateReply } from "../generated/GetExportEstimateReply";
import { bytes } from "../format";
import { useSelect, useStore } from "../store";
import { encoderName } from "./StatusBar";
import css from "./ExportDialog.module.css";

type Kind = ExportProfile["kind"];

type Phase =
  | { name: "choose" }
  | { name: "running"; jobId: number }
  | { name: "done"; clipId: number }
  | { name: "failed"; message: string };

export function profileFor(kind: Kind, mb: number): ExportProfile {
  switch (kind) {
    case "Mix":
      return { kind: "Mix" };
    case "SeparateTracks":
      return { kind: "SeparateTracks" };
    case "TargetSize":
      return { kind: "TargetSize", bytes: Math.round(mb * 1_000_000) };
  }
}

export function ExportDialog(props: { clip: ClipEntry; inUs: number; outUs: number; onClose: () => void }) {
  const store = useStore();
  const settings = useSelect((s) => s.settings);
  const jobs = useSelect((s) => s.jobs);
  const encoders = useSelect((s) => s.jobEncoder);
  const [kind, setKind] = useState<Kind>("Mix");
  const [mb, setMb] = useState(() => Math.round((settings?.export.target_bytes ?? 10_000_000) / 1e6));
  const [estimate, setEstimate] = useState<GetExportEstimateReply | null>(null);
  const [phase, setPhase] = useState<Phase>({ name: "choose" });
  const { clip, inUs, outUs } = props;

  // Vorab-Schätzung nur für die Zielgröße.
  useEffect(() => {
    if (kind !== "TargetSize" || !(mb > 0)) {
      setEstimate(null);
      return;
    }
    let alive = true;
    const t = setTimeout(() => {
      void store.ipc
        .getExportEstimate(clip.clip_id, inUs, outUs, Math.round(mb * 1_000_000))
        .then((r) => alive && setEstimate(r))
        .catch(() => alive && setEstimate(null));
    }, 150);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [kind, mb, clip.clip_id, inUs, outUs, store]);

  // Ende des eigenen Auftrags.
  useEffect(() => {
    if (phase.name !== "running") return;
    return store.ipc.onEvent((ev) => {
      if (ev.event === "JobCompleted" && ev.body.job_id === phase.jobId) setPhase({ name: "done", clipId: ev.body.clip_id });
      if (ev.event === "JobFailed" && ev.body.job_id === phase.jobId) setPhase({ name: "failed", message: ev.body.message });
    });
  }, [phase, store]);

  async function start() {
    try {
      const r = await store.ipc.startExportJob(clip.clip_id, inUs, outUs, profileFor(kind, mb));
      setPhase({ name: "running", jobId: r.job_id });
      void store.refreshJobs();
    } catch (e) {
      setPhase({ name: "failed", message: String((e as Error).message ?? e) });
    }
  }

  const tooSmall = kind === "TargetSize" && estimate?.too_small;
  const job = phase.name === "running" ? jobs.find((j) => j.job_id === phase.jobId) : undefined;
  const encoder = phase.name === "running" ? encoders[phase.jobId] : undefined;
  const ahead = job?.state === "queued" ? jobs.filter((j) => j.job_id !== job.job_id && j.state === "running").length : 0;

  return (
    <div className={css.backdrop}>
      <div className={css.dialog} role="dialog" aria-label="Exportieren">
        <h2>Exportieren</h2>
        <p className={css.span}>
          {((outUs - inUs) / 1e6).toLocaleString("de-DE", { maximumFractionDigits: 2 })} s aus „{clip.path.split(/[\\/]/).pop()}“
        </p>

        {phase.name === "choose" && (
          <>
            <fieldset className={css.choices}>
              <label>
                <input type="radio" name="profile" checked={kind === "Mix"} onChange={() => setKind("Mix")} />
                <span>
                  <b>Mischung</b>
                  <small>Eine Tonspur mit allem, für Teilen und Abspielen.</small>
                </span>
              </label>
              <label data-disabled={!clip.has_tracks || undefined}>
                <input
                  type="radio"
                  name="profile"
                  checked={kind === "SeparateTracks"}
                  disabled={!clip.has_tracks}
                  onChange={() => setKind("SeparateTracks")}
                />
                <span>
                  <b>Mit getrennten Spuren</b>
                  <small>
                    {clip.has_tracks
                      ? "Mischung als Standard, dazu System, Mikrofon und Discord einzeln für den Schnitt."
                      : "Für diesen Clip gibt es keine Einzelspuren."}
                  </small>
                </span>
              </label>
              <label>
                <input type="radio" name="profile" checked={kind === "TargetSize"} onChange={() => setKind("TargetSize")} />
                <span>
                  <b>Zielgröße</b>
                  <small>Kleiner als eine Grenze, z. B. 10 MB für Discord.</small>
                </span>
              </label>
            </fieldset>
            {kind === "TargetSize" && (
              <div className={css.size}>
                <label>
                  Höchstens{" "}
                  <input
                    type="number"
                    min={1}
                    max={2000}
                    step={1}
                    value={mb}
                    aria-label="Zielgröße in MB"
                    onChange={(e) => setMb(Number(e.target.value))}
                  />{" "}
                  MB
                </label>
                <p className={css.estimate} role="status">
                  {estimate?.too_small &&
                    `Zu klein für diese Länge. Mindestens ${Math.ceil(estimate.too_small.min_target_bytes / 1e6)} MB oder kürzer schneiden.`}
                  {estimate?.plan &&
                    `${estimate.plan.height}p${estimate.plan.fps}, ca. ${(estimate.plan.video_bps / 1e6).toLocaleString("de-DE", { maximumFractionDigits: 1 })} Mbit/s`}
                  {estimate?.plan?.warn_low_quality && " – Qualität wird sehr niedrig"}
                </p>
              </div>
            )}
            <div className={css.buttons}>
              <button onClick={props.onClose}>Abbrechen</button>
              <button className="primary" disabled={!!tooSmall || (kind === "TargetSize" && !(mb > 0))} onClick={() => void start()}>
                Exportieren
              </button>
            </div>
          </>
        )}

        {phase.name === "running" && (
          <>
            <div className={css.progress} role="progressbar" aria-valuenow={Math.round(job?.percent ?? 0)} aria-valuemin={0} aria-valuemax={100}>
              <span style={{ width: `${job?.percent ?? 0}%` }} />
            </div>
            <p className={css.state}>
              {job?.state === "queued"
                ? `Wartet${ahead ? ` (${ahead} Export läuft)` : ""} …`
                : `${Math.round(job?.percent ?? 0)} %${encoder ? ` · ${encoder.includes("x264") ? "Export über CPU" : encoderName(encoder)}` : ""}`}
            </p>
            <div className={css.buttons}>
              <button
                onClick={() =>
                  void store.ipc.cancelJob(phase.jobId).then(() => {
                    void store.refreshJobs();
                    props.onClose();
                  })
                }
              >
                Abbrechen
              </button>
            </div>
          </>
        )}

        {phase.name === "done" && (
          <>
            <p className={css.ok}>Fertig. Die Datei liegt im Export-Ordner.</p>
            <div className={css.buttons}>
              <button onClick={() => void store.ipc.reveal(phase.clipId)}>Im Ordner anzeigen</button>
              <button
                className="primary"
                onClick={() =>
                  void store.ipc
                    .copyFile(phase.clipId)
                    .then(() => store.notify("Datei kopiert. Mit Strg+V z. B. in Discord einfügen."))
                }
              >
                Kopieren
              </button>
              <button onClick={props.onClose}>Schließen</button>
            </div>
          </>
        )}

        {phase.name === "failed" && (
          <>
            <p className={css.error}>Export fehlgeschlagen: {phase.message}</p>
            <div className={css.buttons}>
              <button onClick={() => setPhase({ name: "choose" })}>Zurück</button>
              <button onClick={props.onClose}>Schließen</button>
            </div>
          </>
        )}
        {phase.name === "choose" && kind === "TargetSize" && estimate?.plan && (
          <p className={css.hint}>Ziel {bytes(Math.round(mb * 1_000_000))}; wird es überschritten, folgt ein zweiter, kleinerer Versuch.</p>
        )}
      </div>
    </div>
  );
}
