# Zoom-Out-Bericht – Stufe 8 (zweiter Zoom-Out)

*   **Dokument:** [docs/designs/gameclip.md](docs/designs/gameclip.md), ganzes Dokument (Zeilen 1–1371)
*   **Standard:** zoom-out-standard.md · **Kontext:** [concepts_brief.md](concepts_brief.md), [zoom_out_report.md](zoom_out_report.md)
*   **Datum:** 2026-11-04
*   **Ergebnis:** 0 Blocker, 7 Major, 9 Nits
*   **Stand der 15 Befunde aus Stufe 5:**
    *   5 erledigt
    *   3 teilweise erledigt
    *   7 noch offen

---

## Teil A – Stand der Befunde aus Stufe 5

| Befund | Stand | Bemerkung |
| --- | --- | --- |
| B1 G1 in Phase 0 nicht messbar | teilweise, jetzt Major (S8-M1) | R10 verlegt die Entscheidung auf „vor M4“, aber M0 (Z. 804) enthält noch „Vergleich nach G1“ – zwei widersprüchliche Zeitpläne. |
| M1 eigener Worker statt `ffmpeg.exe` | offen (S8-M5) | Unverändert. Neu: S9 liegt in M0, der Worker kommt erst in M3 – S9 wird also praktisch mit `ffmpeg.exe` gebaut. Empfehlung: T4 nach S9 entscheiden. Stellt Autorenentscheidung T4 in Frage. |
| M2 Abgleich-Mechanik des Index | offen und gewachsen (S8-M4) | Watcher, 24-h-Frist, Umbenennen, doppelte IDs unverändert; dazu kamen Stern-Datei, OneDrive-Regeln, Stern bei Übernahme. |
| M3 `gameclip-fake-daemon` | offen (S8-M6) | Frontend-Tests laufen weiter gegen gemocktes `ipc.ts`; der Socket der Attrappe wird nie benutzt. |
| M4 eigenes `clip://` | offen (S8-M7) | Unverändert; Tauri-Asset-Protokoll mit Scope zur Laufzeit würde reichen. |
| M5 MSIX vor Prüfung der Signatur | erledigt | Q14 vor S1, R3 mit Auswegen, Alternative V benannt (Rest: S8-N6). |
| N1 Speichermodell | erledigt | durchgängig nacheinander mit Zähler. |
| N2 G8 „nie“ | erledigt | G8 endet mit `TooLarge` (Rest: S8-N3). |
| N3 Export-Ordner im Clip-Ordner | teilweise (S8-N1) | Text sagt „nebeneinander“, Standardwert-Tabelle Z. 786–787 noch verschachtelt und mit `%USERPROFILE%\Videos`. |
| N4 Migrations-Gerüst | offen | Dateien 17 und 46 unverändert. |
| N5 Protokoll-Versionen, Capabilities | offen | unverändert. |
| N6 `UiPlatform` und Grenze | offen und schlimmer | Z. 22 behauptet „nur über `gameclip-ui-platform`“, aber Datei 60 (`ipc_client.rs`) ruft Named Pipes, `GetNamedPipeServerProcessId`, `IApplicationActivationManager` und `StartupTask` direkt auf. Die Behauptung stimmt so nicht. |
| N7 zu viel Detail | offen | Presets, Limiter, Abfrageintervall, Fenstergröße, 4-px-Raster, Icon-Größen. |
| N8 Prognose, Q12/Q13, P2, Bibliotheken | größtenteils erledigt | Offen: Alternative AB steht als „Rejected by author“, obwohl P2 noch unbestätigt ist. |
| N9 Discord-Spur spät einplanen | offen | S4 in M0 verlangt sogar „Discord-Spur getrennt“ (Z. 1359). |

---

## Teil B – Befunde dieser Stufe

### Major

#### S8-M1 – Zwei widersprüchliche Zeitpläne für das Abbruchkriterium G1

*   **Stelle:** Z. 63, 69–73, R10 (723), M0 (804), Prüfplan (1366)
*   **Problem:** M0 enthält „Vergleich nach G1“, R10 verlegt die Entscheidung auf „vor M4“. In M0 brauchen die Werte 1, 3 und 4 weiterhin Installer, Einstellungen, Worker, Größen-Export und Zwischenablage. Vor M4 gemessen, greift G1 erst, wenn alles gebaut ist. Ein unklares Kriterium gibt falsche Sicherheit.
*   **Lösung:**
    *   G1 in zwei Teile trennen:
        *   **G1a in M0:** Ausgangswerte von Game Bar und OBS für alle vier Größen, dazu RAM/CPU von GameClip aus S1.
        *   **G1b nach M3, vor M4:** der vollständige 2-von-4-Vergleich.
    *   Besser noch: den dünnen Durchstich aus Stufe 5 übernehmen (S1-Speichern → ein `ffmpeg.exe`-Aufruf mit Zielgröße → CF_HDROP über das Tray). Damit wird Wert 4 schon in M0 messbar, und S9 braucht ohnehin einen Export-Weg in M0.

#### S8-M2 – Die Stern-Sicherung `.gameclip-stars.json` hat keine Wirkung

*   **Stelle:** Z. 302, 307, 309, 417, 491, 694, 744, R5 (718)
*   **Problem:** Übernommene Clips bekommen ohnehin einen Stern, und wiederhergestellte Clips sind bis zur Übernahme `foreign`, also nie löschbar. Mit und ohne Datei ist das Ergebnis dasselbe: alles geschützt.

    Dafür bringt die Datei Kosten mit:
    *   eine zweite Quelle für Sterne ohne Regel, welche bei Abweichung gilt
    *   eine versteckte Datei in sichtbaren Ordnern, die mitsynchronisiert und mitgeteilt wird
    *   Reste beim Ordnerwechsel
    *   eine gemeinsame Datei, wenn Clip- und Export-Ordner derselbe Ordner sind
*   **Lösung:**
    *   `.gameclip-stars.json` streichen.
    *   Optional: Bei Neuaufbau oder Neuinstallation alle getaggten Dateien in den eingestellten Ordnern automatisch mit Stern übernehmen. Das bietet dieselbe Sicherheit, ohne hunderte Klicks auf „Übernehmen“ (`AdoptClip` nimmt nur eine ID). `foreign` bleibt dann nur für kopierte Dateien mit doppelter ID.
    *   Sollen Clips ohne Stern nach einer Neuinstallation löschbar bleiben, muss die JSON-Datei die *einzige* Quelle für Sterne werden, also ohne Spalte `starred`.

#### S8-M3 – Die OneDrive-Regel verursacht falsches Löschverhalten

*   **Stelle:** Z. 279, R6 (719), Z. 419, Dateien 19/48
*   **Probleme:**
    1.  **Falsches Löschen.** Platzhalter zählen mit ihrer Größe zum Limit, werden aber nie gelöscht. OneDrive lagert die ältesten Dateien zuerst aus. Diese Platzhalter füllen das Limit, und `plan_deletions` löscht dann die *neuesten* lokalen Clips oder warnt ohne Ende. Dabei belegen Platzhalter keinen lokalen Platz, und genau den schützt G9.
    2.  **Lücken in den Verträgen.** `plan_deletions` (Datei 19) kennt keine Platzhalter. `retention_exec` (Datei 48) liest vor jedem Löschen den Tag, was das Herunterladen auslöst, das Z. 279 verbietet. Dasselbe gilt für Abgleich und Neuaufbau.
    3.  **Warnung beim ersten Start.** Viele Nutzer haben einen umgeleiteten Videos-Ordner und bekämen die Warnung schon beim ersten Start. Das widerspricht G10. Außerdem setzt die Standardwert-Tabelle `%USERPROFILE%\Videos` fest.
    4.  **Watcher.** Er bekommt Ereignisse aus der Synchronisation.
*   **Lösung (einfachste zuerst):**
    *   Platzhalter zählen mit 0 Bytes, sind von `plan_deletions` und von jedem Tag-Lesen ausgenommen. Erkannt werden sie nur über `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`, ohne die `%OneDrive%`-Pfadregel.
    *   Oder für v1: einmal warnen, wenn ein Cloud-Ordner gewählt wird, und den Standard-Clip-Ordner nur dann unter `<Videos>` legen, wenn er nicht in der Cloud liegt, sonst einen lokalen Pfad vorschlagen.
    *   In beiden Fällen Platzhalter ausdrücklich in die Eingaben der Dateien 19 und 48 aufnehmen.

#### S8-M4 – Abgleich-Mechanik des Index (aus Stufe 5 M2, gewachsen)

*   **Stelle:** Z. 303–309, Dateien 46/47
*   **Problem:** Gemessen am Nutzen ist das jetzt das aufwendigste Teilsystem.
*   **Lösung:**
    *   Abgleich nur bei Bedarf: beim Start, vor dem Löschen und bei `ListClips`.
    *   `watcher.rs` und die 24-h-Frist entfallen.
    *   SQLite mit WAL, Schreib-Thread und Migrationen erneut gegen eine atomar geschriebene JSON-Datei abwägen.

#### S8-M5 – Eigener Worker (aus Stufe 5 M1, offen)

Größter vermeidbarer Code-Block in M3. S9 in M0 ist der natürliche Zeitpunkt, um T4 mit Messdaten zu entscheiden.

#### S8-M6 – Attrappe des Dienstes (aus Stufe 5 M3, offen)

#### S8-M7 – `clip://` (aus Stufe 5 M4, offen)

### Nits

| ID | Stelle | Problem | Lösung |
| --- | --- | --- | --- |
| S8-N1 | Z. 416, 424 vs. 786–787 | Standardwert-Tabelle verschachtelt den Export-Ordner und nutzt `%USERPROFILE%\Videos`. | Tabelle an den Text angleichen. |
| S8-N2 | Z. 266, 296–297, 309, Datei 44 | `audio_offset_samples` ist neben gemeinsamem t0 mit Edit-Lists überflüssig; der Neuaufbau stellt weder ihn noch `tracks_path` wieder her. `tracks_path` folgt ohnehin aus der UUID. | Beide Spalten streichen; falls ein Versatz nötig ist, in den MP4-Tag schreiben. |
| S8-N3 | Z. 338, Datei 56 | Die Wiederholkette kann im schlimmsten Fall 9–12 vollständige Neukodierungen eines 300-s-Clips bedeuten; zwei Schwellen (0,05/0,03) an zwei Stellen. | Auflösung einmal vorab aus der Schwelle wählen, höchstens ein Korrekturversuch, dann `TooLarge`; Schwellen als *vorläufig* in die Standardwerte. |
| S8-N4 | Z. 373–405 | Zustandsmaschine: „Leerlauf“ ist praktisch nur Start- und Endpunkt; die Regel „Exporte außer in Leerlauf“ wirkt nicht; `mic_paused` als Einschränkung zeigt „Eingeschränkt“ für eine bewusste Aktion; Vorrang der Tray-Symbole fehlt; „beenden“ vs. „abschließen“; Wartezeiten sind Umsetzungsdetail. | „Leerlauf“ streichen oder in einem Satz definieren, `mic_paused` aus „Eingeschränkt“ nehmen, überall „abschließen“. |
| S8-N5 | Z. 578, 584, 596, 628, 656 | Neue Alternativen zitieren unpassende D-IDs (AA/AE → D13, AC → D2, AF → D24) und stehen als „Rejected by author“, obwohl sie aus der Review stammen; AB trotz offener Bestätigung. | Verweise korrigieren oder als „in der Review vorgeschlagen, Bestätigung durch den Autor offen“ kennzeichnen. |
| S8-N6 | R3 (716), Q2 (105) | Azure Trusted Signing ändert das Zertifikatssubjekt und damit den unveränderlichen Publisher. | In R3 festhalten: Die Wahl des Auswegs ist vor dem ersten öffentlichen Release (M5) endgültig. |
| S8-N7 | R13 (726) | Verweis auf „E1“, das im Dokument nicht definiert ist; automatische Pausen fehlen in den Open Questions. | Als Q17 aufnehmen oder entfernen. |
| S8-N8 | Q16 (114) | Lizenzprüfung „bis M1“, aber `rsmpeg`, `wasapi`, `windows` werden schon in den M0-Prototypen genutzt. | Für diese Bibliotheken vor M0 prüfen. |
| S8-N9 | R11 (724) | Versprochene sichtbare Warteschlange gibt es in keiner Komponente. | Zusage streichen oder eine Zeile in `ExportDialog`/`StatusBar`. |

---

## Teil C – Bewertung der Ergänzungen aus Stufe 6/7

*   **Gut, ohne unnötige Komplexität:**
    *   Begriffe
    *   Q14/Q15 als Prüfungen vor S1
    *   die überarbeitete Zustandsmaschine (bis auf S8-N4)
    *   R1–R10 und R12
    *   die Alternativen AA, AC, AD, AE, AG (bis auf die Verweise in S8-N5)
*   **Bringen Komplexität oder neue Probleme:**
    *   `.gameclip-stars.json` (S8-M2)
    *   die OneDrive-Platzhalter-Regel (S8-M3)
    *   der Neuaufbau ohne Versatz- und Spurpfad-Felder (S8-N2)
    *   die Wiederholkette beim Größen-Export (S8-N3)
    *   Verweise auf nicht vorhandene Funktionen oder IDs (S8-N7, S8-N9)
*   **Weiterhin gut:**
    *   reine Core-Funktionen
    *   abgelehnte Adapter-Abstraktion
    *   nur den neuesten Abschnitt speichern
    *   Frontend ohne Router und ohne State-Bibliothek
    *   isolierter Worker
    *   nur `clip_id`s statt freier Pfade
