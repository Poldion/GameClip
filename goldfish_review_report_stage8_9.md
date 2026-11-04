Status: resolved (vom Autor beauftragt: „Löse du die Befunde für mich“; alle PROPOSE FIX angewendet)

# Goldfish Annotated Review Report – Stufe 8 und 9

**Target:** `docs/designs/gameclip.md` (ganzes Dokument; Stufe 8 Zeilen 1–1371, Stufe 9 Zeilen 762–1370 der vorherigen Fassung)
**Quellen:** [intermediate_zoom_out_report.md](intermediate_zoom_out_report.md) (Stufe 8: 0 B / 7 M / 9 N), [implementation_readiness_report.md](implementation_readiness_report.md) (Stufe 9: 3 B / 43 M / 27 N)
**Sicherung der vorherigen Fassung:** `C:\Users\JoshS\AppData\Local\Temp\claude\D--ProgrammierProjekte-GameClip\e1ddde65-38e4-4e95-b57d-64b85a238dcf\scratchpad\gameclip.before-stage9.md`

**Übersicht:** 2 ESCALATE · 4 vom Autor entschieden · 0 CONTEST · Rest PROPOSE FIX (angewendet) · 2 DECLINE

## Vom Autor entschieden (Architektur-Gates)

| Befund | Entscheidung | Folge im Dokument |
| --- | --- | --- |
| S8-M5 / Stufe 5 M1 – eigener Worker statt `ffmpeg.exe` | **A: eigener Worker bleibt** (T4) | Alternative Y vermerkt die dreimalige Gegenempfehlung. Worker-Grundgerüst schon in M0 für S9. Neues Crate `gameclip-gpu` (VideoProcessor) für Aufnahme und Worker; der Callback über Crate-Grenzen entfällt. |
| S8-M4 / Stufe 5 M2 – Abgleich des Index | **C: bei Bedarf, Watcher nur bei offener Galerie** (T6) | Watcher löst nur einen Abgleich aus; 24-h-Frist ersetzt durch „zweimal hintereinander fehlend“; `.part` und `pending` werden übersprungen; Überlauf → Abgleich. |
| S8-M6 / Stufe 5 M3 – `gameclip-fake-daemon` | **C: gestrichen, Mock-Modus im Frontend** (T7) | `ipc.mock.ts`, `npm run dev:mock`; `interprocess` entfällt. |
| S8-M7 / Stufe 5 M4 – `clip://` | **A: eigenes `clip://`, korrigiert** (T8) | Ursprung `http://clip.localhost`, CSP, Capabilities, Routen, 206/404/416 festgelegt. |

## ESCALATE (brauchen weiterhin deine Antwort)

| ID | Frage | Stand im Dokument |
| --- | --- | --- |
| Q17 (früher E1) | Automatische Pausen: beim Sperren, im Akkubetrieb, außerhalb von Vollbild-Apps? | Open Question; bis dahin pausiert nur der Nutzer. Risiko R13 verweist darauf. |
| Q18 (früher E2) | Bestätigung P2 (System-Ton ohne GameClips eigenen Ton) | Open Question; Alternative AB als „Bestätigung offen“ gekennzeichnet. |

## PROPOSE FIX – angewendet

### Stufe 9 – Blocker

| Befund | Änderung |
| --- | --- |
| implementation-readiness-1-1 IPC-Bodies | Vollständige Nachrichtentabelle (Befehle, Antworten, Ereignisse) in Datei 24; Handshake mit Antwort und Schließen bei ungleicher Version; JSON-Merge-Patch für `SetSettings`; `FieldError` in `details`. |
| implementation-readiness-1-2 Schema `settings.toml` | Tabelle „Einstellungen“ mit TOML-Schlüssel, Typ, Einheit, Standard, Bereich, Pipeline-Wirkung; getrennte Tabelle „Feste Kennungen“; Messziele nur noch im Prüfplan; `None` für Standardgerät und Hauptmonitor. |
| implementation-readiness-1-3 Zeitachse beim Kürzen | Ganzzahlige µs Anzeigezeit nach Edit-List, Ende ausschließlich; `frames.rs` mit Cache; `timeline.ts` und Test. |

### Stufe 9 – Major

| Befund | Änderung |
| --- | --- |
| 1-4 Export-Ordner | Standardordner zur Laufzeit über `FOLDERID_Videos`, nebeneinander; lokaler Ordner, wenn Videos in OneDrive liegt. |
| 1-5 fehlende Dateien, relative Pfade | Alle Einträge mit Pfad ab Repository-Wurzel; `Cargo.toml`/`lib.rs`/`build.rs`/Icons/`.cargo/config.toml`/`about.*`/vitest-Setup ergänzt; Abhängigkeitsgraph der Crates. |
| 1-6 Versionen | Tabelle mit Mindestversion, Features und Nutzer je Crate, exakte Versionen über eingechecktes `Cargo.lock`; npm-Liste; Zeittyp `time::OffsetDateTime` (UTC im Index, Ortszeit im Dateinamen). |
| 1-7 cargo-deny-Grenze | Ersetzt durch `cargo tree … -i windows-sys`/`-i windows`, plus Prüfung, dass `gameclip-ui` `windows` nicht direkt nennt. |
| 1-8 CI-Reihenfolge | Frontend vor `cargo build --release`; generierte Typen eingecheckt mit `git diff`-Prüfung; Linux-Job ohne Attrappe. |
| 1-9 Windows-Aufrufe in `gameclip-ui` | Pipe-Client, Start des Dienstes und Autostart nach `gameclip-ui-platform`; dort freie Funktionen in `cfg(windows)`-Modulen statt Trait (auch S8-N6). |
| 1-10 `clip://` | Siehe Autoren-Gate T8. |
| 1-11 `use_metadata_tags` | `movflags +use_metadata_tags` überall; vollständiger Tag-Satz; Spurtitel als `handler_name`; Rücklesetest `tags.rs`. |
| 1-12 Index-Schema | Vollständige DDL; `missing_count` statt Zeitstempel; Stern-Datei entfällt (S8-M2). |
| 1-13 Wettlauf Speichern/Watcher | Eintrag `pending` vor dem Umbenennen; Abgleich überspringt `pending` und `.part`. |
| 1-14 Speichern | Zeitlimit 500 ms, Teilfehler-Regeln, Fehlercodes, Schritte 1–9 geordnet. |
| 1-15 Cloud-Platzhalter | `placeholder` in `ClipInfo` und Index; 0 Bytes; nie gelesen oder gelöscht; Attribut vor jedem Lesen (auch S8-M3). |
| 1-16 gemeinsames Limit, `ClipInfo` | `effective_limit` = kleineres Limit; `ClipInfo` definiert; Auslöser des Löschens benannt. |
| 1-17 Ordnerwechsel | Clips im alten Ordner bleiben, zählen nicht, werden nie automatisch gelöscht. |
| 1-18 Ringpuffer | Regel aus dem Technischen Plan in `ring.rs`, `video_pipeline.rs` (`force_keyframe`) und Tests; `gap_before`; Regel für Pakete über Blockgrenzen und größer als ein Block; µs aus QPC, Zeitbasis 1/90 000. |
| 1-19 Spuren-Regel | „Spur existiert, wenn Quelle eingeschaltet; Stille, wenn sie nicht läuft“, überall gleich. |
| 1-20 Stufen und Prognose | Bitraten für alle Auflösungen und 30 fps; `forecast(&Settings, physical_ram_bytes)` mit `reserve_bytes` und `total_bytes` als Vergleichswert. |
| 1-21 Testwerte | Konkrete erwartete Werte (nachgerechnet); Testquelle mit Dauer-Parameter, kodierter Bildnummer und Rauschen. |
| 1-22 Größen-Export | `export_plan.rs` in Core mit Stufentabelle und Konstanten; `GetExportEstimate` für die Vorab-Warnung; ein Korrekturversuch, dann `TooLarge` (auch S8-N3). |
| 1-23 Worker-Vertrag | `output_id`; Strukturen in `gameclip-protocol::worker`; `SetGPUThreadPriority` in `main.rs`; `JobProgress.encoder`; `WorkerError`-Codes; Abbruch über Job-Objekt. |
| 1-24 Encoder-Profile | Getrennte Profile `Capture`/`Export`; `forced-idr`; QSV-Optionen in S2 gegen FFmpeg 7.1 prüfen; Kopie für x264 im Worker benannt. |
| 1-25 Skalier-Callback | Entfällt durch `gameclip-gpu::Converter`. |
| 1-26 Hotkey | `Hotkey { code, ctrl, alt, shift }`, VK-Tabelle `keymap.rs`; `TestRegister` prüft nur, `SetSettings` übernimmt; Hotkey wird beim Trennen wieder registriert. |
| 1-27 Tray-Symbole | Vorrang-Reihenfolge; `mic_paused` keine Einschränkung (auch S8-N4). |
| 1-28 Zustandsmaschine | `Event`/`Effect` aufgezählt; Standardregel für nicht aufgeführte Paare; fehlende Zeilen ergänzt; „Leerlauf“ entfällt; Wartezeiten 5 Werte. |
| 1-29 Verhalten ohne Datei | HDR (`hdr.rs`), Media Feature Pack (`media.rs`), Mikrofonpegel (`SubscribeMicLevel`/`MicLevel`), Autostart (`service.rs`), TaskId, Toast-Knopf über Protokoll `gameclip:` und `--reveal`. |
| 1-30 Tests und Abdeckung | Tests für `gameclipd` (Datei 55); Zeilenabdeckung auf stabiler Toolchain; `fmt` und Abdeckung in CI; Dateien den Meilensteinen zugeordnet. |
| 1-31 `panic=abort` | Gilt bewusst für den ganzen Workspace, mit Begründung. |
| 1-32 Log-Größe | Täglich rotierend, höchstens 7 Dateien; kein Größenlimit mehr versprochen. |
| 1-33 FFmpeg-Linken | Variablen in `.cargo/config.toml` `[env]` (Namen von `rusty_ffmpeg`, prüfen); `PATH` in CI; „per .gitignore ausgeschlossen“. |
| 1-34 Mutex und `--save-clip` | Argumente zuerst, dann Mutex; zweite Instanz endet still mit 0. |
| 1-35 Release/Signatur | `-NoSign` + SignPath-Action in CI, `-TestCert` lokal definiert; Konfigurationsdateien genannt. |
| codebase-details-1-1 Lizenzliste | Exakte SPDX-Liste inkl. `BSL-1.0`, `GPL-3.0-or-later`. |
| codebase-details-1-2 BtbN | Archiv im eigenen Repository gespiegelt; FFmpeg 7.1 fest; kein `scale_d3d11`. |
| codebase-details-1-3 BindFlags | `D3D11_BIND_RENDER_TARGET`, Einzeltexturen, Pool 16. |
| codebase-details-1-4 `FlushFileBuffers` | Datei nach `av_write_trailer` neu öffnen und `sync_all`; Format ausdrücklich `mp4`/`ipod`. |
| codebase-details-1-5 serde | `tag`+`content`; `id` als `u32` in einer Hülle; ts-rs-Export-Konfiguration; `UnknownCommand`. |
| codebase-details-1-6 Watcher-Überlauf | Überlauf → Abgleich; eigene Dateien über `.part`/`pending` ausgeschlossen. |
| codebase-details-1-7 Nachrichtenschleife | Tray und Hotkey auf demselben Nachrichten-Thread (`msgthread.rs`). |
| codebase-details-1-8 Manifest | Namespaces `rescap`/`uap`/`uap5`, `DeviceCapability`, StartupTask-Attribute, `MaxVersionTested`. |
| egm-style-1-1 deutsche Bezeichner | Alle Bezeichner englisch/ASCII. |
| egm-style-1-2 „vollständig getestet“ | Ersetzt durch 100 % Zeilenabdeckung für `retention.rs`. |
| egm-style-1-3 `GetMixFormat` | Als „in S4 zu prüfen“ gekennzeichnet. |

### Stufe 8 – zusätzlich zu den Gates

| Befund | Änderung |
| --- | --- |
| S8-M1 G1-Zeitplan | G1a (Ende M0, Durchstich S1 → Worker-Grundgerüst → Tray-Kopie) und G1b (nach M3). |
| S8-M2 Stern-Datei | Gestrichen. Unbekannte getaggte Clips werden automatisch mit Stern übernommen; `foreign` nur für Kopien. |
| S8-M3 OneDrive | Platzhalter 0 Bytes, nie gelesen/gelöscht; Warnung nur bei Auswahl; lokaler Standard, wenn Videos in OneDrive liegt. |
| Stufe 5 N4 Migrationen | Nur `schema_version` und Nur-Lesen-Regel in v1; Migrationen mit der ersten Schema-Änderung. |
| Stufe 5 N5 Protokollversion | Eine Version mit Gleichheitsprüfung statt Major/Minor. |
| Stufe 5 N9 Discord spät | Discord-Spur und Kriterium in M2. |
| S8-N1 Standardordner | Siehe 1-4. |
| S8-N2 `audio_offset_samples`/`tracks_path` | Beide Spalten entfallen; gemeinsames t0 per Edit-List; Pfad aus der UUID. |
| S8-N4 Zustandsmaschine | Siehe 1-28. |
| S8-N5 Verweise der Alternativen | AA, AC, AD, AE, AF als „Review Stufe 7“ gekennzeichnet; AB „Bestätigung offen“. |
| S8-N6 Publisher | R3: Wahl des Auswegs vor M5 endgültig. |
| S8-N7 E1 | Als Q17 aufgenommen. |
| S8-N8 Q16 | Für M0-Bibliotheken vor M0. |
| S8-N9 Warteschlange | `ListJobs`; Export-Dialog zeigt „wartet“. |

### Stufe 9 – Nits (angewendet)

Namensregeln (1-36), Dokumentstatus (1-37), Pipe-Details (1-38), Galerie-Details (1-39), Prognose-Beispiel ≈ 250 MB (cd-1-9), AAC-FIFO (cd-1-10), Loopback-Zeitbasis und ein API-Weg (cd-1-11), Taktgeber mit absoluten Zeiten (cd-1-12), Exit-Codes der Kommandozeile (cd-1-13), `drag`-Crate statt Plugin (cd-1-14), „wenn verfügbar“ entfernt (cd-1-15), 8 000 000 Bytes (cd-1-16), CI-Aufrufe (cd-1-17), Kleinigkeiten (cd-1-18), Pufferlänge 15–300 s auch in der Übersicht (cd-1-19), deutsche Feldbezeichnungen einheitlich (style-1-4/1-5), Einheiten (style-1-8/1-9), G1-Schwelle im Prüfplan (style-1-10), „vorläufig“ je Wert (style-1-11), Laufwerksstamm (style-1-12), CC0-1.0 (style-1-13), Encoder-Optionen ausgeschrieben (style-1-14), Wortwahl Mikrofon/±1 ms (style-1-15).

## DECLINE

| Befund | Grund |
| --- | --- |
| Stufe 5 N7 / S8 „zu viel Detail“ (Fenstergröße, Icon-Größen, Limiter-Werte) | Behalten, aber als *vorläufig* markiert; für einen Plan ohne offene Entscheidungen sind konkrete Startwerte nötig. |
| egm-style-1-6 genaue Abschnittsnamen in jedem Verweis | Verweise nennen jetzt den Unterabschnitt in Anführungszeichen; eine Verlinkung jeder Stelle bringt keinen Mehrwert für den Leser. |

## Nicht durch Lesen prüfbar (in Prototypen oder Q16 zu bestätigen)

`rusty_ffmpeg`-Variablennamen, QSV-Optionen in FFmpeg 7.1, Namespace der `graphicsCapture*`-Fähigkeiten (Q15), Toasts mit `AppListEntry="none"` (S1), Flatten eines adjazent getaggten serde-Enums (Test `roundtrip.rs`), exakte Crate-Versionen (Q16).
