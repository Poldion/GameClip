Status: resolved (vom Autor beauftragt; keine weitere Nachprüfung erlaubt – Selbstprüfung gegen den Bericht)

# Goldfish Annotated Review Report – Stufe 9, Nachprüfung

**Target:** `docs/designs/gameclip.md` · **Quelle:** [implementation_readiness_report_round2.md](implementation_readiness_report_round2.md) (1 Blocker, 19 Major, 12 Nits)
**Sicherung vor diesen Korrekturen:** `C:\Users\JoshS\AppData\Local\Temp\claude\D--ProgrammierProjekte-GameClip\e1ddde65-38e4-4e95-b57d-64b85a238dcf\scratchpad\gameclip.before-final.md`

Alle Befunde: PROPOSE FIX, angewendet. Kein CONTEST, kein ESCALATE neu (Q17, Q18 bleiben offen).

## Blocker und Major – Selbstprüfung

| Befund | Änderung | Geprüft |
| --- | --- | --- |
| cd-2-1 Index-Schlüssel | `row_id INTEGER PRIMARY KEY` (= `clip_id`) und `tag_id` ohne UNIQUE; Eigentumsregel „`ok` und Tag = `tag_id`, höchstens eine `ok`-Zeile je `tag_id`“; `AdoptClip` schreibt den Tag neu (Umpacken); Einzelspuren unter `tracks\<tag_id>.m4a`; Technischer Plan angeglichen | DDL, `ClipInfo`, `ClipEntry`, `reconcile`, Tests (Datei 55) nutzen `row_id`/`tag_id` |
| cd-2-2 `sync_all` | `OpenOptions::new().write(true)` in Datei 29, Worker verweist darauf | Technischer Plan sagt „mit Schreibrecht neu öffnen“ |
| cd-2-3 ts-rs | Regel `#[ts(type = "number")]` für alle `u64`; Feature `serde-json-impl`; `gameclip-core` mit optionalem Feature `ts`; `extra` übersprungen; Zeitpunkte als RFC-3339-String; `TS_RS_EXPORT_DIR` in `.cargo/config.toml`; benannte `…Reply`-Typen | Dateien 1, 3, 13, 16, 24 |
| cd-2-4 Grenz-Prüfung | `cargo tree … --prefix none --format "{p}"` mit Mustervergleich; Fehler des Befehls zählt als Fehler; Technischer Plan verweist auf Datei 7 | kein `-i windows` mehr im Dokument |
| cd-2-5 Media Feature Pack | `LoadLibraryExW` in `media.rs` als einzige Stelle; kein Ladezeit-Linken; Feld aus `GetCapabilities` entfernt; `dumpbin`-Prüfung in S2 | Dateien 24, 26, 58, Prüfplan S2 |
| cd-2-6 Toast-Aktivierung | Manifest ohne `Parameters`; `main.rs` erkennt `gameclip:reveal?id=`; neuer Befehl `GetClip`; Rückfall auf den Clip-Ordner | Dateien 24, 60, 68 |
| cd-2-7 IDR | `forced_idr 1` für AMF und QSV; Regel „jeder Schnappschuss/jede Lücke beginnt auf einem IDR“ | Datei 27 |
| cd-2-8 Converter | Eingabe BGRA oder NV12, `TexRef { tex, array_slice }`; Software-Dekodierung über `av_hwframe_transfer_data` in den Worker-Pool; eigener Frames-Kontext je Prozess | Dateien 26, 27, 29 |
| cd-2-9 gemeinsames Limit | `retention_groups`: eine Gruppe mit `min(...)` bei gleichem Ordner; Testfall mit Zahlen | Dateien 19, 22, 47 |
| cd-2-10 Datenverlust früherer Ordner | „fehlt“ = `Path::exists` je Zeile; Scan nur für neue Dateien im aktuellen Ordner; `in_current_folder` bei der Abfrage berechnet; Test | Dateien 45, 55 |
| cd-2-11 Größen-Export | Rechnung in `f64`; `Result<ExportPlan, PlanError::TooSmall>` mit `MIN_VIDEO_BPS = 50 000`; `GetExportEstimate` liefert `too_small`; Dienst bricht ohne Worker mit `TooLarge` ab; kein Hochskalieren; 480p = 854×480; Testzeile 1 MB / 300 s | Dateien 18, 22, 24, 49, 56 |
| cd-2-12 Test-Infrastruktur | `FileProbe`-Trait statt Attribut setzen; `PacketSource` in `ring.rs`; `fake-worker` als `[[bin]]` mit `required-features`; `worker_path`-Parameter; `gameclipd/src/lib.rs`; Crate `gameclip-testsupport` | Dateien 30, 38, 40, 45, 49, 55, 57 |
| cd-2-13 Range | `a-b`, `a-`, `-n`; 206 mit höchstens 1 MiB; 416 nur bei `a ≥ size`; S5 prüft `bytes=0-` | Datei 60, Prüfplan |
| cd-2-14 Abhängigkeiten | `png`, `json-patch`, `time/parsing`, vollständige `windows`-Features, Graph korrigiert (media → windows, gameclipd → clipboard-win/uuid/time, ui-platform → core) | Datei 1, Abhängigkeitsgraph |
| ir-2-1 CI-Reihenfolge | `npm ci && npm run build` vor allen Cargo-Schritten | Datei 7 |
| ir-2-2 Nummern und Meilensteine | Verweise auf Dateien 24, 25, 41, 36 korrigiert; Meilenstein-Tabelle mit allen Dateien; M0-Paket nur Dienst und Worker, Pipe-Prüfung über `--save-clip` | Meilensteine, Datei 68, S1 |
| ir-2-3 verworfene Ereignisse | „beliebig“-Regeln für `HdrChanged`, `HotkeyRegistered`, `HotkeyFailed`, `CpuEncoderOnly`, `SaveFinished`; Tabellenzeile auf `HotkeyRegistered` nach `SetSettings` | Datei 15, Zustandstabelle |
| ir-2-4 Hotkeys | A–Z/0–9 nur mit Strg oder Alt; in Einstellungen, `keymap`, Technischem Plan und Tests | Dateien 16, 21, 22, Hotkey-Abschnitt |
| ir-2-5 Pool vs. Einzeltexturen | Texture-Array mit `initial_pool_size = 16`, Slice-Index an `Converter`; Prüfung in S2 | Dateien 26, 27 |
| style-2-1 Abweichungen | Datei 24 maßgeblich; Technischer Plan angeglichen (`duration_us`, `in_us`, Fortschritt 0–100, `u32`-IDs, eine Protokollversion, `TestRegister{hotkey}`, Fähigkeiten, Fehlercodes, `drag`-Crate) | Suche nach alten Namen ergibt 0 Treffer |
| style-2-2 doppelte Namen | `StateEvent`/`ServiceEvent`, `ClipStatus`/`ServiceStatus`, `ServiceState` | Dateien 15, 19, 24 |

## Nits – angewendet

style-2-3 (Repository-Satz), style-2-4 (Feldregel), style-2-5 (Schrittfolge, `.part`-Löschen), style-2-6 (S5-Wortlaut), style-2-7 (Rauschen), cd-2-15 (relativer Timer), cd-2-16 (`serde(default)`, Tupel-Syntax), cd-2-17 (`[workspace.package]`), cd-2-18 (Installationsschritte, Abdeckung je Datei per Skript, `git status --porcelain`), cd-2-19 (StartupTask-Verschachtelung, `EntryPoint`, `desktop` entfernt), cd-2-20 (480p-Breite, kein Hochskalieren, Zählung je Verbindung), ir-2-6 (Endgriff am letzten Bild).

## Nicht durch Lesen prüfbar

Weiterhin in Prototypen oder Q16 zu bestätigen: `rusty_ffmpeg`-Variablen, QSV-/AMF-Optionsnamen in FFmpeg 7.1, Texture-Array-Pool je Hersteller, `graphicsCapture*`-Namespace (Q15), Toasts mit `AppListEntry="none"`, Flatten des adjazent getaggten serde-Enums, exakte Crate-Versionen und Feature-Namen von `windows` 0.61.
