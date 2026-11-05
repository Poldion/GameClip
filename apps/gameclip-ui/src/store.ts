// SPDX-License-Identifier: GPL-3.0-or-later
// Externer Store über useSyncExternalStore (Design, Datei 62): Status, Clips, Exporte, Aufträge,
// Einstellungen. Ereignisse des Dienstes halten ihn aktuell; keine weitere State-Bibliothek.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { FolderKind } from "./generated/FolderKind";
import type { JobInfo } from "./generated/JobInfo";
import type { ListClipsReply } from "./generated/ListClipsReply";
import type { ServiceEvent } from "./generated/ServiceEvent";
import type { ServiceStatus } from "./generated/ServiceStatus";
import type { Settings } from "./generated/Settings";
import type { Connection, Ipc } from "./ipc";
import { bytes, duration } from "./format";

export interface Notice {
  id: number;
  kind: "info" | "error";
  text: string;
}

export interface State {
  connection: Connection;
  status: ServiceStatus | null;
  settings: Settings | null;
  readOnly: boolean;
  clips: ListClipsReply | null;
  exports: ListClipsReply | null;
  jobs: JobInfo[];
  /** Encoder je laufendem Auftrag (aus `JobProgress`). */
  jobEncoder: Record<number, string>;
  micLevel: number | null;
  notices: Notice[];
}

export class Store {
  private state: State = {
    connection: { state: "connecting" },
    status: null,
    settings: null,
    readOnly: false,
    clips: null,
    exports: null,
    jobs: [],
    jobEncoder: {},
    micLevel: null,
    notices: [],
  };
  private listeners = new Set<() => void>();
  private offs: (() => void)[] = [];
  private nextNotice = 1;

  constructor(public ipc: Ipc) {}

  get = () => this.state;

  subscribe = (l: () => void) => {
    this.listeners.add(l);
    return () => {
      this.listeners.delete(l);
    };
  };

  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    this.listeners.forEach((l) => l());
  }

  /** Verbindet Ereignisse und lädt den Anfangszustand. */
  async start() {
    this.offs.push(this.ipc.onEvent((ev) => this.onEvent(ev)));
    this.offs.push(
      this.ipc.onConnection((c) => {
        this.set({ connection: c });
        if (c.state === "connected") void this.loadAll();
      }),
    );
    const c = await this.ipc.connection();
    this.set({ connection: c });
    if (c.state === "connected") await this.loadAll();
  }

  stop() {
    this.offs.forEach((o) => o());
    this.offs = [];
  }

  async loadAll() {
    const [status, s, clips, exports, jobs] = await Promise.all([
      this.ipc.getStatus(),
      this.ipc.getSettings(),
      this.ipc.listClips("clips"),
      this.ipc.listClips("exports"),
      this.ipc.listJobs(),
    ]);
    this.set({ status, settings: s.settings, readOnly: s.read_only, clips, exports, jobs: jobs.jobs });
  }

  async refreshFolder(folder: FolderKind) {
    const list = await this.ipc.listClips(folder);
    this.set(folder === "clips" ? { clips: list } : { exports: list });
  }

  async refreshJobs() {
    this.set({ jobs: (await this.ipc.listJobs()).jobs });
  }

  setSettings(settings: Settings) {
    this.set({ settings });
  }

  notify(text: string, kind: Notice["kind"] = "info") {
    const n = { id: this.nextNotice++, kind, text };
    this.set({ notices: [...this.state.notices, n] });
    setTimeout(() => this.dismiss(n.id), 6000);
  }

  dismiss(id: number) {
    this.set({ notices: this.state.notices.filter((n) => n.id !== id) });
  }

  private onEvent(ev: ServiceEvent) {
    switch (ev.event) {
      case "StateChanged":
        this.set({ status: ev.body });
        break;
      case "SaveCompleted":
        this.notify(`Clip gespeichert (${duration(ev.body.duration_us)}, ${bytes(ev.body.size_bytes)})`);
        break;
      case "SaveFailed":
        this.notify(`Speichern fehlgeschlagen: ${ev.body.message}`, "error");
        break;
      case "NewClip":
      case "ClipDeleted":
      case "ClipsChanged":
        void this.refreshFolder(ev.body.folder);
        break;
      case "StorageWarning":
        this.notify(
          ev.body.reason === "all_starred"
            ? "Speicherlimit überschritten: Alle Clips haben einen Stern, nichts wurde gelöscht."
            : "Speicherlimit überschritten: Ein einzelner Clip ist größer als das Limit.",
          "error",
        );
        break;
      case "JobProgress": {
        const jobs = this.state.jobs.map((j) =>
          j.job_id === ev.body.job_id ? { ...j, percent: ev.body.percent, state: "running" as const } : j,
        );
        this.set({ jobs, jobEncoder: { ...this.state.jobEncoder, [ev.body.job_id]: ev.body.encoder } });
        break;
      }
      case "JobCompleted":
        void this.refreshJobs();
        break;
      case "JobFailed":
        this.notify(`Export fehlgeschlagen: ${ev.body.message}`, "error");
        void this.refreshJobs();
        break;
      case "MicLevel":
        this.set({ micLevel: ev.body.dbfs });
        break;
      case "FormatChanged":
        void this.ipc.getStatus().then((status) => this.set({ status }));
        break;
    }
  }
}

export const StoreContext = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(StoreContext);
  if (!s) throw new Error("StoreContext fehlt");
  return s;
}

/** Wählt einen Teil des Zustands; rendert nur neu, wenn er sich ändert. */
export function useSelect<T>(pick: (s: State) => T): T {
  const store = useStore();
  return useSyncExternalStore(store.subscribe, () => pick(store.get()));
}
