# Goldfish Review Report: Stage 9, Implementation Readiness Test

**Target:** `D:\ProgrammierProjekte\GameClip\docs\designs\gameclip.md` (`762-1370`, "## Detailed Implementation Plan" to the end of the file)
**Mode:** REVIEW. **GoldfishContext:** `D:\ProgrammierProjekte\GameClip\concepts_brief.md`. **Project state:** greenfield, no code yet. codebase-details checked the named APIs, crates, FFmpeg options, Windows and MSIX facts, and internal consistency.

## Reviewer Status

| Reviewer | Status | Raw findings (B / M / N) |
| --- | --- | --- |
| `egm-implementation-readiness` | completed | 3 / 27 / 9 |
| `egm-codebase-details` | completed | 0 / 28 / 13 |
| `egm-style` | completed | 0 / 7 / 14 |

Each reviewer ran once and needed no retry.

**After deduplication:** 73 findings. 🔴 3 Blocker, 🟡 43 Major, 🟢 27 Nit.

**How findings were merged:** When reviewers reported the same issue, they share one entry. The id belongs to the reviewer whose report was most complete, and the entry lists the other reviewers as co-reporters. The merged entry takes the highest severity any reviewer proposed. All line numbers refer to the target file. Line numbers below 762 are cross-references to earlier sections only.

**Verdict:** NOT READY. Two shared contracts are undefined: the IPC message bodies and the settings.toml schema. The frame-time mapping that frame-accurate trimming depends on is also undefined. In many places the plan contradicts the Technical Plan or contains wrong technical facts.

---

## Prioritized Findings (Blockers -> Major -> Nits)

### 🔴 implementation-readiness-1-1: IPC message bodies are undefined - Severity: Blocker
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `983-986` (also 879-882, 1132, 1252)
* **Discrepancy Description:** messages.rs says only "alle Befehle und Ereignisse aus dem Technischen Plan". Lines 362-368 name the messages but give almost none of their fields. These are undefined:
  * the replies to GetStatus, GetSettings, ListDevices, ListClips and GetFrameTimes
  * the bodies of StateChanged, SaveCompleted, SaveFailed{reason}, NewClip, ClipDeleted, FormatChanged and StorageWarning
  * the key encoding in TestRegister{key}
  * the merge semantics of SetSettings{partial}, and how `Vec<Feldfehler>` maps onto InvalidArgument
  * the server's reply to Hello
  * whether JobFailed carries `reason` (line 340) or `code` (line 338)

  docs/protocol.md (item 10) is supposed to define all of this, but it is itself a file still to be written.
* **Rationale & Recommendation:** The service, the UI, the fake daemon and a later Linux version all share this contract. Field names cannot change later without a major-version bump. Add a message table to the plan or to the content of item 10, with name, direction, Rust type, JSON fields with types, and units. Define the Hello exchange, including the server reply and the close behaviour.

### 🔴 implementation-readiness-1-2: The settings.toml schema has no keys, types or encodings - Severity: Blocker
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #22 and `egm-style` #4)
* **Affected Scope:** `766-797`, `930-931` (also 1089, 1270)
* **Discrepancy Description:** Settings is to contain "allen Feldern aus „Festgelegte Standardwerte“". That table has three problems:
  * It mixes in things that are not settings: minimum build, App-ID, pipe name with runtime SID, mutex, repository URL, and the measurement target "Dienst-RAM ≤ 80 MB".
  * It lacks fields the plan uses elsewhere: `first_run_done` (1270), mic device ID (1089), monitor identity, hotkey modifier and key encoding, tone on/off, autostart mirror, quality tier, and the unit of the export target size.
  * It gives no TOML key names, no types (MB or bytes, dB as float), and no representation for "Windows-Standardgerät" or "primärer Monitor".
* **Rationale & Recommendation:** settings.toml is a persisted format with forward-only migrations (item 17), so whatever v1 contains is permanent. Split the table into three: user settings (the Settings struct), fixed identifiers (constants), and acceptance targets (link to the Verification Plan). Add a schema table with TOML key, Rust type, unit, default, allowed range, and whether the field affects the pipeline. Define the sentinel or null for the default device and primary monitor, and how a monitor is identified across reboots.

### 🔴 implementation-readiness-1-3: GetFrameTimes and in_pts/out_pts have no unit and no mapping to the video timeline - Severity: Blocker
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1285-1286`, `1299` (also 1145)
* **Discrepancy Description:** The plan does not say:
  * which timebase is used (ms float, 90 kHz ticks or the stream timebase)
  * whether out_pts is inclusive or exclusive
  * how a stream PTS maps to `<video>.currentTime` when the MP4 has an edit list (line 1145 writes t0 and AAC priming as an edit list, which shifts presentation time)
  * which file implements GetFrameTimes on the service side, and whether the result is cached
* **Rationale & Recommendation:** G7 (frame-accurate trimming) depends on this mapping. Define it, for example as integer microseconds of presentation time after the edit list, with out exclusive. Name the implementing file, for example `gameclipd/src/frames.rs` on top of gameclip-media. Add a test that maps currentTime to a frame index.

---

### 🟡 implementation-readiness-1-4: Default export folder contradicts the storage-location section - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #1 and `egm-style` #3)
* **Affected Scope:** `786-787` (earlier: 416, 419, 424)
* **Discrepancy Description:** The plan sets `%USERPROFILE%\Videos\GameClip\Export`, which nests the export folder inside the clip folder. Line 416 gives `<Videos>\GameClip Export` and line 424 says the two folders sit side by side. Line 419 requires `SHGetKnownFolderPath(FOLDERID_Videos)` so that OneDrive redirection is detected, which a hard-coded `%USERPROFILE%` path defeats.
* **Rationale & Recommendation:** Use `<FOLDERID_Videos>\GameClip` and `<FOLDERID_Videos>\GameClip Export`, resolved at runtime rather than stored as constants.

### 🟡 implementation-readiness-1-5: Crate manifests and module files are missing, and some paths are relative - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #7 and #8)
* **Affected Scope:** `905-1237` (in particular 979, 993, 1001, 1049-1103, 1119-1206, 1210, 1228, 1241)
* **Discrepancy Description:** These files are not in the list:
  * Cargo.toml for gameclip-protocol, gameclip-fake-daemon, gameclip-capture-win, gameclipd, gameclip-worker and gameclip-ui-platform
  * lib.rs for gameclip-media and gameclip-capture-win, and `src/audio/mod.rs`
  * `apps/gameclip-ui/src-tauri/build.rs` and `icons/icon.ico`, which tauri-build needs on Windows
  * the cargo-about config and template, the vitest setup, and `.cargo/config.toml`

  Entries 32-58 use crate-relative paths such as `src/d3d.rs`. The dependency graph between the crates is not stated, for example whether protocol depends on core, or worker on capture-win.
* **Rationale & Recommendation:** The standard requires exact repository-relative paths so the list can serve as a write allowlist. List every file with its full path and give the dependencies of each crate.

### 🟡 implementation-readiness-1-6: Dependencies are unpinned and incomplete (Zero-Decision Rule) - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #2)
* **Affected Scope:** `822`, `829`, `909`, `920`, `1003`, `1100`, `1124`, `1189`, `1233`, `1258`
* **Discrepancy Description:** The plan requires "feste Versionen" but gives no versions and no Rust version.
  * **Crates used but missing from `[workspace.dependencies]`:** thiserror, rubato, crossbeam(-channel), interprocess, tauri, tauri-build, tauri-plugin-drag, tracing-subscriber, bitflags, a WAV parser, PNG/ICO decoding, and a date/time crate.
  * **Undecided time type:** naming.rs takes `zeit` and the index stores created_at, but neither the time type nor local vs. UTC is decided.
  * **Missing features:** the full `windows` feature list, the rsmpeg FFmpeg-version feature, uuid `v4`/`serde`, and serde `derive`.
  * **Missing npm packages:** `@tauri-apps/api`, `@tauri-apps/cli`, the drag plugin JS package and jsdom.
* **Rationale & Recommendation:** Add a table with crate, exact version, features and consuming crate, plus the npm list. Choose the time type, with local time for file names.

### 🟡 implementation-readiness-1-7: The cargo-deny platform ban cannot be scoped as written - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #4)
* **Affected Scope:** `837` (also 1344)
* **Discrepancy Description:** `[bans] deny` applies to the whole workspace graph, and gameclipd legitimately uses `windows`. The only scoping is `wrappers`, which covers direct parents only, so every third-party crate that uses windows/windows-sys would need listing (tauri, tray-icon, wasapi, interprocess, ...). On the MSVC target, uuid -> getrandom can also pull in windows-sys transitively.
* **Rationale & Recommendation:** Specify the mechanism exactly. For example, CI fails if `cargo tree -p gameclip-core -p gameclip-protocol -e normal --target x86_64-pc-windows-msvc -i windows-sys` (and `-i windows`) prints anything. The brief proposes this check at line 162. State which transitive Windows crates are allowed.

### 🟡 implementation-readiness-1-8: CI build order breaks the Tauri build, and the Linux frontend job is undefined - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #6 and #7)
* **Affected Scope:** `862-863`, `1258`, `1265`, `1298`, `1335-1336`, `1352`
* **Discrepancy Description:**
  * `cargo build --workspace` runs before `npm run build`, but Tauri's `generate_context!` checks `frontendDist` at compile time.
  * ts-rs writes types during `cargo test`, then `gen:types` copies them. Whether `src/generated/protocol.ts` is committed is undefined.
  * "Frontend-Tests gegen den Fake-Daemon" (863) is impossible as written: the vitest tests mock `ipc.ts`, and the real client path (`ipc_client.rs`) is Windows-only.
* **Rationale & Recommendation:** Use this order: fetch FFmpeg, run cargo test (generates the types), run gen:types, run npm ci/build/test, then cargo build. Decide whether the generated types are committed, and add a diff check if so. For the Linux job, define a real harness (for example a Rust protocol client test against the fake daemon over a Unix socket, with its socket path) or drop the claim.

### 🟡 implementation-readiness-1-9: The Rust side of `gameclip-ui` calls Windows directly - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #18)
* **Affected Scope:** `1252` (rule: 22, 461)
* **Discrepancy Description:** ipc_client.rs uses a Named Pipe, `IApplicationActivationManager`, `GetNamedPipeServerProcessId` and `StartupTask.RequestEnableAsync` inside the Tauri app. Lines 22 and 461 say the UI's Rust side may use Windows only through gameclip-ui-platform, and the `UiPlatform` trait (1231) has none of these methods.
* **Rationale & Recommendation:** Move these calls behind `UiPlatform` (or a second trait), or record an explicit exception. Name the pipe client library and the StartupTask `TaskId`.

### 🟡 implementation-readiness-1-10: `clip://` origin, CSP and capabilities block playback and events - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #17)
* **Affected Scope:** `1245-1246`, `1253`, `1263`
* **Discrepancy Description:** On Windows/WebView2, Tauri 2 serves custom schemes as `http://clip.localhost/...`, not `clip://`. A CSP that allows only `self` and `clip:` therefore blocks `<video>`. IPC needs `ipc: http://ipc.localhost` in connect-src. `listen()` needs `core:event:default`, but "nur eigene Commands + Drag" in the capabilities blocks all events. The request route and the error responses are not specified.
* **Rationale & Recommendation:** Write out the exact CSP string (for example `media-src 'self' http://clip.localhost`) and the permission list in `capabilities/default.json`. Define the route and the responses: 404 for an unknown id, 416 for an invalid range.

### 🟡 implementation-readiness-1-11: The custom MP4 tag `gameclip_id` is dropped without `use_metadata_tags` - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #13)
* **Affected Scope:** `1028-1030`, `1045`, `1168` (contract: 899)
* **Discrepancy Description:** FFmpeg's mov/mp4 muxer writes non-standard keys only with `movflags +use_metadata_tags`. The plan never mentions this, so ownership checks, adoption, the watcher and index rebuild all fail. The index rebuild (309) reads created_at and duration_ms "aus den Tags", but only gameclip_id is defined. Whether the .m4a carries the ID is unclear. Track titles may only round-trip reliably via `handler_name`.
* **Rationale & Recommendation:** In mux.rs and docs/clip-file-format.md, specify the movflags (+use_metadata_tags, plus +faststart for exports) and the full tag set for MP4 and M4A. Add a read-back test with ffprobe.

### 🟡 implementation-readiness-1-12: Index schema lacks columns, and the star backup has no owner - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #21)
* **Affected Scope:** `1154-1156`, `1162`
* **Discrepancy Description:** The watcher stores a "fehlt seit" timestamp, but the clips table (285-299) has no such column. There is no DDL: types, NOT NULL, a unique path, the created_at format and indices are all missing. The migration runner is unspecified (user_version pragma or a meta table). No file writes or reads `.gameclip-stars.json` (302, 307, 309).
* **Rationale & Recommendation:** Put the full DDL of 001_init.sql in the plan, including `missing_since`. Assign the star backup to index.rs and state when it is written and read.

### 🟡 implementation-readiness-1-13: Race between saving or exporting and the watcher - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1143-1144`, `1163`, `1177`
* **Discrepancy Description:** save.rs renames the .part file first and inserts into the index afterwards, and exports do the same. In between, ReadDirectoryChangesW can see the tagged file before its index row exists. The service then marks its own clip as `foreign`, excludes it from the limit and offers "Übernehmen".
* **Rationale & Recommendation:** Insert the row as pending before the rename and switch it to ok afterwards, or have the watcher ignore IDs that are in flight.

### 🟡 implementation-readiness-1-14: Saving has no timeout, partial-failure policy or error codes, and the step range is wrong - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-style` #2)
* **Affected Scope:** `1140-1145`
* **Discrepancy Description:**
  * Step 2 (wait for the packet at the key-press time) has no timeout.
  * There is no policy for MP4 succeeded but M4A failed, or the reverse.
  * Deleting the .part files and the SaveFailed reasons (step 8, line 269) are missing.
  * The dedupe call has no place in the sequence.
  * "Schritte 1–7" is wrong: the flow has 8 steps, and the bullets cover step 8. Step 5 is appended out of order with "Schreibt zusätzlich".
* **Rationale & Recommendation:** Set a wait timeout (for example 500 ms) and a policy for each partial failure (for example keep the MP4 and set tracks_path=NULL). List the SaveFailed codes. Change the text to "Schritte 1–8" and order the bullets by step.

### 🟡 implementation-readiness-1-15: Cloud placeholders are not handled in retention, the executor or the watcher - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #21)
* **Affected Scope:** `955-957`, `1162-1164`, `1168`
* **Discrepancy Description:** Line 279 says placeholders are never read or deleted automatically but still count toward the limit. ClipInfo has no placeholder flag. retention_exec re-reads the tag before deleting and the watcher reads the tags of new files, and either read triggers a download.
* **Rationale & Recommendation:** Add `is_placeholder` to ClipInfo, skip placeholders in plan_deletions, and check the file attributes before any tag read. Add tests for these cases.

### 🟡 implementation-readiness-1-16: Shared-folder limit and ClipInfo are undefined - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #21)
* **Affected Scope:** `955-957`, `972`
* **Discrepancy Description:** When the clip and export folders are the same, the plan does not say which limit applies if the two settings differ. The fields of ClipInfo are not listed. It is not stated whether retention also runs when a limit is lowered or a folder changes.
* **Rationale & Recommendation:** Define the ClipInfo struct, the rule for choosing the shared limit, and the events that trigger retention.

### 🟡 implementation-readiness-1-17: Folder change leaves existing clips undefined - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1161-1164`, `1200`
* **Discrepancy Description:** When the user changes a folder, the plan does not say what happens to index rows in the old folder. Under the current rules they become `missing`, and their single-track files are deleted after 24 h.
* **Rationale & Recommendation:** Define the behaviour, for example keep the rows with an "old folder" flag and exempt them from the 24 h cleanup.

### 🟡 implementation-readiness-1-18: The ring buffer contradicts the reserve-exhaustion rule - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #15)
* **Affected Scope:** `1107-1110`, `1115` (earlier: 251)
* **Discrepancy Description:** Line 251 says that when the pool is full, video is dropped up to the next keyframe, an IDR is requested, audio is dropped for the same period, and the gap is marked. Item 41 says only "neue Pakete werden verworfen und gezählt". These are missing:
  * an IDR-request interface to the encoder
  * a gap marker
  * tests for gaps
  * a rule for packets larger than the free space in a 2 MB block
  * the QPC/PTS mapping used in snapshot()
* **Rationale & Recommendation:** Copy the rule from line 251 into ring.rs, video_pipeline.rs and ring_tests.rs. Define the force-keyframe flag, the gap marker in PacketRef and the block-spanning rule.

### 🟡 implementation-readiness-1-19: Audio track layout when the mic is off or Discord is absent - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` Nit 1)
* **Affected Scope:** `901`, `947`, `1091`, `1095`
* **Discrepancy Description:** forecast fixes n_einzelspuren at 2 or 3 and ignores "Mikrofon aus" (as opposed to paused). The plan does not say whether the .m4a or a SeparateTracks export then contains a mic track, or a Discord track when Discord is not running.
* **Rationale & Recommendation:** Define a track-presence rule, for example "a track exists if its source is enabled, and is silent if the source is not running". Apply it consistently in forecast, mux and the file format, and add tests.

### 🟡 implementation-readiness-1-20: Quality tiers are incomplete, and the forecast signature is inconsistent - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `773-776`, `946-948`
* **Discrepancy Description:** There are no bitrates for 1440p30 or 2160p30. `forecast(settings)` needs the physical RAM, but the signature does not take it. Forecast has no reserve field, and the value compared against the 10 % and 25 % thresholds is not named.
* **Rationale & Recommendation:** Complete the bitrate table. Use `forecast(&Settings, physical_ram_bytes) -> Forecast{buffer_bytes, reserve_bytes, total_bytes, level}` and state that total_bytes is the compared value.

### 🟡 implementation-readiness-1-21: Forecast and size-export tests have no concrete values, and one cannot run - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #20)
* **Affected Scope:** `973`, `1044`, `1220-1224`
* **Discrepancy Description:** forecast_tests has a "Tabelle erwarteter MB" but gives no values. The size test needs 20 durations up to 300 s, but the synthetic source (item 31) is only 10 s long. The encoding of "Bilder durchnummeriert" is not defined. Synthetic test patterns compress trivially, so the test says little about real size accuracy.
* **Rationale & Recommendation:** Put the expected rows in the plan. Give the generator a duration parameter. Define the frame-number encoding, for example a binary pixel pattern. Use more realistic test content.

### 🟡 implementation-readiness-1-22: The size-export ladder is incomplete, and the warning is computed in the wrong process - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #20 and `egm-style` #6)
* **Affected Scope:** `1215`, `1224`, `1286` (earlier: 337)
* **Discrepancy Description:**
  * The 30 fps rule from line 337 ("ab 60 s auch 30 fps") is missing.
  * The ladder order, the starting step for 1440p and 2160p sources, maxrate and the container reserve are missing.
  * The end rule is undefined. At 10 MB, 480p30 drops below 0,05 bpp for clips longer than about 103 s, while test 1224 expects every duration up to 300 s to succeed.
  * The 0,03 warning is to be shown before the job starts (337), but the formula lives in the worker, which only runs after StartExportJob.
  * The sentence at line 1215 packs all of this into one line.
* **Rationale & Recommendation:** Give the ladder as a table with every constant and the end rule. Move the formula into gameclip-core, shared with the UI, or add a `GetExportEstimate` command. Split line 1215 into sub-bullets.

### 🟡 implementation-readiness-1-23: The worker contract lacks the output ID, schema location, GPU priority and error codes - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #21)
* **Affected Scope:** `1172-1177`, `1213-1215`
* **Discrepancy Description:**
  * Exports need a new gameclip_id (339), but the job input (319) has no ID field.
  * The crate that holds the job JSON structs is not named.
  * `SetGPUThreadPriority` (328) is missing.
  * The `{"encoder": ...}` line and the "Export über CPU" notice have no IPC event; JobProgress carries only a percent.
  * The worker's error codes and its cancel path are not defined.
* **Rationale & Recommendation:** Add `output_id`, define the structs in a named crate, place the GPU priority call in a worker file, extend JobProgress or add a new event, and list the error codes.

### 🟡 implementation-readiness-1-24: Encoder presets and options are inconsistent and partly unsuitable - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #12)
* **Affected Scope:** `1016-1017`, `1214` (earlier: 332)
* **Discrepancy Description:**
  * encoder.rs sets libx264 to `superfast` + `zerolatency` for every caller, while the worker fallback at line 332 uses `veryfast`.
  * `zerolatency` disables lookahead and mbtree, which hurts quality at a fixed target size.
  * NVENC needs `forced-idr=1` for on-demand IDR (ring buffer, line 251).
  * Whether QSV `look_ahead 0` is available in libvpl builds is doubtful, and no QSV rate-control mode is set.
  * The GPU-to-CPU download step on the worker's x264 path is unspecified.
* **Rationale & Recommendation:** Define separate capture and export profiles, with the preset passed per caller. Add `forced-idr`, verify the QSV options against the pinned FFmpeg, and specify the download step. (codebase-details confidence: medium-high for forced-idr, low-medium for QSV.)

### 🟡 implementation-readiness-1-25: The scale-fallback callback has no interface - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1037`
* **Discrepancy Description:** The fallback to the D3D11 VideoProcessor "als Callback" has no signature, no texture ownership rule and no crate dependency. As written, the worker would have to depend on capture-win.
* **Rationale & Recommendation:** Define the trait or fn signature (in/out textures, sizes, color space) and the dependency. Alternatively, move convert.rs into a crate both sides can use. See also codebase-details-1-2 (`scale_d3d11` and the FFmpeg version).

### 🟡 implementation-readiness-1-26: Hotkey encoding, TestRegister semantics and resume on UI exit - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1181`, `1287`
* **Discrepancy Description:** No location is named for the mapping from `KeyboardEvent.code` to VK and modifiers. It is unclear whether a successful TestRegister commits the key or whether SetSettings is also required. Nothing says what happens to a suspended hotkey when the UI crashes during the picker. The reply channel through PostThreadMessage is undefined.
* **Rationale & Recommendation:** Define the key type (for example `{code, ctrl, alt, shift}`) and the VK table in core. State the commit sequence, and resume the hotkey when the IPC client disconnects.

### 🟡 implementation-readiness-1-27: The tray icon set and its priority are inconsistent - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1185`, `1326`
* **Discrepancy Description:** There are six icons but no priority when several conditions apply, and no icons for Startet, Konfiguriert neu, Erholt sich or Leerlauf. `mic_paused` is a Restriction (920), which by line 375 already shows as "Eingeschränkt".
* **Rationale & Recommendation:** Give the selection rule as an ordered list over (state, restrictions, saves_in_flight), and resolve how mic_paused is shown.

### 🟡 implementation-readiness-1-28: State-machine Event/Effect enums and uncovered combinations - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `920-923`, `971`
* **Discrepancy Description:** The Event and Effect variants are not listed. The transition table (379-401) has no rows for:
  * a hotkey press in Leerlauf or Fehler
  * device loss in Startet, Konfiguriert neu or Pausiert
  * what "laufende Speichervorgänge beenden" means (finish or abort)
  * several start results at once

  The table also gives 5 attempts but only 4 back-off delays. "Jede Zeile" tests therefore leave behaviour undefined.
* **Rationale & Recommendation:** List the enums, add a default rule for unlisted pairs (for example "ignore + log"), and settle the missing rows.

### 🟡 implementation-readiness-1-29: Required behaviours have no implementing file - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #21 and toast activation)
* **Affected Scope:** `1075`, `1193`, `1252`, `1277`, `1312`
* **Discrepancy Description:** No file or message is named for:
  * HDR detection (for example `IDXGIOutput6::GetDesc1` ColorSpace)
  * Media Feature Pack detection
  * the mic level meter in FirstRun
  * autostart toggling from Settings
  * the StartupTask TaskId
  * the toast button "Im Ordner anzeigen"

  The toast button needs a COM activator (`desktop:ToastNotificationActivation` + `com:` server) or protocol activation. Without one, it relaunches gameclipd, which then hits the mutex. The windows-rs method is `CreateToastNotifierWithId`. Whether toasts work at all with `AppListEntry="none"` should be checked (low confidence).
* **Rationale & Recommendation:** Assign each behaviour to a file and an IPC message, and add the activation entries to the manifest (item 68).

### 🟡 implementation-readiness-1-30: No automated gameclipd tests, and branch coverage needs nightly - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` and `egm-style`)
* **Affected Scope:** `1332-1366`, `1347-1349` (also 829, 862-863, 807)
* **Discrepancy Description:**
  * These have no tests: retention_exec, the watcher, migrations, save.rs error paths, jobs.rs timeout and cancel, the pipe ACL and path validation, and the hotkey.
  * `cargo llvm-cov --branch` needs nightly, but line 829 pins stable.
  * CI runs neither coverage nor `cargo fmt --check`.
  * Milestones are not mapped to file items.
  * M0 needs S9 (worker, M3) and S4 (mixer).
* **Rationale & Recommendation:** Add gameclipd tests that use temp folders and a fake pipeline. Choose a nightly coverage job or line coverage. Add fmt and coverage to CI. Map each file to a milestone.

### 🟡 implementation-readiness-1-31: `panic=abort` cannot be set for gameclipd alone - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (Nit) and `egm-codebase-details` (#3, Major)
* **Affected Scope:** `823`
* **Discrepancy Description:** Cargo forbids `panic`, `lto` and `rpath` in `[profile.release.package.X]`, so the setting applies to every binary in the workspace. Confidence: high.
* **Rationale & Recommendation:** Accept the setting workspace-wide with a written reason, use a custom profile, or drop "für gameclipd".

### 🟡 implementation-readiness-1-32: tracing-appender cannot enforce "je 10 MB" - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` #19 and `egm-style` #7)
* **Affected Scope:** `1205` (earlier: 414)
* **Discrepancy Description:** `RollingFileAppender` rotates by time and supports `max_log_files`, but it has no size limit. Confidence: high.
* **Rationale & Recommendation:** Drop the 10 MB cap here and at line 414, or pick a crate with size-based rotation (for example `file-rotate`) and add it to the dependency list.

### 🟡 implementation-readiness-1-33: The FFmpeg linking mechanism is wrong and contradictory - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (Nit) and `egm-codebase-details` (#9, Major; co-reported by `egm-style` #20)
* **Affected Scope:** `1003`, `1322` (also 1045, 1351)
* **Discrepancy Description:**
  * A crate's build.rs cannot set environment variables for a dependency's build script (confidence: high).
  * rusty_ffmpeg does not read `FFMPEG_DIR`. It reads `FFMPEG_INCLUDE_DIR` + `FFMPEG_DLL_PATH`, or `FFMPEG_LIBS_DIR`/`FFMPEG_PKG_CONFIG_PATH` (confidence: medium).
  * ffmpeg.lock holds a URL and SHA, not a directory, so line 1003 contradicts line 1322.
  * The DLLs and ffprobe are not on PATH during tests.
  * "ohne in den Projektbaum zu committen" is unclear, because `third_party/ffmpeg/` is inside the project tree.
* **Rationale & Recommendation:** Set the variables in `.cargo/config.toml` `[env]` or in the CI step. Name the variables and the rsmpeg feature exactly. Write "(per .gitignore ausgeschlossen)".

### 🟡 implementation-readiness-1-34: Mutex and `--save-clip` are in the wrong order - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (Nit) and `egm-codebase-details` (Major)
* **Affected Scope:** `1122`, `1126`
* **Discrepancy Description:** The bullets take the mutex before the CLI branch. Followed literally, a `--save-clip` call fails while the service is running. The behaviour of a second instance (exit code, silent exit) is not stated.
* **Rationale & Recommendation:** Check for `--save-clip` first, then take the mutex. Define how a second instance exits.

### 🟡 implementation-readiness-1-35: Release, signing and licence tooling is not configured - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness` (Nit) and `egm-codebase-details` (Major)
* **Affected Scope:** `855`, `869`, `1318`, `1337`
* **Discrepancy Description:** SignPath is a remote signing service used through its GitHub action. It does not plug into a local SignTool call inside build-msix.ps1. `-TestCert` (1337) is not defined in item 69. The cargo-about config and template, the npm licence tool and the SignPath parameters are all missing.
* **Rationale & Recommendation:** Split packing and signing into separate steps, define `-TestCert` (create or import the certificate, Trusted People install), and list the configuration files.

### 🟡 codebase-details-1-1: cargo-deny licence allow-list is incomplete and not valid SPDX - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `836`
* **Discrepancy Description:** clipboard-win is BSL-1.0 (confidence: medium-high), which is not on the list. "BSD-2/3" and "GPL-3.0" are not valid current SPDX identifiers, and the project's own GPL-3.0-or-later crates must also be allowed.
* **Rationale & Recommendation:** Write out the exact `[licenses] allow` array with `BSD-2-Clause`, `BSD-3-Clause`, `GPL-3.0-or-later` and `BSL-1.0`, then check it with `cargo deny list` once the dependencies are fixed.

### 🟡 codebase-details-1-2: BtbN pinning is fragile, and the FFmpeg 7.1 pin conflicts with features the plan uses - Severity: Major
* **Reviewer Lens:** `egm-codebase-details` (co-reported by `egm-implementation-readiness` N3)
* **Affected Scope:** `1015`, `1037`, `1322`
* **Discrepancy Description:** BtbN overwrites and prunes its release assets, so a SHA-pinned URL will break (confidence: medium-high). As far as codebase-details knows, `scale_d3d11` exists only from FFmpeg 8.x (confidence: medium), so the "falls vorhanden" branch is dead code under 7.1. D3D11 input to `h264_mf` is also a newer addition.
* **Rationale & Recommendation:** Mirror the archive as a release of the project's own repository (which also satisfies the source offer), or build FFmpeg in CI. Choose 7.1 or 8.x explicitly.

### 🟡 codebase-details-1-3: NV12 frame pool lacks the BindFlags the VideoProcessor output needs - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1007`, `1071`
* **Discrepancy Description:** `CreateVideoProcessorOutputView` requires `D3D11_BIND_RENDER_TARGET`, so `AVD3D11VAFramesContext.BindFlags` must be set explicitly. With texture arrays, the view needs `D3D11_VPOV_DIMENSION_TEXTURE2DARRAY` plus the slice index. Confidence: medium-high.
* **Rationale & Recommendation:** Specify BindFlags, MiscFlags, and array vs. single textures. Check the pool size of 8 against the number of frames the encoder holds in flight.

### 🟡 codebase-details-1-4: `FlushFileBuffers` is not reachable through FFmpeg's file protocol, and `.part` hides the format - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1028-1029`
* **Discrepancy Description:** libavformat's `file:` protocol owns the file handle and never fsyncs, and the `+faststart` pass reopens the file by name. Confidence: high. With the `.part` extension, `avformat_alloc_output_context2` cannot guess the container format.
* **Rationale & Recommendation:** Use a custom AVIOContext, or reopen the file after `av_write_trailer` and call `sync_all`. Pass `mp4`/`ipod` as the format explicitly.

### 🟡 codebase-details-1-5: Protocol serde tagging contradicts the envelope - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `983` (envelope: 355-357), `1258`, `1265`
* **Discrepancy Description:** `#[serde(tag = "type")]` puts the fields next to `type`, but the envelope expects `{"id","type","body"}`, and events use the key `event`. Confidence: high. These are also undefined:
  * how unknown commands from newer clients are handled
  * how `u64` ids are typed: ts-rs emits `bigint`, but JSON.parse produces `number`
  * how ts-rs export is configured: `#[ts(export)]`, `TS_RS_EXPORT_DIR`, and one file vs. many
* **Rationale & Recommendation:** Use `tag = "type", content = "body"` for commands and `tag = "event", content = "body"` for events, with `id` in a wrapper struct. Specify `#[ts(type = "number")]` and the export configuration.

### 🟡 codebase-details-1-6: Watcher overflow and the service's own file events - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1154-1168`
* **Discrepancy Description:** There is no handling for a `ReadDirectoryChangesW` overflow (`ERROR_NOTIFY_ENUM_DIR`), which requires a full rescan. The service's own `.part` writes and rename events are not ignored either.
* **Rationale & Recommendation:** Specify a rescan on overflow and a filter for the service's own operations. See also implementation-readiness-1-13.

### 🟡 codebase-details-1-7: The tray has no message-pump thread - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1124`, `1181`, `1185`
* **Discrepancy Description:** tray-icon on Windows needs a thread with a Win32 message loop. The main loop is a blocking crossbeam receiver, so the thread that hosts the tray is undecided. Confidence: high.
* **Rationale & Recommendation:** Name the thread, for example the existing message-only hotkey thread.

### 🟡 codebase-details-1-8: MSIX manifest capabilities are in the wrong elements and namespaces - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1312-1314`
* **Discrepancy Description:**
  * `runFullTrust` is a `rescap:Capability`.
  * `microphone` is a `DeviceCapability`.
  * The graphicsCapture capabilities are probably rescap too (confidence: medium).
  * `windows.startupTask` needs `uap5`, TaskId, Executable and DisplayName.
  * `TargetDeviceFamily` needs `MaxVersionTested`.
* **Rationale & Recommendation:** Write out the manifest fragments with their namespaces.

### 🟡 egm-style-1-1: German and non-ASCII identifiers break the naming rule at line 359 - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `930`, `946`, `948`, `955`, `963-965`, `1014`, `1018`, `1028`, `1109`, `1231`
* **Discrepancy Description:** The plan uses German identifiers such as `Feldfehler`, `puffer_bytes`, `gesamt_bytes`, `stufe`, `Ok/Warnung/Abgelehnt`, `loeschen`, `EncoderKandidat`, `pfad`, `video_pakete`, `von_qpc` and `spur`. Two are non-ASCII: `auflösung` (1014) and `größe` (1231). They appear mixed with English names in the same signatures.
* **Rationale & Recommendation:** Use English ASCII names throughout (`FieldError`, `buffer_bytes`, `level: Ok|Warning|Rejected`, `resolution`, `size` ...), or narrow the rule at line 359 explicitly.

### 🟡 egm-style-1-2: "vollständig getestet" overstates the coverage target - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `958` (compare 1347)
* **Discrepancy Description:** "vollständig" cannot be checked and conflicts with the 90 % branch-coverage target. Action: CALIBRATE.
* **Rationale & Recommendation:** Name the test cases and give a target for retention.rs (for example 100 % branch coverage), or delete "vollständig".

### 🟡 egm-style-1-3: Unverified claim about `GetMixFormat` stated as fact - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1082`
* **Discrepancy Description:** "weil `GetMixFormat` hier nicht verfügbar ist" is stated as fact, but the brief (line 101) marks it "(prüfen)". Action: CALIBRATE.
* **Rationale & Recommendation:** Write "laut Brief zu prüfen, S4".

---

### 🟢 implementation-readiness-1-36: Naming rules are imprecise - Severity: Nit
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` Nit 13)
* **Affected Scope:** `963-965`
* **Discrepancy Description:** It is unclear whether the 64-character limit covers only the program part or the whole name. The invalid characters, reserved names (CON, NUL) and trailing dots or spaces are not listed. The suffixes for SeparateTracks exports and non-integer MB targets are not defined.
* **Rationale & Recommendation:** Specify each of these exactly.

### 🟢 implementation-readiness-1-37: "Das Repository ist leer" is false, and item 9 is not a Create - Severity: Nit
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-codebase-details` Nit 10 and `egm-style` #10)
* **Affected Scope:** `813`, `874-877`
* **Discrepancy Description:** concepts_brief.md and this design document already exist.
* **Rationale & Recommendation:** Mark item 9 as Modify, or write "Außer diesem Dokument ...". Delete "in der endgültigen Fassung".

### 🟢 implementation-readiness-1-38: Pipe details are incomplete - Severity: Nit
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** `1198-1199`
* **Discrepancy Description:** The exact SDDL string, the behaviour for a 5th client, and the check for "gameclipd.exe aus dem eigenen Paket" (for example the image path vs. InstalledLocation) are not given.
* **Rationale & Recommendation:** Specify them.

### 🟢 implementation-readiness-1-39: Gallery details are open - Severity: Nit
* **Reviewer Lens:** `egm-implementation-readiness` (co-reported by `egm-style` #21)
* **Affected Scope:** `1278`, `1302`
* **Discrepancy Description:**
  * Whether manual delete is permanent or uses the recycle bin, and whether it applies to foreign files, is undefined.
  * The plan does not say whether untagged videos are shown or where the used-space sum comes from.
  * The test wording "foreign ohne Löschoption für das Limit" is ambiguous.
  * There is no command to fetch the job list after reconnecting (needed for R11).
* **Rationale & Recommendation:** Define these, and rephrase the test, for example "Fremd-Clips werden markiert und nicht in die Platzanzeige eingerechnet".

### 🟢 codebase-details-1-9: Forecast example does not match the earlier figure - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `947` (earlier: 255)
* **Discrepancy Description:** With the defaults, the formula gives about 250 MB (116 + 54 reserve + 80), not "ca. 210 MB".
* **Rationale & Recommendation:** Reconcile the formula and the example.

### 🟢 codebase-details-1-10: The AAC encoder needs FLTP input with 1024-sample frames - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1022`
* **Discrepancy Description:** The mixer delivers interleaved 10 ms blocks (480 samples).
* **Rationale & Recommendation:** Add an AVAudioFifo, deinterleaving and re-chunking to 1024 samples.

### 🟢 codebase-details-1-11: Loopback time base and initialization flags - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `822`, `1062`, `1080`, `1083`
* **Discrepancy Description:** `pu64QPCPosition` is in 100 ns units and `SystemRelativeTime` is a TimeSpan. The flags `LOOPBACK|EVENTCALLBACK|AUTOCONVERTPCM` are missing. The plan uses both the `wasapi` crate and raw `ActivateAudioInterfaceAsync`, and the two can bring incompatible `windows` versions.
* **Rationale & Recommendation:** Name one time base and one API path.

### 🟢 codebase-details-1-12: The pacer needs absolute due times - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1067`
* **Discrepancy Description:** `SetWaitableTimer` takes its period in whole milliseconds, so 16.667 ms drifts.
* **Rationale & Recommendation:** Use one-shot absolute due times derived from QPC.

### 🟢 codebase-details-1-13: CLI exit codes and launching from WindowsApps - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1133-1134`
* **Discrepancy Description:** Shells do not wait for a `windows_subsystem="windows"` exe, so exit codes are only visible with `start /wait` or `-Wait`. Launching the exe directly from WindowsApps runs it without package identity (confidence: low-medium).
* **Rationale & Recommendation:** Document this, or use an App Execution Alias.

### 🟢 codebase-details-1-14: Drag is exposed twice - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1233`, `1246`, `1254`
* **Discrepancy Description:** tauri-plugin-drag is a Tauri plugin, but gameclip-ui-platform is meant to be a plain crate.
* **Rationale & Recommendation:** Use the `drag` crate in ui-platform, or keep the plugin only in the app.

### 🟢 codebase-details-1-15: "wenn verfügbar" for MinUpdateInterval is redundant - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `1061`, `1314`
* **Discrepancy Description:** MinVersion 26100 already guarantees the API, so the restricted mode below the minimum build from the brief can no longer occur.
* **Rationale & Recommendation:** Delete "wenn verfügbar" and record that the restricted mode is gone.

### 🟢 codebase-details-1-16: The 8 MB message limit has no byte value - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `984`
* **Discrepancy Description:** Under the decimal glossary (line 44), 8 MB is 8 000 000 bytes, not 8 MiB.
* **Rationale & Recommendation:** State the value in bytes.

### 🟢 codebase-details-1-17: Clippy and fmt calls in CI are incomplete - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `862`
* **Discrepancy Description:** The clippy call lacks `--workspace --all-targets` (compare line 1342). `cargo fmt --check` and `npm run typecheck` are missing.
* **Rationale & Recommendation:** Align CI with the Verification Plan.

### 🟢 codebase-details-1-18: Small undefined items - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** `937`, `1015`, `1053-1059`, `1174`
* **Discrepancy Description:** These are not specified:
  * the name of the `h264_mf` Cargo feature
  * the `vN_to_vN+1` naming pattern
  * the `CreateDirect3D11DeviceFromDXGIDevice` wrapping for WGC
  * `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`

  Also, gameclip-fake-daemon is missing from the crate table (453-462).
* **Rationale & Recommendation:** Add these details.

### 🟢 codebase-details-1-19: Minimum buffer of 15 s contradicts "1 bis 5 Minuten" - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details` (co-reported by `egm-style` #15)
* **Affected Scope:** `770` (earlier: 12)
* **Discrepancy Description:** Line 770 allows 15-300 s, but the overview and the brief say 1-5 minutes.
* **Rationale & Recommendation:** Align one of the two places.

### 🟢 egm-style-1-4: Mixed German and English labels - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `811`, `819-824`, `1332-1353`
* **Discrepancy Description:** "Warum" sits next to "Type / Scope", "Proposed Changes" and "Verification Plan".
* **Rationale & Recommendation:** Pick one language for the labels.

### 🟢 egm-style-1-5: Uneven file-entry template - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `874-877`, `907-914`, `934`, `960-965`, `989-991`, `1001-1003`, `1049-1101`, `1119-1206`
* **Discrepancy Description:** Many entries lack "Type / Scope" or "Warum", and item 9 lacks "Proposed Changes".
* **Rationale & Recommendation:** Make all fields required, or state once when "Warum" may be omitted.

### 🟢 egm-style-1-6: Cross-references use translated or vague section names - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `923`, `947`, `983`, `1140`, `1154`, `1185`, `1206`
* **Discrepancy Description:** "aus dem Technischen Plan" does not match the heading "Technical Plan (Architecture)", and the subsections are often not named.
* **Rationale & Recommendation:** Name the exact subsection each time.

### 🟢 egm-style-1-7: "Zustände" used for things that are not states - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1326` (with 1185)
* **Discrepancy Description:** Eingeschränkt and Mikrofon pausiert are restrictions, and Speichert is a counter.
* **Rationale & Recommendation:** Write "für die sechs Tray-Symbole (siehe Datei 51)".

### 🟢 egm-style-1-8: Units and constants in the forecast formula - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `947` (with 792)
* **Discrepancy Description:** "128k" and "160k" have no unit. The 80 MB "Grundbedarf" reuses the RAM measurement target.
* **Rationale & Recommendation:** Write "128 kbit/s" and "160 kbit/s", and give the constant its own name or link it to the target explicitly.

### 🟢 egm-style-1-9: MB is not defined, and the boundary wording differs - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `789`, `1224`, `1365`
* **Discrepancy Description:** Line 316 uses 10 000 000 bytes. Line 1224 says "unter 10 MB" and line 1365 says "keiner über der Zielgröße", which differ at the boundary.
* **Rationale & Recommendation:** Define MB = 10^6 bytes and use "höchstens Zielgröße" in both places.

### 🟢 egm-style-1-10: G1 criterion lost its threshold - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1366`
* **Discrepancy Description:** The definition from line 69 is missing. Action: CALIBRATE.
* **Rationale & Recommendation:** Add "(≥ 10 % Abstand, vorläufig)".

### 🟢 egm-style-1-11: Unclear what "(vorläufig)" applies to - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1357`
* **Discrepancy Description:** At the end of the S1 row, it is unclear whether the marker covers only the RAM value or every threshold.
* **Rationale & Recommendation:** Put the marker after each provisional value.

### 🟢 egm-style-1-12: Ambiguous path rule - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1200`
* **Discrepancy Description:** Read literally, "nicht unter dem Laufwerksstamm" excludes every path on the drive.
* **Rationale & Recommendation:** Write "nicht der Laufwerksstamm selbst (z. B. `D:\`)".

### 🟢 egm-style-1-13: Vague licence claim for the sounds - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1189`
* **Discrepancy Description:** "freie Lizenz" does not name the licence. Action: CALIBRATE.
* **Rationale & Recommendation:** Name it, for example CC0-1.0.

### 🟢 egm-style-1-14: Vague wording in the encoder entry - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1017`
* **Discrepancy Description:** "die passende Zuordnung" is vague. For lines 1003 and 1322, see implementation-readiness-1-33.
* **Rationale & Recommendation:** Write "encoderspezifische Optionen" and list them.

### 🟢 egm-style-1-15: Wording drift and an unclear test target - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** `1135`, `1222`, `1277`
* **Discrepancy Description:**
  * Line 1277 says "standardmäßig" while line 442 says "dauerhaft aufgenommen". The two mean different things in a privacy notice.
  * Line 1135 says "nur als intern erwähnt", which conflicts with line 100 ("ohne Dokumentation").
  * Line 1222 says "Audio-Länge ±1 ms" without saying compared to what.
* **Rationale & Recommendation:** Use one wording in each case, and write "entspricht der Videolänge auf ±1 ms".
