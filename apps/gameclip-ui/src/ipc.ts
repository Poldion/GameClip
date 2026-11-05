// SPDX-License-Identifier: GPL-3.0-or-later
// Typisierte Befehle an den Dienst (Design, Datei 62). Die Rust-Seite von Tauri hält die Pipe;
// hier gibt es je Protokollbefehl eine Funktion. Alle Typen kommen aus src/generated (ts-rs).

import type { AutostartReply } from "./generated/AutostartReply";
import type { AutostartState } from "./generated/AutostartState";
import type { ClipEntry } from "./generated/ClipEntry";
import type { Command } from "./generated/Command";
import type { CopyDiagnosticsReply } from "./generated/CopyDiagnosticsReply";
import type { ErrorCode } from "./generated/ErrorCode";
import type { ExportProfile } from "./generated/ExportProfile";
import type { FieldError } from "./generated/FieldError";
import type { FolderKind } from "./generated/FolderKind";
import type { Forecast } from "./generated/Forecast";
import type { GetCapabilitiesReply } from "./generated/GetCapabilitiesReply";
import type { GetExportEstimateReply } from "./generated/GetExportEstimateReply";
import type { GetFrameTimesReply } from "./generated/GetFrameTimesReply";
import type { GetSettingsReply } from "./generated/GetSettingsReply";
import type { Hotkey } from "./generated/Hotkey";
import type { ListClipsReply } from "./generated/ListClipsReply";
import type { ListDevicesReply } from "./generated/ListDevicesReply";
import type { ListJobsReply } from "./generated/ListJobsReply";
import type { Response } from "./generated/Response";
import type { ServiceEvent } from "./generated/ServiceEvent";
import type { ServiceStatus } from "./generated/ServiceStatus";
import type { SetSettingsReply } from "./generated/SetSettingsReply";
import type { Settings } from "./generated/Settings";
import type { StartExportJobReply } from "./generated/StartExportJobReply";
import type { TestRegisterReply } from "./generated/TestRegisterReply";

/** Teil-Einstellungen für `SetSettings` (JSON-Merge-Patch). */
export type SettingsPatch = { [K in keyof Settings]?: Settings[K] extends object ? Partial<Settings[K]> | null : Settings[K] };

/** Verbindung zum Dienst, wie die Tauri-Seite sie meldet. */
export type Connection =
  | { state: "connecting" }
  | { state: "connected"; service_version: string }
  | { state: "service_unreachable" }
  | { state: "version_mismatch"; message: string };

/** Autostart-Zustand; ohne Verbindung zum Dienst „unavailable“. */
export type Autostart = AutostartState;

export class IpcError extends Error {
  constructor(
    public code: ErrorCode | "Disconnected",
    message: string,
    public fields: FieldError[] = [],
  ) {
    super(message);
  }
}

export type Unsubscribe = () => void;

export interface Ipc {
  getStatus(): Promise<ServiceStatus>;
  getSettings(): Promise<GetSettingsReply>;
  setSettings(partial: SettingsPatch): Promise<SetSettingsReply>;
  getForecast(settings: Settings): Promise<Forecast>;
  listDevices(): Promise<ListDevicesReply>;
  getCapabilities(): Promise<GetCapabilitiesReply>;
  startBuffer(): Promise<void>;
  stopBuffer(): Promise<void>;
  restart(): Promise<void>;
  saveClip(): Promise<void>;
  pauseMic(on: boolean): Promise<void>;
  suspendHotkey(on: boolean): Promise<void>;
  testRegister(hotkey: Hotkey): Promise<TestRegisterReply>;
  subscribeMicLevel(on: boolean): Promise<void>;
  watchFolders(on: boolean): Promise<void>;
  listClips(folder: FolderKind): Promise<ListClipsReply>;
  getClip(clipId: number): Promise<ClipEntry>;
  setStar(clipId: number, on: boolean): Promise<void>;
  adoptClip(clipId: number): Promise<void>;
  deleteClip(clipId: number): Promise<void>;
  getFrameTimes(clipId: number): Promise<GetFrameTimesReply>;
  getExportEstimate(clipId: number, inUs: number, outUs: number, targetBytes: number): Promise<GetExportEstimateReply>;
  startExportJob(clipId: number, inUs: number, outUs: number, profile: ExportProfile): Promise<StartExportJobReply>;
  cancelJob(jobId: number): Promise<void>;
  listJobs(): Promise<ListJobsReply>;
  copyDiagnostics(): Promise<CopyDiagnosticsReply>;
  getAutostart(): Promise<Autostart>;
  setAutostart(on: boolean): Promise<Autostart>;
  // Nur Oberfläche (gameclip-ui-platform):
  copyFile(clipId: number): Promise<void>;
  reveal(clipId: number): Promise<void>;
  pickFolder(current: string | null): Promise<string | null>;
  hasMediaFoundation(): Promise<boolean>;
  /** URL des Vorschaubilds (eigenes Protokoll `clip`). */
  thumbnailUrl(clipId: number): string;
  /** URL der Datei für `<video>` (eigenes Protokoll `clip`, mit HTTP-Range). */
  clipUrl(clipId: number): string;
  /** Datei aus der Galerie ziehen (Drag-and-drop nach Discord oder in den Explorer). */
  startDrag(clipId: number): Promise<void>;
  /** Teilen-Dialog von Windows mit der Datei. */
  share(clipId: number): Promise<void>;
  onEvent(cb: (ev: ServiceEvent) => void): Unsubscribe;
  onConnection(cb: (c: Connection) => void): Unsubscribe;
  connection(): Promise<Connection>;
  /** Ansicht aus dem Startargument (Tray: Galerie oder Einstellungen), einmalig. */
  initialView(): Promise<StartView | null>;
  /** Tray öffnet eine Ansicht, während das Fenster schon offen ist. */
  onOpenView(cb: (v: StartView) => void): Unsubscribe;
}

export type StartView = "gallery" | "settings";

/** Baut ein `Ipc` aus einer Funktion, die einen Befehl schickt und die Antwort liefert. */
export function makeIpc(
  send: (cmd: Command) => Promise<Response>,
  native: Pick<
    Ipc,
    "copyFile" | "reveal" | "pickFolder" | "hasMediaFoundation" | "thumbnailUrl" | "clipUrl" | "startDrag" | "share" | "onEvent" | "onConnection" | "connection" | "initialView" | "onOpenView"
  >,
): Ipc {
  async function call<T>(cmd: Command): Promise<T> {
    const r = await send(cmd);
    if (!r.ok) {
      const e = r.error;
      const fields = Array.isArray(e?.details) ? (e.details as FieldError[]) : [];
      throw new IpcError(e?.code ?? "Internal", e?.message ?? "Unbekannter Fehler", fields);
    }
    return r.body as T;
  }
  const none = async (cmd: Command) => {
    await call<unknown>(cmd);
  };
  return {
    getStatus: () => call({ type: "GetStatus" }),
    getSettings: () => call({ type: "GetSettings" }),
    setSettings: (partial) => call({ type: "SetSettings", body: { partial: partial as never } }),
    getForecast: (settings) => call({ type: "GetForecast", body: { settings } }),
    listDevices: () => call({ type: "ListDevices" }),
    getCapabilities: () => call({ type: "GetCapabilities" }),
    startBuffer: () => none({ type: "StartBuffer" }),
    stopBuffer: () => none({ type: "StopBuffer" }),
    restart: () => none({ type: "Restart" }),
    saveClip: () => none({ type: "SaveClip" }),
    pauseMic: (on) => none({ type: "PauseMic", body: { on } }),
    suspendHotkey: (on) => none({ type: "SuspendHotkey", body: { on } }),
    testRegister: (hotkey) => call({ type: "TestRegister", body: { hotkey } }),
    subscribeMicLevel: (on) => none({ type: "SubscribeMicLevel", body: { on } }),
    watchFolders: (on) => none({ type: "WatchFolders", body: { on } }),
    listClips: (folder) => call({ type: "ListClips", body: { folder } }),
    getClip: (clip_id) => call({ type: "GetClip", body: { clip_id } }),
    setStar: (clip_id, on) => none({ type: "SetStar", body: { clip_id, on } }),
    adoptClip: (clip_id) => none({ type: "AdoptClip", body: { clip_id } }),
    deleteClip: (clip_id) => none({ type: "DeleteClip", body: { clip_id } }),
    getFrameTimes: (clip_id) => call({ type: "GetFrameTimes", body: { clip_id } }),
    getExportEstimate: (clip_id, in_us, out_us, target_bytes) =>
      call({ type: "GetExportEstimate", body: { clip_id, in_us, out_us, target_bytes } }),
    startExportJob: (clip_id, in_us, out_us, profile) =>
      call({ type: "StartExportJob", body: { clip_id, in_us, out_us, profile } }),
    cancelJob: (job_id) => none({ type: "CancelJob", body: { job_id } }),
    listJobs: () => call({ type: "ListJobs" }),
    copyDiagnostics: () => call({ type: "CopyDiagnostics" }),
    // Ohne Dienst kein Autostart: der Schalter bleibt gesperrt statt einen Fehler zu zeigen.
    getAutostart: () =>
      call<AutostartReply>({ type: "GetAutostart" }).then(
        (r) => r.state,
        () => "unavailable" as const,
      ),
    setAutostart: (on) =>
      call<AutostartReply>({ type: "SetAutostart", body: { on } }).then(
        (r) => r.state,
        () => "unavailable" as const,
      ),
    ...native,
  };
}

async function tauriIpc(): Promise<Ipc> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  const sub = <T,>(name: string, cb: (v: T) => void): Unsubscribe => {
    let off: (() => void) | null = null;
    let cancelled = false;
    void listen<T>(name, (e) => cb(e.payload)).then((u) => {
      if (cancelled) u();
      else off = u;
    });
    return () => {
      cancelled = true;
      off?.();
    };
  };
  return makeIpc((cmd) => invoke<Response>("call", { cmd }), {
    copyFile: (clipId) => invoke("copy_file", { clipId }),
    reveal: (clipId) => invoke("reveal", { clipId }),
    pickFolder: (current) => invoke("pick_folder", { current }),
    hasMediaFoundation: () => invoke("has_media_foundation"),
    thumbnailUrl: (clipId) => `http://clip.localhost/thumb/${clipId}`,
    clipUrl: (clipId) => `http://clip.localhost/${clipId}`,
    startDrag: (clipId) => invoke("start_drag", { clipId }),
    share: (clipId) => invoke("share", { clipId }),
    onEvent: (cb) => sub<ServiceEvent>("service-event", cb),
    onConnection: (cb) => sub<Connection>("connection", cb),
    connection: () => invoke("connection"),
    initialView: () => invoke("initial_view"),
    onOpenView: (cb) => sub<StartView>("open-view", cb),
  });
}

let instance: Promise<Ipc> | null = null;

/** Die Verbindung der App: im Mock-Modus (`VITE_MOCK=1`, Tests) gegen Beispieldaten. */
export function ipc(): Promise<Ipc> {
  if (!instance) {
    instance =
      import.meta.env.VITE_MOCK === "1" ? import("./ipc.mock").then((m) => m.createMockIpc()) : tauriIpc();
  }
  return instance;
}

/** Für Tests: eine eigene Instanz setzen. */
export function setIpc(i: Ipc) {
  instance = Promise.resolve(i);
}
