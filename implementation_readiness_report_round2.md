# Goldfish Review Report: Stage 9 Re-Audit (Implementation Readiness, Round 2)

**Target:** `D:\ProgrammierProjekte\GameClip\docs\designs\gameclip.md` (lines 787-1435, "## Detailed Implementation Plan" to the end)
**Mode:** REVIEW. This is the single allowed re-audit after fixes.
**Inputs:** previous report `implementation_readiness_report.md`, resolution log `goldfish_review_report_stage8_9.md`, GoldfishContext `concepts_brief.md`.
**Excluded by instruction:** the author decisions T4 (custom worker), T6 (on-demand index reconcile plus a gallery-only watcher), T7 (frontend mock instead of a fake daemon) and T8 (custom `clip://` protocol); all DECLINE items; and everything the log lists as "not verifiable by reading". Reviewers confirmed they did not re-raise any of these. cd-2-6 is about the format of the toast activation argument, not about T8.

## Reviewer Status

| Reviewer | Status | Notes |
| --- | --- | --- |
| `egm-implementation-readiness` | completed | Standard: implementation-readiness-standard.md |
| `egm-codebase-details` | completed | Same standard. The persona file is `agents/codebase_details.md` (underscore). |
| `egm-style` | completed | Uses the persona's own rules |

## Verdict

**NOT READY for Phase 5.** All three Blockers from round 1 are fixed, and most of the round-1 Majors are resolved. However, the re-audit found 1 new Blocker (the index primary key) and 19 Majors that are either new or left over from partial fixes. Most of them are local edits of one to a few lines.

The defects that would break the build or lose data if implemented as written are:
- cd-2-1: the index primary key
- cd-2-10: reconcile deletes rows that belong to a previous folder
- cd-2-2: `sync_all` is called on a read-only handle
- cd-2-3: ts-rs type generation is wrong
- cd-2-4: the boundary check fails with an error instead of passing or failing cleanly
- ir-2-1: the CI build steps run in the wrong order
- cd-2-5: Media Feature Pack detection makes the exe fail to load on N editions

**Severity counts (after deduplication):** 🔴 Blocker 1 · 🟡 Major 19 · 🟢 Nit 12

## Previous Blocker/Major → Status

When two reviewers disagreed, the table shows the more conservative status, and the Notes column says so.

| Previous id | Topic | Status | Current lines / follow-up |
| --- | --- | --- | --- |
| implementation-readiness-1-1 | IPC bodies | resolved | 1060-1105. Remaining drift tracked in style-2-1 and cd-2-3 |
| implementation-readiness-1-2 | Settings schema | resolved | 791-842. New contradiction in ir-2-4 |
| implementation-readiness-1-3 | Timeline | resolved | 1084, 1142, 1269-1271, 1369, 1381. Nit ir-2-6 |
| implementation-readiness-1-4 | Export folder | resolved | 833-834 |
| implementation-readiness-1-5 | Manifests and paths | resolved | 855-867, 884-1406 |
| implementation-readiness-1-6 | Versions and features | partly | 889-915. See cd-2-14 |
| implementation-readiness-1-7 | Boundary check | partly | 948. The command fails as written, see cd-2-4 |
| implementation-readiness-1-8 | CI order | partly | 947. Codebase-details: resolved. Implementation-readiness: clippy and test still run before `npm run build`, see ir-2-1 |
| implementation-readiness-1-9 | Windows calls in the UI | resolved | 1321-1327, 1342 |
| implementation-readiness-1-10 | `clip://` serving | resolved | 1335-1336, 1343. New Range issue in cd-2-13 |
| implementation-readiness-1-11 | `use_metadata_tags` | resolved | 977, 1141, 1150 |
| implementation-readiness-1-12 | Index DDL | resolved, with a new defect | 1236-1259. The new primary-key defect is cd-2-1 |
| implementation-readiness-1-13 | Save/watcher race | resolved | 1223, 1259, 1275 |
| implementation-readiness-1-14 | Save steps | resolved | 1218-1227. Numbering Nit in style-2-5 |
| implementation-readiness-1-15 | Placeholders | partly | Production code is fixed (1022-1024, 1259, 1267). The test cannot be built, see cd-2-12 |
| implementation-readiness-1-16 | Shared limit and ClipInfo | partly | 1025, 1267. See cd-2-9 |
| implementation-readiness-1-17 | Folder change | partly | 1024, 1259. See cd-2-10 |
| implementation-readiness-1-18 | Ring buffer | resolved | 1186-1197 |
| implementation-readiness-1-19 | Track rule | resolved | 853, 1009 |
| implementation-readiness-1-20 | Tiers and forecast | resolved | 844-851, 1008-1011 |
| implementation-readiness-1-21 | Test values | resolved | 1044-1045, 1148. Both reviewers recomputed the numbers and found them correct |
| implementation-readiness-1-22 | Size-export ladder | partly | 1016-1017, 1085, 1313. See cd-2-11 |
| implementation-readiness-1-23 | Worker contract | resolved | 1105, 1275, 1311 |
| implementation-readiness-1-24 | Encoder profiles | partly | 1129-1132. Implementation-readiness: resolved. Codebase-details: `forced_idr` is missing for AMF and QSV, see cd-2-7 |
| implementation-readiness-1-25 | Scale callback | partly | Replaced by gameclip-gpu (1118), but the Converter cannot serve the worker, see cd-2-8 |
| implementation-readiness-1-26 | Hotkey | partly | 832, 1037, 1077, 1214. Codebase-details: resolved. Implementation-readiness: a state-table row conflicts with the new rule, see ir-2-3 and ir-2-4 |
| implementation-readiness-1-27 | Tray | resolved | 1283 |
| implementation-readiness-1-28 | State machine | partly | 994-998. Codebase-details: resolved. Implementation-readiness: events are dropped, see ir-2-3 |
| implementation-readiness-1-29 | Behaviours without a file | partly | 1119, 1177, 1287, 1325. See cd-2-5 and cd-2-6 |
| implementation-readiness-1-30 | gameclipd tests and coverage | partly | 947, 1297-1304, 1413. See cd-2-12, ir-2-2 and cd-2-18 |
| implementation-readiness-1-31 | `panic=abort` | resolved | 916 |
| implementation-readiness-1-32 | Log size | resolved | 1295 |
| implementation-readiness-1-33 | FFmpeg linking | resolved | 926-927, 947, 1402 |
| implementation-readiness-1-34 | Mutex order | resolved | 1205 |
| implementation-readiness-1-35 | Release and signing | resolved | 954-955, 1396-1398 |
| codebase-details-1-1 | Licence list | resolved | 931 |
| codebase-details-1-2 | BtbN and FFmpeg version | resolved | 894, 1402 |
| codebase-details-1-3 | BindFlags | partly | 1118, 1127. A conflict with the FFmpeg pool allocation is in ir-2-5 |
| codebase-details-1-4 | Flush and format | partly | The format at 1141 is fixed. The flush call is wrong, see cd-2-2 |
| codebase-details-1-5 | serde and ts-rs | partly | 1060-1062. ts-rs is still broken, see cd-2-3 |
| codebase-details-1-6 | Watcher overflow | resolved | 1259, 1263 |
| codebase-details-1-7 | Message pump | resolved | 1214 |
| codebase-details-1-8 | Manifest | resolved | 1388-1392. Nit cd-2-19 |
| style-1-1 | German and non-ASCII identifiers | resolved | 789, 994-1105 |
| style-1-2 | "vollständig getestet" | resolved | 1026, 947, 1413 |
| style-1-3 | `GetMixFormat` stated as fact | resolved | 1172 |

**Tally (46 previous Blocker/Major items):** 29 resolved, 17 partly resolved, 0 still fully open.

## Prioritized Findings (Blockers -> Major -> Nits)

### 🔴 codebase-details-2-1: The index primary key cannot hold `foreign` duplicates - Severity: Blocker
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness` (implementation-readiness rated it Major)
* **Affected Scope:** Lines 1241-1259, 1080-1083, 1267, 1343. The rules are at 284 and 323.
* **Discrepancy Description**: `clips.id TEXT PRIMARY KEY` is the UUID stored in the file's `gameclip_id` tag, and a copied file carries the same tag. This causes three problems:
  - The rule "doppelte ID → foreign" (1259) needs a second row with the same primary key, so the INSERT fails.
  - Even without the constraint, `ListClips` would return two entries with the same `clip_id`. `SetStar`, `AdoptClip`, `DeleteClip`, `GetFrameTimes`, `StartExportJob` and the `/<clip_id>` route could not tell the original from the copy.
  - `AdoptClip` gives the row a new id but leaves the file's tag unchanged. The next `retention_exec` check (tag ≠ index, 1267) therefore marks the adopted clip `foreign` again.
* **Rationale & Recommendation**:
  - Separate the row key from the file tag, for example with `row_id INTEGER PRIMARY KEY` plus `tag_id TEXT NOT NULL`, with no UNIQUE constraint on `tag_id`.
  - Address clips by row id throughout the protocol.
  - Define ownership as "row has `status='ok'` and the file's tag equals the row's `tag_id`".
  - Decide whether `AdoptClip` rewrites the file's tag (a remux) or records the adopted row as owner despite the duplicate.
  - Update the DDL, `ClipInfo`, `ClipEntry`, the reconcile rules and the tests (1043, 1301).

### 🟡 codebase-details-2-2: `sync_all` on a read-only handle fails - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Line 1141, and the worker at 1312 (rule at 350)
* **Discrepancy Description**: `std::fs::File::open` opens the file read-only. On Windows, `sync_all` calls `FlushFileBuffers`, which requires a handle with GENERIC_WRITE and otherwise fails with ERROR_ACCESS_DENIED. Both reviewers rate their confidence as high.
* **Rationale & Recommendation**: Use `OpenOptions::new().write(true).open(path)?.sync_all()?` in both the service and the worker.

### 🟡 codebase-details-2-3: ts-rs generation is under-specified and partly will not compile - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 902, 907, 984, 1003, 840, 1060-1062, 1080, 1084
* **Discrepancy Description**:
  - (a) ts-rs maps `u64` to `bigint`, but `JSON.parse` yields `number`. This affects `size_bytes`, `used_bytes`, `limit_bytes`, `frames_us`, `duration_us`, `in_us`/`out_us`, `adapter_luid` and the byte limits in settings. The previous fix only changed the ids.
  - (b) `body: Option<Value>` and `partial: Value` need the ts-rs feature `serde-json-impl`, which is not in the feature list.
  - (c) `Settings`, `Forecast`, `ExportPlan`, `Hotkey`, `Resolution`, `Quality` and the clip-status enum live in gameclip-core, which has no ts-rs dependency (984). The orphan rule prevents gameclip-protocol from implementing `TS` for them.
  - (d) `extra: toml::Table` and `time::OffsetDateTime` have no `TS` implementation. Nothing says whether `extra` is sent over IPC at all.
  - (e) `export_to` is resolved relative to `TS_RS_EXPORT_DIR` (default `./bindings`), so `../../apps/…` ends up at `crates/apps/…`. Confidence: medium to medium-high.
  - (f) `Response.body: Option<Value>` leaves every reply untyped in TS.
* **Rationale & Recommendation**:
  - Add `#[ts(type = "number")]` to all `u64` fields, or state a global rule.
  - Add `serde-json-impl` to the ts-rs features.
  - Give gameclip-core an optional `ts` feature and use `cfg_attr(feature = "ts", derive(TS))`.
  - Mark `extra` with `#[serde(skip)]` and `#[ts(skip)]` for IPC, and send `created_at` as an RFC 3339 `String`.
  - Set `TS_RS_EXPORT_DIR` in the `[env]` section of `.cargo/config.toml`.
  - Name the response structs for each command, for example `ListClipsReply`.

### 🟡 codebase-details-2-4: The boundary check `cargo tree -i windows-sys` fails with an error - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Line 948 (also 489)
* **Discrepancy Description**: With tauri, tray-icon, wasapi and `windows` 0.61, `Cargo.lock` will contain several versions of `windows-sys` and `windows`. `cargo tree -i <name>` then stops with "specification is ambiguous", so the check errors instead of printing nothing. Confidence: medium-high.
* **Rationale & Recommendation**: Use `cargo tree -p gameclip-core -p gameclip-protocol -e normal --target x86_64-pc-windows-msvc --prefix none --format "{p}"` and fail if any line matches `^windows(-sys)? `. State that a non-zero exit code also counts as a failure.

### 🟡 codebase-details-2-5: Media Feature Pack detection through `MFStartup` makes the exe unloadable on N editions - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1326, 1361, 1073, 893
* **Discrepancy Description**:
  - windows-rs links `MFStartup` as a normal load-time import of mfplat.dll. On an N or KN edition without the pack, gameclip-ui.exe fails to start, so it can never show the hint. Confidence: high.
  - The BtbN avcodec DLL may also import mfplat, which would stop gameclipd from starting too. Confidence: medium.
  - Ownership is split. Detection lives only in the ui-platform file `media.rs`, but `GetCapabilities.media_feature_pack` is reported by the service, and no gameclipd file computes it.
* **Rationale & Recommendation**:
  - Detect the pack with `LoadLibraryExW("mfplat.dll", LOAD_LIBRARY_SEARCH_SYSTEM32)`.
  - Do not link Media Foundation into any exe unless the import is delay-loaded.
  - Choose one owner for the check.
  - In S2, check the avcodec imports with `dumpbin /dependents`. If they include mfplat, decide on an FFmpeg build without Media Foundation or a check that runs before the FFmpeg DLLs load.

### 🟡 codebase-details-2-6: The toast protocol activation never reaches `--reveal`, and there is no lookup from id to path - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1287, 1341, 1391 (also 512)
* **Discrepancy Description**:
  - A full-trust app with `uap:Protocol Name="gameclip"` and no `Parameters` attribute receives the URI itself (`gameclip:reveal?id=…`) as its argument, not `--reveal <id>`. main.rs never parses the URI form.
  - `SHOpenFolderAndSelectItems` needs a path, but no command returns a path for a given id.
  - Nothing defines what happens when the service is not reachable.
* **Rationale & Recommendation**:
  - Either declare `Parameters="--reveal &quot;%1&quot;"` in the manifest, or parse `gameclip:reveal?id=` directly in main.rs.
  - Define the lookup: connect, Hello, then `ListClips` over both folders, or a new `GetClip{clip_id}` command.
  - Define the fallback when the service is not reachable.

### 🟡 codebase-details-2-7: On-demand keyframes are not IDR frames on AMF and QSV - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Lines 1130, 1132, 1193
* **Discrepancy Description**: Keyframes are requested with `pict_type = AV_PICTURE_TYPE_I`, and only NVENC gets `forced-idr 1`. h264_amf and h264_qsv have a `forced_idr` option that is off by default. Without it they emit non-IDR I-frames, and with multiple reference frames the frames after a gap or snapshot start may not be decodable. Confidence: medium.
* **Rationale & Recommendation**: Set `forced_idr 1` for AMF and QSV in both profiles. The QSV option is verified in S2. State that every snapshot or gap boundary must start on an IDR frame.

### 🟡 codebase-details-2-8: The `Converter` cannot serve the export worker - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Lines 1118, 1143, 1312 (rule at 348)
* **Discrepancy Description**:
  - The `Converter` accepts only BGRA input as a plain `&ID3D11Texture2D`.
  - d3d11va decoding in the worker produces NV12 frames as slices of a texture array, and software decoding produces CPU frames. Neither can be passed in.
  - It is undefined whether `Mix` and `SeparateTracks` go through the Converter, and which `AVHWFramesContext` the export encoder uses.
* **Rationale & Recommendation**:
  - Extend the API with an input format (BGRA or NV12) and an `array_slice: u32` (`D3D11_VPIV_DIMENSION_TEXTURE2D` with ArraySlice).
  - Specify the upload path for software decoding.
  - Specify who owns the frames context in the worker.

### 🟡 codebase-details-2-9: The shared-folder limit is ambiguous - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1023-1025, 1267, 1043 (rule at 288)
* **Discrepancy Description**: `effective_limit` returns a pair `(u64, u64)`, and `plan_deletions` is called once per kind. When clips and exports share a folder, the plan can be read as two separate sums checked against the same limit. That allows twice the limit, while line 288 requires one common sum.
* **Rationale & Recommendation**: State that when `same_folder` is true, `retention_exec` makes one `plan_deletions` call over clips and exports together, with `min(limit)`. Give the expected result in the "gemeinsamer Ordner" test (1043).

### 🟡 codebase-details-2-10: Reconcile deletes rows from the previous folder (data loss) - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1259, 1022, 1043 (rule at 290)
* **Discrepancy Description**:
  - `reconcile(folder_path, folder)` scans only the current folder.
  - Rows whose files are in a previous folder therefore count as "fehlende Datei". After two reconciles they are deleted together with their .m4a.
  - This contradicts the rule that clips in the previous folder stay in the index and are never deleted automatically.
  - `in_current_folder` has no column and no stated derivation.
* **Rationale & Recommendation**:
  - Define "missing" as `!Path::exists(row.path)`, checked per row.
  - The directory scan applies only to rows whose `path` parent equals `folder_path`.
  - Derive `in_current_folder` from the path at query time.

### 🟡 codebase-details-2-11: `export_plan` has no rule for a non-positive budget and no way to predict `TooLarge` - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1016, 1313, 1317, 1427, 836 (formula at 352)
* **Discrepancy Description**:
  - `export.target_bytes` may be as low as 1 000 000. For a 300 s clip, video_bps = (7 360 000 − 28 800 000 − 512 000)/300, which is negative and underflows in `u64`.
  - `ExportPlan` has no field that marks a plan as infeasible, yet the tests at 1317 and 1427 expect `TooLarge` "wenn export_plan das vorhersagt".
* **Rationale & Recommendation**:
  - Compute in i64 or f64.
  - Add `feasible: bool`, or return `Result<ExportPlan, PlanError::TooSmall>`, with a floor such as video_bps < 50 000.
  - Expose the result through `GetExportEstimate` and let the worker fail fast with `TooLarge`.
  - Name the error code and add test rows for this case.

### 🟡 codebase-details-2-12: gameclipd and worker test infrastructure cannot be built as specified - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1297-1304, 1201, 1145-1148, 1317
* **Discrepancy Description**:
  - (a) `SetFileAttributesW` cannot set `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`; only the file system or a cloud-filter provider sets it. The placeholder test is therefore impossible as written. Confidence: high.
  - (b) `trait PacketSource` has no defining file in production code, and save.rs calls `Ring::snapshot` directly.
  - (c) `tests/support/fake_worker.rs` "als Test-Binärdatei" is not a Cargo target, and jobs.rs launches `gameclip-worker.exe` by a fixed name.
  - (d) gameclipd has only `main.rs`, so integration tests cannot import its modules. It needs a `src/lib.rs`.
  - (e) The worker tests reuse `gameclip-media/tests/support/synth.rs` from another crate, but no mechanism for that is stated.
* **Rationale & Recommendation**:
  - Put the attribute probe behind a function or trait that tests can replace.
  - Define `PacketSource` (methods and file) and have `Ring` implement it.
  - Declare the fake worker as a `[[bin]]` target, use `env!("CARGO_BIN_EXE_…")`, and give jobs.rs a `worker_path` parameter.
  - Add `crates/gameclipd/src/lib.rs`.
  - Share synth through a `test-support` feature, `#[path]`, or a dev-only crate.

### 🟡 codebase-details-2-13: The `clip://` Range handling rejects WebView2's first media request - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Line 1343
* **Discrepancy Description**: The plan specifies only `bytes=a-b`. Chromium/WebView2 media requests typically start with `bytes=0-` (open-ended) and can send suffix ranges. Read literally, "ungültiger Bereich → 416" rejects the first request, so playback fails. The response length for an open-ended range is also undefined.
* **Rationale & Recommendation**:
  - Support `a-`, `a-b` and `-n`.
  - Answer an open-ended range with 206 and at most N bytes (for example 1 MiB), with a correct `Content-Range`.
  - Return 416 only when a ≥ size.

### 🟡 codebase-details-2-14: Dependency and feature gaps - Severity: Major
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness` (implementation-readiness rated it a Nit)
* **Affected Scope:** Lines 863-866, 889-913, 984, 1406, 1070
* **Discrepancy Description**:
  - No PNG decoder is listed, although the tray icons ship as PNG and `tray-icon` needs RGBA.
  - No JSON Merge Patch crate is listed for `SetSettings`, and the plan does not say it is hand-written.
  - `time` lacks the `parsing` feature.
  - The `windows` feature list lacks modules the plan calls: `Win32_Storage_FileSystem`, `Win32_UI_Input_KeyboardAndMouse`, the WinRT Graphics Capture and Direct3D11 interop modules, `Win32_Graphics_Imaging`, `Data_Xml_Dom`, `Win32_Security_Authorization` and `Win32_System_Console`.
  - gameclip-media is not listed as a `windows` user, although hwdevice.rs takes an `ID3D11Device`.
  - gameclipd lacks `uuid`, `time` and `clipboard-win`, which the M0 CF_HDROP copy needs.
  - ui-platform uses `PIPE_NAME` and `STARTUP_TASK_ID` from gameclip-core but depends only on gameclip-protocol.
* **Rationale & Recommendation**:
  - Add the missing rows and features.
  - Fix the dependency graph: media → windows, gameclipd → clipboard-win, uuid and time, ui-platform → core (or have protocol re-export the constants).

### 🟡 implementation-readiness-2-1: The CI order still breaks the Tauri crate build - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`. Codebase-details considered 1-8 resolved, but did not address the `generate_context!` point.
* **Affected Scope:** Line 947
* **Discrepancy Description**: `cargo clippy --workspace --all-targets` and `cargo test --workspace` compile `tauri::generate_context!`, which needs `../dist`. On a clean checkout, `../dist` only exists after `npm run build`, so both steps fail.
* **Rationale & Recommendation**: Run `npm ci && npm run build` first. This works because the generated types are committed. Then run clippy and test, then the drift check on the generated files.

### 🟡 implementation-readiness-2-2: Cross-reference numbers and the milestone mapping are wrong after renumbering - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`, `egm-codebase-details`, `egm-style` (N6)
* **Affected Scope:** Lines 871-873, 970 (also 383, outside the range)
* **Discrepancy Description**:
  - Stale numbers at line 970:
    - "Datei 22" (the message table) is now Datei 24.
    - "Datei 23" (the protocol example test) is now Datei 25.
    - "Datei 44" (the CLI) is now Datei 41.
  - Stale number at line 873: "Datei 39" (the Discord track) is now Datei 36.
  - At line 871, "25–31 (ohne Größen-Export)" is attached to the wrong range. The size export is in files 18 and 56.
  - Files 9-12, 57, 58 and 67 are not assigned to any milestone.
  - M0 builds the package (68-71), whose manifest needs gameclip-ui.exe, and S1 requires the pipe to be reachable from gameclip-ui (1419). The UI (58-66) is only built in M2.
* **Rationale & Recommendation**:
  - Renumber the references.
  - Assign every file to a milestone.
  - Either build a minimal ui-platform pipe client plus a UI stub in M0, or state that the M0 package contains only the service app.

### 🟡 implementation-readiness-2-3: The state machine silently drops runtime events - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`, `egm-codebase-details` (row 421, as a Nit)
* **Affected Scope:** Lines 996-998 (state table at 400-423)
* **Discrepancy Description**: The default "nicht aufgeführte Paare → unverändert" silently drops several events:
  - `HdrChanged` while buffering. The pipeline emits it at runtime (1168).
  - `HotkeyRegistered` and `HotkeyFailed` after `SetSettings{hotkey}`.
  - `CpuEncoderOnly` after recovery.
  - `SaveFinished` during reconfiguration or recovery. As a result, `saves_in_flight` never drops back and no sound plays.

  Separately, row 421 ("erfolgreiches TestRegister → hotkey_inactive fällt weg") contradicts line 1077, where `TestRegister` only probes the key and releases it.
* **Rationale & Recommendation**:
  - Add "beliebig" (any state) rows for `HdrChanged`, `HotkeyRegistered`, `HotkeyFailed`, `CpuEncoderOnly` and `SaveFinished`.
  - Change row 421 to trigger on `HotkeyRegistered` after `SetSettings`.

### 🟡 implementation-readiness-2-4: The allowed hotkeys contradict the Technical Plan - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`, `egm-codebase-details`
* **Affected Scope:** Lines 832, 1037 (rule at 361)
* **Discrepancy Description**: The settings table and the keymap allow A–Z and 0–9, possibly with no modifier. Line 361 allows only F1–F24 (except F12), plus combinations with Strg, Alt or Shift. A bare letter registered as a global `RegisterHotKey` would swallow that key system-wide.
* **Rationale & Recommendation**: Choose one rule, for example "A–Z and 0–9 only with at least one modifier". Apply it in `validate()` and `to_vk`, and cover it with a keymap test.

### 🟡 implementation-readiness-2-5: "Poolgröße 16" conflicts with "Einzeltexturen, keine Texture-Arrays" - Severity: Major
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** Lines 1118, 1127
* **Discrepancy Description**:
  - In FFmpeg's d3d11va hwframes, `initial_pool_size > 0` allocates one texture array of that size.
  - Single textures are only produced with a dynamic pool (`initial_pool_size = 0`).
  - Contexts derived for QSV need a fixed pool, which means an array.

  Confidence: medium-high.
* **Rationale & Recommendation**: Choose one of these and verify it in S2:
  - Use texture arrays with `D3D11_VPOV_DIMENSION_TEXTURE2DARRAY` and pass the slice index in `Converter::run`. This fits with cd-2-8.
  - Use a dynamic pool and handle QSV derivation separately.

### 🟡 style-2-1: Units, progress scale and field names differ from the Technical Plan, although line 1106 declares them fixed - Severity: Major
* **Reviewer Lens:** `egm-style`, `egm-codebase-details`, `egm-implementation-readiness` (raised as a Nit about outdated earlier sections)
* **Affected Scope:** Lines 1086, 1100, 1105, 1247 and 1106, against 307, 328, 336-338, 362, 371-377, 384-385 and 462
* **Discrepancy Description**: The plan and the Technical Plan disagree on the same fields:

  | Field | Plan | Technical Plan |
  | --- | --- | --- |
  | Duration in `Done` and the DDL | `duration_us` | `duration_ms` |
  | Export range | `in_us`/`out_us` | `in_pts`/`out_pts` |
  | Progress | `Progress{percent}` with no scale stated | `{"progress": 0.42}` (a 0–1 fraction) |
  | Ids | u32 | u64 |
  | Protocol version | single `protocol` | `protocol_major`/`protocol_minor` |
  | Test-register argument | `TestRegister{hotkey}` | `TestRegister{key}` |

  The Technical Plan also still has the old GetCapabilities fields, the old error-code list and `tauri-plugin-drag`.
* **Rationale & Recommendation**:
  - Make the plan (Datei 24) authoritative, and either align the earlier lines or replace them with a pointer to Datei 24.
  - State the progress scale explicitly, for example `percent: f32` (0.0–100.0), and use the same scale in the worker's JSON line and in `JobProgress`.

### 🟡 style-2-2: Two pairs of different types share the names `Event` and `Status` - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** Lines 996 and 1060 (`Event`); 1022, 1068, 1080 and 1093 (`Status`)
* **Discrepancy Description**:
  - `Event` is both the core state-machine input and the protocol's IPC event enum, and gameclipd uses both crates.
  - `Status` is both the clip status and the service-status body.
  - `ClipEntry.status` does not name its type, so the reader cannot tell which `Status` is meant.
  - ts-rs writes one file per type, so both `Status` types would map to the same `Status.ts`.
* **Rationale & Recommendation**: Rename them, for example `StateEvent`/`ServiceEvent` and `ClipStatus`/`ServiceStatus`, and name the type of `ClipEntry.status`.

### 🟢 style-2-3: The claim "the repository is empty" is false - Severity: Nit
* **Reviewer Lens:** `egm-style` (residue of implementation-readiness-1-37)
* **Affected Scope:** Line 789
* **Discrepancy Description**: The repository root also contains `design_draft.md` and several review reports.
* **Rationale & Recommendation**: Write "Das Repository enthält noch keinen Code; alle folgenden Dateien werden neu angelegt."

### 🟢 style-2-4: The file-entry template claim does not match the entries - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** Lines 880, 966
* **Discrepancy Description**: Line 880 says "Jeder Eintrag nennt Art, Änderungen und Begründung", but many entries have no "Art" or "Begründung".
* **Rationale & Recommendation**: Either narrow line 880, for example "Art und Begründung nur, wo nicht offensichtlich", or add the missing fields.

### 🟢 style-2-5: Save-step numbering and the `.part` cleanup wording - Severity: Nit
* **Reviewer Lens:** `egm-style`, `egm-codebase-details`
* **Affected Scope:** Lines 1218-1227
* **Discrepancy Description**:
  - "Schritte 1–9 aus ‚Speichern per Hotkey‘ in dieser Reihenfolge" does not match the order at lines 268-279: the program-name step and the partial-failure step move, and the shared-t0 step is missing.
  - Step 7 says "beide .part löschen", although the .m4a may already have been renamed in step 6.
* **Rationale & Recommendation**:
  - Drop "in dieser Reihenfolge", or renumber the Technical Plan. The plan's order is the correct one.
  - Clarify which files step 7 deletes.

### 🟢 style-2-6: The S5 wording uses `clip://` instead of the actual origin - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** Line 1422 (against 1335, 1362)
* **Discrepancy Description**: Lines 1335 and 1362 establish `http://clip.localhost` as the actual origin, but S5 says "über `clip://`".
* **Rationale & Recommendation**: Use one term throughout, for example "über das `clip`-Protokoll (`http://clip.localhost`)".

### 🟢 style-2-7: An unverified claim about compression - Severity: Nit
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** Line 1148
* **Discrepancy Description**: "damit die Kompression realistischer ist" claims something that nobody has verified.
* **Rationale & Recommendation**: Say what the noise actually does: "damit der Encoder das Bild nicht trivial komprimiert".

### 🟢 codebase-details-2-15: `SetWaitableTimer` time base - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Line 1164
* **Discrepancy Description**: `SetWaitableTimer` takes an absolute due time as FILETIME system time, not as a QPC value.
* **Rationale & Recommendation**: Use a one-shot relative wait of (t0 + n/fps) − QPC_now, recomputed on each tick.

### 🟢 codebase-details-2-16: Hotkey serde and Rust syntax details - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Lines 832, 1037
* **Discrepancy Description**:
  - The default `{ code = "F8" }` only deserializes if ctrl, alt and shift have `#[serde(default)]`.
  - `(u32 modifiers, u32 vk)` is not valid Rust syntax.
* **Rationale & Recommendation**: Add `#[serde(default)]` to the three flags and write the tuple as `(modifiers: u32, vk: u32)` or `(u32, u32)`.

### 🟢 codebase-details-2-17: `edition` belongs in `[workspace.package]` - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Line 888
* **Discrepancy Description**: In a virtual workspace, `edition` goes in `[workspace.package]`, and members use `edition.workspace = true`. Only `resolver = "3"` belongs in `[workspace]`.
* **Rationale & Recommendation**: Move `edition` to `[workspace.package]`.

### 🟢 codebase-details-2-18: Coverage gates and CI tooling details - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 947, 1413
* **Discrepancy Description**:
  - `--fail-under-lines` checks only the total, so it cannot enforce per-crate or per-file gates.
  - The install steps for cargo-llvm-cov, cargo-deny and cargo-about are missing.
  - `git diff --exit-code` misses new untracked generated files.
* **Rationale & Recommendation**:
  - Run per-crate gates as separate `-p` runs, and check retention.rs and export_plan.rs with a script over the lcov output.
  - Add the install steps.
  - Use `git status --porcelain` for the drift check.

### 🟢 codebase-details-2-19: Manifest StartupTask nesting - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`
* **Affected Scope:** Line 1391
* **Discrepancy Description**:
  - `uap5:StartupTask` must sit inside `<uap5:Extension Category="windows.startupTask" Executable="gameclipd.exe" EntryPoint="Windows.FullTrustApplication">`.
  - Both Applications need `EntryPoint="Windows.FullTrustApplication"`.
  - The `desktop` namespace is no longer used.
* **Rationale & Recommendation**: Show the nesting in the manifest description.

### 🟢 codebase-details-2-20: Export ladder edge cases and the client reference count - Severity: Nit
* **Reviewer Lens:** `egm-codebase-details`, `egm-implementation-readiness`
* **Affected Scope:** Lines 1016, 1291
* **Discrepancy Description**:
  - The width of the 480p tier is not defined. The test value implies 854.
  - Nothing forbids upscaling a 720p or 30 fps source to 1080p60.
  - With up to 4 clients, `WatchFolders` and `SuspendHotkey` need a reference count per connection, not "off on any disconnect".
* **Rationale & Recommendation**: Define the 480p width, forbid upscaling, and specify a reference count per connection.

### 🟢 implementation-readiness-2-6: The timeline end handle at the last frame - Severity: Nit
* **Reviewer Lens:** `egm-implementation-readiness`
* **Affected Scope:** Lines 1269-1271, 1369
* **Discrepancy Description**: The plan does not define what the end handle snaps to after the last frame.
* **Rationale & Recommendation**: Snap to the clip's `duration_us`, which is the end of the last frame.
