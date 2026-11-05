// SPDX-License-Identifier: GPL-3.0-or-later
// Dieselbe Schnittstelle wie ipc.ts mit Beispieldaten und simuliertem Fortschritt (Design,
// Datei 62). Läuft bei `VITE_MOCK=1` (npm run dev:mock) und in den Tests.

import type { ClipEntry } from "./generated/ClipEntry";
import type { Command } from "./generated/Command";
import type { FolderKind } from "./generated/FolderKind";
import type { JobInfo } from "./generated/JobInfo";
import type { Response } from "./generated/Response";
import type { ServiceEvent } from "./generated/ServiceEvent";
import type { ServiceStatus } from "./generated/ServiceStatus";
import type { Settings } from "./generated/Settings";
import { type Autostart, type Connection, type Ipc, makeIpc } from "./ipc";
import { hotkeyError } from "./hotkey";

export function defaultSettings(): Settings {
  return {
    schema_version: 1,
    first_run_done: false,
    capture: { buffer_seconds: 60, resolution: "1080p", fps: 60, quality: "medium", monitor: null, cursor: true },
    audio: { mic_enabled: true, mic_device: null, discord_enabled: false, gain_db: { system: 0, mic: 0, discord: 0 } },
    hotkey: { code: "F8", ctrl: false, alt: false, shift: false },
    storage: { clip_folder: null, export_folder: null, clip_limit_bytes: 10_000_000_000, export_limit_bytes: 10_000_000_000 },
    export: { target_bytes: 10_000_000 },
    sound: { enabled: true, volume: 0.7 },
  };
}

function clip(id: number, over: Partial<ClipEntry> = {}): ClipEntry {
  return {
    clip_id: id,
    tag_id: `01JC4V7Q1S6M8B2X3Y4Z5A6B${id.toString().padStart(2, "0")}`,
    path: `C:\\Users\\Spieler\\Videos\\GameClip\\Clips\\Valorant_2026-11-0${id % 9}_21-14-03.mp4`,
    size_bytes: 140_000_000 + id * 1_000_000,
    created_at: `2026-11-0${id % 9}T20:14:03Z`,
    duration_us: 60_000_000,
    starred: false,
    has_tracks: true,
    status: "ok",
    placeholder: false,
    in_current_folder: true,
    ...over,
  };
}

/** Beispieldaten: eigene Clips, ein fremder, einer aus einem früheren Ordner. */
export function sampleClips(): ClipEntry[] {
  return [
    clip(1, { starred: true }),
    clip(2),
    clip(3, { status: "foreign", path: "C:\\Users\\Spieler\\Videos\\GameClip\\Clips\\kopie.mp4" }),
    clip(4, { in_current_folder: false, path: "D:\\Alt\\Clips\\Valorant_2026-10-01_20-00-00.mp4" }),
  ];
}

export interface MockState {
  settings: Settings;
  readOnly: boolean;
  clips: ClipEntry[];
  exports: ClipEntry[];
  jobs: JobInfo[];
  status: ServiceStatus;
  calls: Command[];
  /** Tasten, die „ein anderes Programm“ schon belegt hat. */
  takenKeys: string[];
  autostart: Autostart;
  mediaFoundation: boolean;
  connection: Connection;
}

export function mockState(): MockState {
  return {
    settings: defaultSettings(),
    readOnly: false,
    clips: sampleClips(),
    exports: [],
    jobs: [],
    status: {
      state: { kind: "buffering" },
      restrictions: [],
      mic_paused: false,
      saves_in_flight: 0,
      encoder: "h264_nvenc",
      capture_resolution: [1920, 1080],
      buffered_us: 60_000_000,
      forecast_total_bytes: 905_969_664,
    },
    calls: [],
    takenKeys: ["F9"],
    autostart: "enabled",
    mediaFoundation: true,
    connection: { state: "connected", service_version: "0.1.0" },
  };
}

function mergePatch(target: unknown, patch: unknown): unknown {
  if (patch === null || typeof patch !== "object" || Array.isArray(patch)) return patch;
  const out: Record<string, unknown> =
    target && typeof target === "object" && !Array.isArray(target) ? { ...(target as Record<string, unknown>) } : {};
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === null) delete out[k];
    else out[k] = mergePatch(out[k], v);
  }
  return out;
}

export function createMockIpc(state: MockState = mockState()): Ipc & { state: MockState; emit(ev: ServiceEvent): void } {
  const listeners = new Set<(ev: ServiceEvent) => void>();
  const connListeners = new Set<(c: Connection) => void>();
  const emit = (ev: ServiceEvent) => listeners.forEach((l) => l(ev));
  let nextJob = 1;
  let nextClip = 100;
  let micTimer: ReturnType<typeof setInterval> | null = null;

  const ok = (id: number, body: unknown = {}): Response => ({ id, ok: true, body: body as never, error: null as never });
  const err = (id: number, code: string, message: string, details?: unknown): Response =>
    ({ id, ok: false, body: null as never, error: { code, message, details } }) as unknown as Response;

  const list = (folder: FolderKind) => (folder === "clips" ? state.clips : state.exports);
  const find = (clipId: number) => [...state.clips, ...state.exports].find((c) => c.clip_id === clipId);

  async function send(cmd: Command): Promise<Response> {
    state.calls.push(cmd);
    const id = state.calls.length;
    switch (cmd.type) {
      case "GetStatus":
        return ok(id, state.status);
      case "GetSettings":
        return ok(id, { settings: state.settings, read_only: state.readOnly });
      case "SetSettings": {
        const merged = mergePatch(state.settings, cmd.body.partial) as Settings;
        const keyErr = hotkeyError(merged.hotkey);
        if (keyErr) return err(id, "InvalidArgument", "Ungültige Einstellungen", [{ key: "hotkey", reason: keyErr }]);
        state.settings = merged;
        return ok(id, { settings: merged });
      }
      case "GetForecast": {
        const s = cmd.body.settings.capture;
        const mbps = { "720p": 8, "1080p": 12, "1440p": 20, "2160p": 40 }[s.resolution] * (s.fps / 60);
        const buffer = Math.round((mbps * 1_000_000 * s.buffer_seconds) / 8);
        const total = buffer + 200_000_000;
        return ok(id, { buffer_bytes: buffer, reserve_bytes: 200_000_000, total_bytes: total, level: total > 3e9 ? "warning" : "ok" });
      }
      case "ListDevices":
        return ok(id, {
          monitors: [
            { id: "\\\\.\\DISPLAY1", name: "Monitor 1 (Hauptmonitor)", primary: true, width: 2560, height: 1440, hdr: false },
            { id: "\\\\.\\DISPLAY2", name: "Monitor 2", primary: false, width: 1920, height: 1080, hdr: false },
          ],
          mics: [
            { id: "{mic-1}", name: "Headset-Mikrofon (USB)", default: true },
            { id: "{mic-2}", name: "Webcam-Mikrofon", default: false },
          ],
        });
      case "GetCapabilities":
        return ok(id, { separate_tracks: true, max_buffer_s: 300, forecast: true, encoders: ["h264_nvenc", "libx264"], hdr_supported: false });
      case "StartBuffer":
      case "StopBuffer":
      case "Restart":
        return ok(id);
      case "SaveClip": {
        const c = clip(nextClip++, { created_at: new Date().toISOString() });
        state.clips = [c, ...state.clips];
        setTimeout(() => {
          emit({ event: "SaveCompleted", body: { clip_id: c.clip_id, path: c.path, duration_us: c.duration_us, size_bytes: c.size_bytes, has_tracks: true, shortened: false } });
          emit({ event: "NewClip", body: { clip_id: c.clip_id, folder: "clips" } });
        }, 50);
        return ok(id);
      }
      case "PauseMic":
        state.status = { ...state.status, mic_paused: cmd.body.on };
        return ok(id);
      case "SuspendHotkey":
      case "WatchFolders":
        return ok(id);
      case "SubscribeMicLevel":
        if (micTimer) clearInterval(micTimer);
        micTimer = cmd.body.on
          ? setInterval(() => emit({ event: "MicLevel", body: { dbfs: -40 + Math.random() * 30 } }), 100)
          : null;
        return ok(id);
      case "TestRegister": {
        const taken = state.takenKeys.includes(cmd.body.hotkey.code);
        return ok(id, { free: !taken, error: taken ? 1409 : null });
      }
      case "ListClips": {
        const clips = list(cmd.body.folder);
        const used = clips.filter((c) => c.status === "ok" && c.in_current_folder).reduce((a, c) => a + c.size_bytes, 0);
        const limit = cmd.body.folder === "clips" ? state.settings.storage.clip_limit_bytes : state.settings.storage.export_limit_bytes;
        return ok(id, { clips, used_bytes: used, limit_bytes: limit });
      }
      case "GetClip": {
        const c = find(cmd.body.clip_id);
        return c ? ok(id, c) : err(id, "NotFound", `Clip ${cmd.body.clip_id} gibt es nicht`);
      }
      case "SetStar": {
        const c = find(cmd.body.clip_id);
        if (!c) return err(id, "NotFound", "unbekannt");
        c.starred = cmd.body.on;
        return ok(id);
      }
      case "AdoptClip": {
        const c = find(cmd.body.clip_id);
        if (!c) return err(id, "NotFound", "unbekannt");
        c.status = "ok";
        return ok(id);
      }
      case "DeleteClip": {
        const c = find(cmd.body.clip_id);
        if (!c) return err(id, "NotFound", "unbekannt");
        const folder: FolderKind = state.clips.includes(c) ? "clips" : "exports";
        state.clips = state.clips.filter((x) => x !== c);
        state.exports = state.exports.filter((x) => x !== c);
        setTimeout(() => emit({ event: "ClipDeleted", body: { clip_id: c.clip_id, folder } }), 0);
        return ok(id);
      }
      case "GetFrameTimes": {
        const c = find(cmd.body.clip_id);
        if (!c) return err(id, "NotFound", "unbekannt");
        const n = Math.round((c.duration_us / 1_000_000) * 60);
        return ok(id, { frames_us: Array.from({ length: n }, (_, i) => Math.round((i * 1_000_000) / 60)), duration_us: c.duration_us });
      }
      case "GetExportEstimate": {
        const { in_us, out_us, target_bytes } = cmd.body;
        const seconds = (out_us - in_us) / 1_000_000;
        const min = Math.round(seconds * 40_000);
        if (target_bytes < min) return ok(id, { plan: null, too_small: { min_target_bytes: min } });
        const bps = Math.round((target_bytes * 8) / seconds) - 128_000;
        return ok(id, { plan: { width: bps < 2_000_000 ? 1280 : 1920, height: bps < 2_000_000 ? 720 : 1080, fps: 60, video_bps: bps, warn_low_quality: bps < 1_000_000 }, too_small: null });
      }
      case "StartExportJob": {
        const job: JobInfo = { job_id: nextJob++, clip_id: cmd.body.clip_id, profile: cmd.body.profile, state: "running", percent: 0 };
        state.jobs.push(job);
        const timer = setInterval(() => {
          job.percent = Math.min(100, job.percent + 10);
          emit({ event: "JobProgress", body: { job_id: job.job_id, percent: job.percent, encoder: "h264_nvenc" } });
          if (job.percent >= 100) {
            clearInterval(timer);
            state.jobs = state.jobs.filter((j) => j !== job);
            const out = clip(nextClip++, { path: "C:\\Users\\Spieler\\Videos\\GameClip\\Exporte\\export.mp4" });
            state.exports = [out, ...state.exports];
            emit({ event: "JobCompleted", body: { job_id: job.job_id, clip_id: out.clip_id } });
            emit({ event: "NewClip", body: { clip_id: out.clip_id, folder: "exports" } });
          }
        }, 200);
        return ok(id, { job_id: job.job_id });
      }
      case "CancelJob":
        state.jobs = state.jobs.filter((j) => j.job_id !== cmd.body.job_id);
        return ok(id);
      case "ListJobs":
        return ok(id, { jobs: state.jobs });
      case "CopyDiagnostics":
        return ok(id, { text: "GameClip 0.1.0 – Diagnose\nWindows: 25H2 (Mock)\n" });
      case "Hello":
        return err(id, "InvalidArgument", "Hello nur einmal");
    }
  }

  const ipc = makeIpc(send, {
    copyFile: async () => {},
    reveal: async () => {},
    pickFolder: async () => "C:\\Users\\Spieler\\Videos\\Neu",
    getAutostart: async () => state.autostart,
    setAutostart: async (on) => {
      state.autostart = on ? "enabled" : "disabled";
      return state.autostart;
    },
    hasMediaFoundation: async () => state.mediaFoundation,
    thumbnailUrl: () =>
      "data:image/svg+xml;utf8," +
      encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="160" height="90"><rect width="160" height="90" fill="#3a3f4b"/></svg>'),
    onEvent: (cb) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    onConnection: (cb) => {
      connListeners.add(cb);
      return () => connListeners.delete(cb);
    },
    connection: async () => state.connection,
    initialView: async () => null,
    onOpenView: () => () => {},
  });
  return Object.assign(ipc, { state, emit });
}
