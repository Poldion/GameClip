# Zoom-Out-Bericht – Stufe 5 (erster Zoom-Out)

*   **Dokument:** [docs/designs/gameclip.md](docs/designs/gameclip.md), ganzes Dokument
*   **Standard:** zoom-out-standard.md
*   **Kontext:** [concepts_brief.md](concepts_brief.md)
*   **Datum:** 2026-11-04
*   **Ergebnis:** 1 Blocker, 5 Major, 9 Nits

**Gesamturteil des Reviewers:** Die Architektur ist tragfähig und für diese Ebene schlank: zwei Prozesse, ein Puffer aus kodierten Paketen, eine reine Zustandsmaschine und eine reine Löschlogik im Core. Die Hauptprobleme sind ein Abbruchkriterium, das zum geplanten Zeitpunkt nicht messbar ist, und mehrere Eigenbauten, für die es ein Standardwerkzeug gibt.

---

## Blocker

### B1 – Das Abbruchkriterium G1 ist in Phase 0 nicht messbar

*   **Stelle:** Zeilen 40, 46–52, 693–696, 1258
*   **Problem:** G1 soll in Phase 0 gemessen werden, „bevor die Oberfläche gebaut wird“. Drei der vier Messwerte brauchen aber Teile, die es erst später gibt:
    *   Wert 1 (Installation bis erster Clip) braucht Installer und ersten Start (M2).
    *   Wert 3 (Anzahl Einstellungen) braucht die Einstellungsoberfläche (M2).
    *   Wert 4 (Hotkey bis Datei < 10 MB in der Zwischenablage) braucht Worker, Größen-Export und Zwischenablage (M3).

    Nur Wert 2 (RAM/CPU) lässt sich aus S1 messen. Da 2 von 4 Werten besser sein müssen, kann das Kriterium in M0 weder bestehen noch scheitern. Die wichtigste Risikokontrolle des Projekts greift damit nicht: Alles wird gebaut, bevor der Nutzen geprüft ist.
*   **Lösung (eine von beiden):**
    *   **(a) G1 teilen.** Phase 0 misst die Ausgangswerte der Wettbewerber und RAM/CPU von GameClip aus S1. Der vollständige 2-von-4-Vergleich wird ein eigenes Kriterium nach M3, vor M4/M5.
    *   **(b) Einen dünnen Durchstich in M0.** S1-Speichern per Hotkey, dann ein `ffmpeg.exe`-Aufruf mit Zielgröße, dann CF_HDROP über eine provisorische Tray-Aktion. So wird Wert 4, das behauptete Alleinstellungsmerkmal, vor jeder UI-Arbeit messbar.

    Der Reviewer hält (b) für am nächsten an der Absicht des Dokuments.

---

## Major

### M1 – Der eigene `gameclip-worker` baut nach, was `ffmpeg.exe` schon kann

*   **Stelle:** T4 (Zeile 142), Zeilen 279–317, Alternative Y (608–612), Dateien 30 und 56–57
*   **Problem:** Der Worker baut Dekodieren, genaues Springen, Neukodieren, GPU-Skalierung, Audio-Schnitt, Muxen mehrerer Spuren, Fortschrittsmeldungen und den Wiederholversuch bei der Größe selbst nach. Der BtbN-GPL-Build enthält `ffmpeg.exe` bereits. Die genannten Nachteile der Kommandozeile halten laut Reviewer nicht:
    *   **Encoder-Auswahl:** Der Dienst kennt das Prüfergebnis und kann `-c:v h264_nvenc` bzw. `libx264` direkt übergeben.
    *   **Fehlercodes:** Exit-Code plus letzte stderr-Zeile reichen für `JobFailed{grund}`.
    *   **Fortschritt:** `-progress pipe:1` liefert strukturierte Schlüssel-Wert-Zeilen.
    *   **Smart-Cut** ist ausdrücklich zurückgestellt (Q7). Ein Standardwerkzeug zu verwerfen, um eine spätere Funktion zu erleichtern, ist laut Reviewer YAGNI.
    *   Bildgenaues Schneiden (`-ss` nach `-i`), `-hwaccel d3d11va`, `-map` mit `-metadata:s:a:N title=` und `-disposition:a:0 default`, `-movflags +faststart`, `-b:v`/`-maxrate` gibt es alle als Optionen.

    Das Dokument räumt selbst „deutlich mehr eigener Code in M3“ ein.
*   **Schichtproblem:** Datei 30 lässt `gameclip-media::scale` einen VideoProcessor-Callback aus `gameclip-capture-win` entgegennehmen. Damit hängt der Worker indirekt am Aufnahme-Crate, und das nur wegen des Eigenbaus.
*   **Lösung:** Prozess-Isolation, Warteschlange und die Regel „nur vom Dienst gewählte Pfade“ bleiben. Statt des eigenen Workers erzeugt ein Job-Builder in `gameclipd` die Argumente für `ffmpeg.exe`; dieser Teil ist rein und im Core testbar. Den eigenen Worker gibt es erst, wenn S9 Smart-Cut rechtfertigt. Damit entfallen der größte Teil der Dateien 30 und 56 sowie der Callback über Crate-Grenzen.
*   **Hinweis:** Dies stellt die Autorenentscheidung T4 in Frage.

### M2 – Zu viel Abgleich-Mechanik für den Clip-Index in v1

*   **Stelle:** Zeilen 250, 271–277, Dateien 46 und 47
*   **Problem:** Das Design enthält:
    *   einen dauerhaften `ReadDirectoryChangesW`-Watcher auf zwei Ordnern
    *   eine 24-h-Frist für `missing` mit Zeitstempeln
    *   Umbenennungs-Verfolgung über Tags
    *   Auflösung doppelter IDs
    *   einen `foreign`-Status mit Übernahme-Ablauf
    *   den Neuaufbau aus Tags mit ausgesetztem Löschen

    Einzeln ist jeder Teil vernünftig, zusammen ergeben sie eine kleine Synchronisations-Engine. Laut Alternative X lässt sich nur der Stern nicht aus den Datei-Tags wiederherstellen. Eigentum und Verbindung zur Spurdatei stecken schon im Tag `gameclip_id`.
*   **Begründung:** Ein paar hundert MP4-Dateien sind in Millisekunden neu eingelesen. Ein dauerhafter Watcher optimiert eine Aktualität, die nichts verlangt. Die Galerie kann beim Öffnen neu einlesen, und vor dem Löschen wird der Tag ohnehin noch einmal gelesen.
*   **Lösung:**
    *   Abgleich nur bei Bedarf: beim Start, vor jedem Löschlauf und bei `ListClips`. `watcher.rs` entfällt in v1.
    *   Statt der 24-h-Frist gilt: Fehlt eine Datei beim Abgleich, werden Eintrag und `.m4a` beim nächsten Abgleich entfernt.
    *   `foreign` und „Übernehmen“ bleiben, weil sie für sicheres Löschen nötig sind.
*   **Nebenbemerkung (Nit):** Mit weniger Koordination ließe sich auch prüfen, ob SQLite mit WAL, einem Schreib-Thread und Migrationen nötig ist. Eine kleine JSON-Datei atomar neu zu schreiben, ist bei dieser Datenmenge billig. Der Ablehnungsgrund „wird komplett neu geschrieben“ ist ein Leistungsargument ohne Engpass.

### M3 – `gameclip-fake-daemon` bringt einen zweiten Transport und ein Crate, das v1 nicht braucht

*   **Stelle:** Zeilen 424, 755, Dateien 24, 67, Zeile 1244
*   **Problem:**
    *   Die Attrappe bringt einen eigenen Transport über Unix-Socket und Named Pipe (über `interprocess`), damit das Frontend unter Linux gegen sie getestet werden kann.
    *   Die Frontend-Tests in Datei 67 laufen aber gegen ein gemocktes `ipc.ts` und erreichen nie einen Socket.
    *   Die Rahmung ist schon in `roundtrip.rs` über `std::io::Read`/`Write` abgedeckt.
    *   Die Rust-Seite von Tauri lässt sich unter Linux ohnehin nicht bauen, weil `gameclip-ui-platform` nur eine Windows-Umsetzung hat.

    Die Attrappe testet also einen Transport, der in v1 nie ausgeliefert wird.
*   **Lösung:** Attrappe aus v1 streichen. Der Linux-CI-Job testet `gameclip-core`, `gameclip-protocol` und `npm test` mit gemocktem `ipc.ts`. Für Tests des Rust-IPC-Clients reichen Pipes im Speicher.

### M4 – Das eigene `clip://`-Protokoll baut Tauris Asset-Protokoll nach

*   **Stelle:** Zeile 395, Datei 60 (`clip_protocol.rs`)
*   **Problem:** Das Asset-Protokoll von Tauri 2 liefert lokale Dateien bereits mit HTTP-Range aus. Die erlaubten Pfade lassen sich zur Laufzeit über den Scope begrenzen. Das Design schreibt 206-Antworten und `Content-Range` selbst.
*   **Lösung:** Das Asset-Protokoll nutzen und jeden vom Dienst genannten Pfad zur Laufzeit in den Scope aufnehmen (oder den Scope auf die beiden Ordner begrenzen). In S5 das Spulen bestätigen. `clip://` nur, wenn S5 zeigt, dass das eingebaute Protokoll versagt.

### M5 – T1 (volles MSIX) ist festgelegt, bevor seine Voraussetzung geprüft ist

*   **Stelle:** Zeilen 139, 439, 450, Q2 (83)
*   **Problem:** In Zeile 439 steht noch „prüfen ob SignPath MSIX unterstützt“. Dabei muss MSIX signiert sein, S1 soll von Anfang an als MSIX laufen, und der Publisher hängt am Zertifikatssubjekt und darf sich nie ändern. Kann die kostenlose Open-Source-Stufe der SignPath Foundation kein MSIX signieren, oder ist ihr Zertifikatssubjekt ungeeignet, ist T1 eine Sackgasse. Das fiele erst auf, nachdem M0 darauf gebaut wurde.
*   **Lösung:** Eine ausdrückliche Prüfung vor S1, neben Q2: „Die SignPath Foundation signiert MSIX, und ihr Publisher-Subjekt ist bekannt.“ Dazu Alternative V (MSI plus Sparse Package) als benannter Ausweichweg, falls die Prüfung scheitert.

---

## Nits

| ID | Stelle | Problem | Lösung |
| --- | --- | --- | --- |
| N1 | Z. 232, 356, Datei 15, Datei 44 | Speichermodell widersprüchlich: „höchstens ein weiterer eingereiht“ (nacheinander) gegen „höchstens zwei gleichzeitig“ (parallel). Die Tabelle führt „Speichert“ als Zustand, Datei 15 sagt, es ist ein Zähler. | Tabelle und save.rs an das Zähler-Modell anpassen und nacheinander oder parallel festlegen. Nacheinander ist einfacher und hält p95 vorhersagbar. |
| N2 | Z. 62, 302, 315, Datei 56 | G8 verspricht „nie“ über der Zielgröße, der Plan hat aber nur einen Wiederholversuch. | Letzten Schritt ergänzen (nächste Auflösungsstufe oder klares `JobFailed`) oder G8 abschwächen. |
| N3 | Z. 378, 384, 249 | Der Standard-Export-Ordner liegt *im* Clip-Ordner, obwohl beide getrennte Limits haben sollen. | Festhalten, dass Scan und Limit nicht rekursiv sind, oder Ordner nebeneinander legen. |
| N4 | Z. 270, 380–383, Dateien 17, 46 | Migrations-Gerüst ohne eine einzige Migration. | Für v1 reichen `schema_version`, Nur-Lesen bei neuerem Schema und Erhalt unbekannter Felder. Gerüst mit der ersten Migration bauen. |
| N5 | Z. 335, 337, 344, Datei 22 | Major/Minor-Aushandlung und `GetCapabilities` für einen späteren Linux-Adapter, obwohl UI und Dienst immer im selben MSIX kommen. | Eine Protokollversion mit Gleichheitsprüfung. Capabilities mit dem Linux-Adapter. „Unbekannte Felder ignorieren“ bleibt. |
| N6 | Z. 420, Datei 58 | Der Trait `UiPlatform` hat nur eine echte Umsetzung, und die Grenze wird nicht durchgesetzt (cargo deny prüft nur Core und Protocol). | `cfg(windows)`-Modul statt Trait und Crate, oder `gameclip-ui` in die deny-Regel aufnehmen. |
| N7 | Dateien 27, 39, 40, 59, 66, 71 | Zu viel Detail: Encoder-Presets, Limiter-Werte, Discord-Abfrageintervall, Fenstergröße, 4-px-Raster, Icon-Größen. | Als „vorläufig“ markieren oder in `docs/test-protocols.md` bzw. in den Code verschieben. |
| N8 | Z. 228/839, 90–91, 211, 437 | Prognoseformel fest mit `3 × 128k`, auch wenn Discord aus ist. Q12/Q13 stehen als offen, sind aber beantwortet. Vorschlag P2 aus dem Brief ist dort nicht als bestätigt eingetragen, wird im Design aber als entschieden behandelt. `global-hotkey` und `windows-capture` sind weder genutzt noch begründet abgelehnt. | Spuranzahl aus den Einstellungen nehmen. Q12/Q13 schließen oder eingrenzen. P2 als bestätigt eintragen. Eine Zeile Begründung zu beiden Bibliotheken in den Alternativen. |
| N9 | Z. 213, 219, Datei 39, Q11 | Die Discord-Spur hat den höchsten Aufwand pro Nutzen. Sie ist eine Autorenentscheidung (D16) und wird nicht in Frage gestellt. | In der Reihenfolge ausdrücklich spät einplanen (nach stabilem M1). Das senkt das Risiko in M0/M1. |

---

## Was der Reviewer als passend bewertet

*   `transition()` und `plan_deletions()` sind reine Funktionen im Core. Die riskanteste Logik lässt sich ohne Hardware testen.
*   Die spekulative Adapter-Abstraktion für Puffer, Muxer und Prognose wurde abgelehnt.
*   Gespeichert wird nur der neueste Pufferabschnitt (einfacher als der Mehrdatei-Vorschlag aus dem Brief).
*   Die Oberfläche kommt ohne Router und ohne State-Bibliothek aus.
*   Der Worker läuft isoliert als eigener Prozess.
*   Befehle enthalten keine freien Pfade, nur Clip-IDs.
