# Goldfish Review Report (Stage 7: Critic Review)

**Target:** `D:\ProgrammierProjekte\GameClip\docs\designs\gameclip.md` (lines 1-653, everything before "## Detailed Implementation Plan" at line 654; written in German)
**Mode:** REVIEW. **Baseline context:** `D:\ProgrammierProjekte\GameClip\concepts_brief.md`. Greenfield project, no code exists.
**Standards:** egm-design and egm-structural used `resources/document_standards/feature-spec-standard.md`. egm-style used its persona's own rules.

## Reviewer Status

| Reviewer | Status | Raw findings reported |
| --- | --- | --- |
| `egm-design` | completed | 0 Blocker / 14 Major / 11 Nit |
| `egm-structural` | completed | 1 Blocker / 6 Major / 9 Nit |
| `egm-style` | completed | 0 Blocker / 9 Major / 14 Nit |

Each reviewer ran once, and no retries were needed.

**After deduplication:** 56 findings: 🔴 1 Blocker, 🟡 25 Major, 🟢 30 Nit. When two reviewers reported the same finding, it is listed once under the primary reviewer's ID, and the others are named under "Also flagged by". Merged findings keep the highest severity any reviewer gave them.

---

## 🔴 Blockers

### structural-1-1: No "Risks & Mitigations" section - Severity: Blocker
* **Reviewer Lens:** `egm-structural` (also flagged by `egm-design` as Major, design #1)
* **Affected Scope:** whole range; risk content is scattered at 40, 306, 450, 624-634
* **Discrepancy Description:** The standard requires a Risks & Limits section that names failure modes, security implications, scalability limits and how each is mitigated. The document has no such section. Several limits are never presented as limits with a mitigation:
  * the RAM ceiling of 25 % (228)
  * one export job at a time (186/285)
  * NVENC session limit falling back to x264 (306)
  * the 8 MB IPC cap (330)
  * loss of stars when the index is corrupt (277)
  * MSIX uninstall deleting single tracks and the index (449)
  * the unconfirmed Vanguard outcome (G4)
* **Rationale & Recommendation:** Add a "## Risks & Mitigations" section, for example before Non-Functional Requirements. Use a table with the columns Risk | Kind | Effect | Mitigation | Link (Q/S/G). Include at least: Vanguard/exclusive fullscreen, SignPath/MSIX signing (design-1-7), worker GPU contention (design-1-6), index loss (design-1-4), OneDrive folders (design-1-8), always-on recording (design-1-9), MSIX rollback (design-1-10).

---

## 🟡 Major

### structural-1-2: Rejections are attributed to the author without linking the decision record - Severity: Major
* **Reviewer Lens:** `egm-structural` (also `egm-design` #2)
* **Affected Scope:** 460, 466, 472, 478, 484, 490, 496, 504, 510, 516, 522, 528, 536, 542, 548, 554, 562, 568, 574, 582, 588, 594, 600, 606, 612, 618
* **Discrepancy Description:** All 26 alternatives say "Rejected by author:" with no link. The Alternatives Rule forbids attributing a rejection to a maintainer without linking the issue, discussion or PR. The brief's D1-D25 register is that record, but no entry cites a D-ID.
* **Rationale & Recommendation:** Add the D-ID and an anchor link into concepts_brief.md to each entry. Suggested mapping: A→D6, C→D9, D→D11, E/H→D3, I/J→D2, K→D4, L→D12, M/N/O→D7, Q/R→D10, S→D24, T→D5, U→D8. Link V-Z to the T1-T5 rows (139-143). The alternative is to drop "by author" and keep only the factual reason.

### structural-1-3: Several rejection reasons restate the decision instead of naming a trade-off - Severity: Major
* **Reviewer Lens:** `egm-structural` (also `egm-design` #2)
* **Affected Scope:** 478 (D), 496 (G), 516 (J), 522 (K), 536 (M), 542 (N), 548 (O), 562 (Q), 568 (R), 588 (U)
* **Discrepancy Description:** Examples: "gehören zum Kern", "zurückgestellt, kein Teil von v1", "Clip- und Export-Ordner enthalten immer nur die Mischung". Each one just repeats the chosen solution. The trade-off sometimes appears under Cons but is never weighed.
* **Rationale & Recommendation:** Rewrite each reason so it says why the cost outweighs the benefit. Examples: O: compatibility with phones and WhatsApp versus a separate index mapping. J: SSD write volume per day. R: anti-cheat risk and blindness to admin-focused games.

### structural-1-4: Load-bearing choices have no alternatives - Severity: Major
* **Reviewer Lens:** `egm-structural` (also `egm-design` #3)
* **Affected Scope:** 192-205, 211, 279-318, 410-425, 436, 452-618
* **Discrepancy Description:** These choices are never weighed against an alternative:
  * WGC versus DDA as the primary capture path
  * Exclude-Self loopback versus plain endpoint loopback (211; brief P2 is unconfirmed)
  * the platform-neutral adapter abstraction (rejected in the brief at line 301, missing here)
  * H.264 only versus HEVC/AV1 (AV1 hardware matters a lot at 10 MB)
  * FFmpeg/rsmpeg versus vendor SDKs or Media Foundation
  * fixed 60 fps pacer versus VFR
  * MP4 tag versus NTFS ADS or sidecar file as the ownership marker
* **Rationale & Recommendation:** Add one alternative entry for each, with pros, cons and a linked rationale.

### structural-1-5: Open Questions are out of sync with the body - Severity: Major
* **Reviewer Lens:** `egm-structural` (also `egm-design` #19, Nit)
* **Affected Scope:** 89-91 versus 219, 245, 249-250, 271-277
* **Discrepancy Description:** The body already answers Q12 (250, 273), Q13 (245, 249) and Q11's fallback (219), but all three are still listed as open. A fresh reader cannot tell what is really open.
* **Rationale & Recommendation:** Mark each one "Vorgeschlagen: siehe Abschnitt X, Bestätigung durch S4", or move it to a resolved list. Keep only what is still unknown.

### structural-1-6: State machine table is incomplete, although tests are derived from it - Severity: Major
* **Reviewer Lens:** `egm-structural` (also `egm-design` #12)
* **Affected Scope:** 347-367 (brief 168-176)
* **Discrepancy Description:** Missing transitions:
  * "Fehler" has no way out (retry, user action, restart).
  * No transition goes back to "Leerlauf" (stop or quit).
  * Device loss during "Speichert" or "Eingeschränkt".
  * A settings change while "Eingeschränkt".
  * Disk full or a missing folder while "Puffert".

  "Eingeschränkt" lumps three independent causes into one state: hotkey failed, CPU encoder, HDR. Also, "Einstellung geändert → Puffer neu anlegen" (359) applies to every setting, but 224 limits it to length and quality.
* **Rationale & Recommendation:** Add the missing state×event rows. Model the restrictions as flags orthogonal to the main state. Restrict the reconfigure trigger to settings that affect the pipeline.

### structural-1-7: The brief requires buffer start/stop in IPC, but the command list lacks it - Severity: Major
* **Reviewer Lens:** `egm-structural`
* **Affected Scope:** 338-342 (brief 148)
* **Discrepancy Description:** There is no StartBuffer or StopBuffer command, which ties in with the missing transition to Leerlauf. The brief's StartTrimJob and StartExportJob were merged into one StartExportJob without saying so.
* **Rationale & Recommendation:** Add the commands or explain why they were dropped. Document the merge.

### design-1-1: Dropping the "newest" packets when the reserve runs out corrupts the H.264 stream - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 227, 232
* **Discrepancy Description:** Every P-frame references the frame before it. If packets are dropped in the middle of a GOP, every later frame up to the next keyframe is undecodable, and that broken run ends up in the next clip. The design does not say how audio is handled in the same window or how the gap is marked. This case is likely: a second queued save, a slow HDD and about 1.1 GB pinned.
* **Rationale & Recommendation:**
  * When the pool runs out, drop until the next keyframe and force an IDR.
  * Handle audio for the same window consistently.
  * Mark the gap in the packet index.
  * Size the reserve from the worst case.
  * Add a test.

### design-1-2: G3 contradicts the 15 % reserve - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 56, 224, 228
* **Discrepancy Description:** G3 says the buffer will be "höchstens 5 % Abweichung" from what is displayed. But the pool is reserved at forecast plus 15 % (224), and the UI shows only the forecast (228).
* **Rationale & Recommendation:** Either include the reserve in the display or in the goal. Also say whether the memory is committed or only reserved, because that decides which metric is measured.

### design-1-3: Time reference between the MP4 and the single-track `.m4a` is undefined - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 234-237, 300, 309
* **Discrepancy Description:** AAC priming, edit lists and the first audio packet relative to the first keyframe differ between the two files. Nothing says how sample-accurate alignment is achieved, yet CUJ 4 and the A/V criterion depend on it.
* **Rationale & Recommendation:** Use a shared t0 (the PTS of the first keyframe), written as an edit list or as `start_offset` in the index. Add a phase-alignment acceptance criterion to S4/S9.

### design-1-4: Index loss can permanently delete clips that were starred - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 246, 275, 277, 449, 606
* **Discrepancy Description:** If the index is lost (corruption or MSIX reinstall), every star is lost. Clips the user adopts then count as unprotected, and the next save deletes them with `DeleteFileW`. Until they are adopted they are `foreign`, so the 10 GB limit stops working. Line 277 also contradicts itself: "aus den Metadaten-Tags neu aufgebaut" versus "alle Clips gelten dann als `foreign`".
* **Rationale & Recommendation:** Persist stars outside the index (ADS or sidecar file), or back up `index.sqlite` outside the package path. Adopt clips as protected by default. Spell out what the rebuild actually restores.

### design-1-5: Reading the program name opens a handle to the game process, which conflicts with the Vanguard rule - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 38, 238
* **Discrepancy Description:** `QueryFullProcessImageName` needs `OpenProcess` on the game process. Vanguard protects that process, so the call can fail or be logged. This contradicts "ohne den Spielprozess anzufassen".
* **Rationale & Recommendation:** Get the PID with `GetWindowThreadProcessId` and the image name from `CreateToolhelp32Snapshot`, so no handle is opened. Or add the handle open to the S1 Vanguard test protocol.

### design-1-6: The worker competes with the running game for the GPU - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 111, 285, 297-306
* **Discrepancy Description:** During a game, an export adds a d3d11va decode, GPU scaling, a second encoder session and possibly a retry. `BELOW_NORMAL` only lowers CPU priority. The game's frame rate during an export is not covered by any criterion.
* **Rationale & Recommendation:** Add a limit on the game's frame drops during an export to S9 (measured with PresentMon). Give the worker device a low GPU priority. Optionally throttle or defer exports while a fullscreen game runs.

### design-1-7: MSIX hard-depends on signing, and signing is unverified - Severity: Major
* **Reviewer Lens:** `egm-design` (related: style-1-6)
* **Affected Scope:** 83, 439, 444-450, 594, 629
* **Discrepancy Description:** SignPath's MSIX support is "prüfen" (439), and there is no plan B. Signing third-party BtbN FFmpeg DLLs (629) conflicts with SignPath's build-from-source rules. Sideloading needs a trusted certificate (S7).
* **Rationale & Recommendation:** Resolve this before S1 and make it a gate. Document a fallback: Azure Trusted Signing, an OV certificate, or Alternative V.

### design-1-8: OneDrive Known Folder Move and cloud placeholders are not considered - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 245, 272-274, 377-378
* **Discrepancy Description:** `Videos` is often redirected to OneDrive. Reading the tag of a placeholder file triggers a download. Permanent deletes sync to the cloud. Sync noise drives `ReadDirectoryChangesW`. The 3 s save competes with the sync client.
* **Rationale & Recommendation:** Use `SHGetKnownFolderPath(FOLDERID_Videos)`. Detect the recall and offline attributes, skip placeholders during reconciliation, and warn when a cloud folder is chosen. Add this to the risk table.

### design-1-9: Always-on recording ignores battery, lock screen and privacy - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 12, 54-57, 446
* **Discrepancy Description:** The monitor and microphone are recorded continuously from login. The design has no policy for battery or energy saver, lock or sleep, or sensitive screen content outside games. It never measures GPU load or power draw.
* **Rationale & Recommendation:** Define a policy: pause on battery or energy saver, pause on lock, an optional "nur bei Vollbild-App" mode, and a GPU/power measurement in Q1. At minimum, list this as a known limit.

### design-1-10: The rollback path through an older MSIX version does not work as described - Severity: Major
* **Reviewer Lens:** `egm-design`
* **Affected Scope:** 449, 635-639
* **Discrepancy Description:** MSIX blocks downgrades by default. Rolling back means uninstalling first, which wipes the virtualized AppData: settings, index and single tracks. How an older version treats a newer SQLite schema is undefined.
* **Rationale & Recommendation:** Document the real rollback path (uninstall versus `ForceUpdateFromAnyVersion`) and its data loss. Define what happens with a newer `schema_version`, for example read-only mode and auto-delete paused.

### style-1-1: G1 title and risk wording are subjective, and the title does not match the criteria - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 40, 46, 52
* **Discrepancy Description:** "Spürbar besser/schneller" cannot be measured. Two of the four criteria are not about speed. "Schlagen" has no margin.
* **Rationale & Recommendation:** Retitle, e.g. "Besser als Game Bar in mindestens 2 von 4 Messgrößen". Define the direction and minimum margin per criterion, or defer it to Q1.

### style-1-2: Problem statement and journey make unsupported generalizations - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 33, 34, 36, 38
* **Discrepancy Description:** "oft zu groß", "kaum Funktionen", "umständlich", "meist zwei oder drei Programme" and "Viele Spieler" state frequency or quality without evidence.
* **Rationale & Recommendation:** Replace them with checkable facts, or tie them to the G1 measurement.

### style-1-3: G8 promises "nie" but is verified with 20 clips and one retry - Severity: Major
* **Reviewer Lens:** `egm-style` (also `egm-design` #15)
* **Affected Scope:** 62, 111, 302, 312, 315
* **Discrepancy Description:** "Überschreitet die Zielgröße nie" is an absolute guarantee. The acceptance test uses 20 clips. After one retry at 10 % lower bitrate, the behaviour is undefined, and hardware rate control gives no hard size guarantee. At 5 min and 10 MB the video bitrate is about 150 kbit/s.
* **Rationale & Recommendation:** Reword to the test criterion. Define the terminal behaviour: iterate using the measured overshoot factor, fall back to x264 two-pass, or end with `JobFailed{TooLarge}`. Add a minimum-quality warning for long clips. See style-1-22 for the MB/MiB question.

### style-1-4: G5 "überall vollständig zu hören" is vague - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 59, 117
* **Discrepancy Description:** The real property is that the default track holds all active sources.
* **Rationale & Recommendation:** State that property directly and name the test targets (Discord, WhatsApp, Android/iOS gallery). Rephrase 117 to "Player, die nur die Standardspur abspielen".

### style-1-5: "Ohne Windows-Code" claims more than the CI check enforces - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 21, 460 versus 420, 424-425
* **Discrepancy Description:** Lines 21 and 460 include the window and the UI. But `gameclip-ui` may use Windows code via `gameclip-ui-platform`, and `cargo deny` checks only `gameclip-core` and `gameclip-protocol`.
* **Rationale & Recommendation:** Name exactly what CI enforces and drop "Fenster", or extend the check.

### style-1-6: Inline "(prüfen)" items are not tracked as Open Questions - Severity: Major
* **Reviewer Lens:** `egm-style` (related: design-1-7)
* **Affected Scope:** 437, 439, 445, 450
* **Discrepancy Description:** Untracked items: SignPath MSIX support, the `graphicsCaptureWithoutBorder` and `graphicsCaptureProgrammatic` capabilities, licence and maintenance checks, and "wie erwartet verhalten". None has an owner or deadline, and line 450 never says what "erwartet" means.
* **Rationale & Recommendation:** Add Q entries with a deadline and an answer source. List the expected behaviour for line 450.

### style-1-7: Goals combine "höchstens" with a range or "~" - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 55, 57
* **Discrepancy Description:** "Höchstens 60 bis 80 MB" and "höchstens ~3 %" cannot be tested.
* **Rationale & Recommendation:** Write "Ziel ≤ 80 MB (vorläufig, Q1)" and "Ziel ≤ 3 % (vorläufig, Q1)".

### style-1-8: Core terms vary - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:**
  * Kürzen/Trim: 61, 142, 168, 189, 279, 316, 392, 395
  * Worker/Arbeitsprozess: 111, 187, 286-302, 317, 627
  * Mischspur/Spur „Mischung“: 105, 236, 300, 308
  * Drift/Abweichung: 216, 431
  * Benachrichtigung/Toast: 184, 408
* **Discrepancy Description:** The same concept goes by different names in different places.
* **Rationale & Recommendation:** Pick one term each and keep a glossary (see structural-1-15).

### style-1-9: Wire and DB identifiers mix German and English - Severity: Major
* **Reviewer Lens:** `egm-style`
* **Affected Scope:** 262, 264, 281-284, 287, 291, 335
* **Discrepancy Description:**
  * `duration_ms`/`size_bytes` versus `dauer_ms`/`bytes` for the same quantities.
  * `protokoll_minor`, `einzelspuren_pfad` and `ziel_part_pfad` sit next to `clip_id`, `in_pts` and `adapter_luid`.
  * The profile name `Zielgröße` is non-ASCII and will become a ts-rs enum identifier.
* **Rationale & Recommendation:** Use English ASCII for all wire, DB and enum identifiers. UI labels can stay German.

---

## 🟢 Nits

### structural-1-8: "Future Improvements" is an H3 under NFR - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** 643
* **Discrepancy / Recommendation:** It is a separate top-level section. Change it to `##`.

### structural-1-9: Gaps in Open Question numbering are not explained - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** 82-91
* **Discrepancy / Recommendation:** Q8-Q10 are missing. Add a note: "Q8-Q10 im Brief erledigt (D7, D21, D22)".

### structural-1-10: Architecture decisions table has a wrong column header, no links and a stray tag - Severity: Nit
* **Reviewer Lens:** `egm-structural` (also `egm-style` N13) | **Affected Scope:** 137-143, 188
* **Discrepancy / Recommendation:** Rename the column "Optionen" to "Entscheidung". Add rejected options (→ Alt. V-Z) and use anchor links. Move the stray "[T2]" at 188 out of the "Läuft in" column.

### structural-1-11: Category heading does not match its contents - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** 556, 570, 576-588
* **Discrepancy / Recommendation:** "Architektur (T1-T5)" also contains T (D5) and U (D8). S (Papierkorb) sits under "Bedienung". Rename the heading or split T/U out, and move S to a storage category.

### structural-1-12: ASCII diagram has a broken right border - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** 147-171 (especially 156-163)
* **Discrepancy / Recommendation:** Realign the border or drop the outer frame.

### structural-1-13: Content repeated inside "Kürzen und Export" - Severity: Nit
* **Reviewer Lens:** `egm-structural` (also `egm-style` N1) | **Affected Scope:** 296-315 (302/315, 303/305, 300/310)
* **Discrepancy / Recommendation:** The 10 % retry, Smart-Cut and audio re-encoding are each stated twice. Keep each rule in one place.

### structural-1-14: Brief references are not links, and the CLI name is inconsistent - Severity: Nit
* **Reviewer Lens:** `egm-structural` (also `egm-design` #22, `egm-style` N14) | **Affected Scope:** 68, 78, 645
* **Discrepancy / Recommendation:** "Anhang A des Briefs" is plain text, so link it. Line 78 uses `gameclipd.exe --save-clip`, but brief D23 uses `gameclip --save-clip`. Settle on one name.

### structural-1-15: No glossary - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** whole range
* **Discrepancy / Recommendation:** WGC, DDA, AUMID, GOP, QPC, TDR, LUID, PTS and CF_HDROP are never defined. Add a Begriffe table or link the brief's.

### structural-1-16: Horizontal rules are inconsistent, and there is no revision history - Severity: Nit
* **Reviewer Lens:** `egm-structural` | **Affected Scope:** 3-6, 8, 620, 641, 652
* **Discrepancy / Recommendation:** Put rules before every H2 or before none. Add a revision line.

### design-1-11: A Discord restart also requires rebinding the System stream - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 211, 213
* **Discrepancy / Recommendation:** The System stream excludes Discord by PID, so after a restart Discord is heard twice in the mix. Specify the rebind for both streams, how the root process is found, and how PTB and Canary are handled.

### design-1-12: Drift compensation is only asserted - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 216
* **Discrepancy / Recommendation:** Describe the control variable (device clock versus QPC), the control loop and its limits.

### design-1-13: Predictable pipe name allows squatting - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 328, 345
* **Discrepancy / Recommendation:** Use `FILE_FLAG_FIRST_PIPE_INSTANCE` and have the client verify the server process. State the threat-model boundary.

### design-1-14: Clips that span several sections deviate from the brief - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 234 (brief 81)
* **Discrepancy / Recommendation:** The brief says several files with a hint. The design saves only the newest section, with no hint. Reconcile the two and document the change.

### design-1-15: Diagnostics omit key events - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 427-432 (brief 69)
* **Discrepancy / Recommendation:** Log changes of capture path and encoder, and the hotkey registration result with its error code.

### design-1-16: Export naming and the job protocol are underspecified - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 316, 317, 342
* **Discrepancy / Recommendation:** The `_2` collision rule is missing for `_trim` and `_10MB`. `StartExportJob` has no response carrying a `job_id`, and `JobProgress` has no `job_id`.

### design-1-17: Starting the daemon from the UI is undefined - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 19, 444-446
* **Discrepancy / Recommendation:** Say who starts `gameclipd` after the user quit it from the tray. MSIX `startupTask` also needs `RequestEnableAsync` or a first-run activation.

### design-1-18: Unbacked assertions need a prototype - Severity: Nit
* **Reviewer Lens:** `egm-design` | **Affected Scope:** 55, 395
* **Discrepancy / Recommendation:** The RAM target with FFmpeg, D3D11 and SQLite loaded, and `clip://` with range support, are stated as facts. Mark them "zu bestätigen in S1/S5".

### style-1-10: Vague latency wording - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 203, 233
* **Discrepancy / Recommendation:** "Fast keine Verzögerung" and "höchstens wenige Bilder" give no number. Give a bound or drop them.

### style-1-11: "Wird nie beschädigt" is absolute - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 227
* **Discrepancy / Recommendation:** State the mechanism instead: blocks are pinned by reference counting and never overwritten. See also design-1-1.

### style-1-12: Hyperbole in Alternatives - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 465, 495, 502, 526, 581, 612
* **Discrepancy / Recommendation:** "Jahre an Treiber-Sonderfällen", "kaum", "deutlich größer" and similar. Give magnitudes or use neutral wording.

### style-1-13: Subjective wording in Alternative V - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 593-594
* **Discrepancy / Recommendation:** "Saubere Identität", "Verknüpfungstricks" and "schwer zu debuggen" are judgments. Name the facts instead.

### style-1-14: "Die Warnung deckt das Risiko ab" overclaims - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 562
* **Discrepancy / Recommendation:** The warning text is still open (Q6), and a warning only points to the risk. Change to "weist auf das Risiko hin".

### style-1-15: Wrong cross-reference and overclaim in Alternative E - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 484
* **Discrepancy / Recommendation:** The "keine Injektion" rule lives in D3 and at line 38, not in G4. Change "ersetzen das Overlay" to "übernehmen die Bestätigung".

### style-1-16: G10 and the UI section contain unmeasurable goals - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 64, 393 (versus 98)
* **Discrepancy / Recommendation:** "Übersichtlich" and "ruhig" cannot be measured, so keep only the three-step criterion. "Mikrofon prüfen" differs from CUJ 1, which says "bestätigt oder wechselt".

### style-1-17: Vague intensifiers - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 522, 632
* **Discrepancy / Recommendation:** Replace "deutlich" with the concrete UI form, e.g. tray state plus toast.

### style-1-18: Filler adjectives - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 390, 391, 419
* **Discrepancy / Recommendation:** Drop "klein" and "einfach".

### style-1-19: Wordy deletion rule - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 246
* **Discrepancy / Recommendation:** The condition just means "Summe aller eigenen Dateien". Shorten it.

### style-1-20: Repeated "trotzdem" in CUJ 5 - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 123
* **Discrepancy / Recommendation:** Rephrase one of the two.

### style-1-21: Mixed-language rejection lines - Severity: Nit
* **Reviewer Lens:** `egm-style` | **Affected Scope:** 510, 568, 588
* **Discrepancy / Recommendation:** "Rejected by author nach Review" switches language mid-phrase. Use one language. See also structural-1-2.

### style-1-22: Ambiguous units and a pronoun reference - Severity: Nit
* **Reviewer Lens:** `egm-style` (units also raised by `egm-design` #15) | **Affected Scope:** 18, 62, 244, 284, 312
* **Discrepancy / Recommendation:** Say whether MB/GB is decimal or binary, e.g. "10 MB = 10 000 000 Bytes". At line 18 write "Der Dienst" instead of "Er".
