Status: resolved (Auto accept: 51 PROPOSE FIX angewendet; E1 und E2 offen; 3 DECLINE)

# Goldfish Annotated Review Report

**Target:** `docs/designs/gameclip.md` (Zeilen 1–653, FeatureRange) · **Quelle:** [critic_report.md](critic_report.md) (Stufe 7)

**Übersicht der Einstufung:** 2 ESCALATE · 51 PROPOSE FIX · 0 CONTEST · 3 DECLINE

> Hinweis: Einige Befunde überschneiden sich mit Stufe 5 ([zoom_out_report.md](zoom_out_report.md)) und Stufe 6 ([comprehension_report.md](comprehension_report.md)). Wo das so ist, löst die vorgeschlagene Änderung den Befund aus Stufe 5 bzw. 6 mit.

## Prioritized Findings & AI Suggestions

---

## ESCALATE (brauchen deine Entscheidung)

### [E1] [`egm-design`] design-1-9 – Severity: Major
*   **Affected Scope:** [Zeilen 12, 54–57, 446](docs/designs/gameclip.md#L12-L57) (Overview, G3, Paket)
*   **Discrepancy Description:** Bildschirm und Mikrofon werden ab Login dauerhaft aufgenommen. Für Akku und Energiesparmodus, Sperren und Standby sowie sensible Bildschirminhalte außerhalb von Spielen gibt es keine Regel. GPU-Last und Stromverbrauch werden nicht gemessen.
*   **Rationale:** Eine Regel festlegen: Pause bei Akku/Energiesparen, Pause beim Sperren, optional „nur bei Vollbild-App“, dazu eine GPU-/Strom-Messung. Mindestens als bekannte Grenze aufführen.
*   ***AI Suggestion:*** ESCALATE
*   ***Justification:*** Das ist eine Produktentscheidung, die im Brief nie getroffen wurde. Ein „nur bei Vollbild“-Modus würde z. B. das Speichern aus randlosen Fenstern verhindern. Der Befund ist berechtigt, aber nur du kannst entscheiden, welche Pausen-Regeln gelten.
*   ***Proposed Edit:*** abhängig von deiner Antwort. Vorschlag zur Auswahl:
    *   (a) Pause beim Sperren des PCs (immer), sonst keine automatische Pause; GPU-/Stromverbrauch wird in S1 gemessen.
    *   (b) wie (a), zusätzlich Pause im Akkubetrieb (abschaltbar).
    *   (c) wie (b), zusätzlich Option „nur aufnehmen, während eine Vollbild-App läuft“ (standardmäßig aus).

### [E2] [`egm-structural`] Teil von structural-1-4 – Bestätigung von P2 (Exclude-Self-Loopback)
*   **Affected Scope:** [Zeile 211](docs/designs/gameclip.md#L211) (Audio-Pipeline)
*   **Discrepancy Description:** Im Brief steht P2 („System-Ton ohne GameClip selbst“) noch als Vorschlag, den der Autor bestätigen muss. Das Design behandelt ihn als entschieden.
*   **Rationale:** Bestätigung festhalten oder Alternative abwägen.
*   ***AI Suggestion:*** ESCALATE
*   ***Justification:*** Im Gespräch hast du gefragt, was P2 bedeutet. Ich habe erklärt, dass damit nur gemeint ist, dass GameClips eigener Bestätigungston nicht im nächsten Clip landet. Eine ausdrückliche Zustimmung gab es danach nicht. Die Reviewer können das Gespräch nicht sehen.
*   ***Proposed Edit:*** Bei Bestätigung: im Brief P2 als D26 eintragen, im Design bei Zeile 211 „(D26)“ ergänzen und Alternative „normales Endpoint-Loopback“ mit Begründung aufnehmen (siehe F-structural-1-4).

---

## PROPOSE FIX – Blocker

### [1] [`egm-structural`] structural-1-1 – Severity: Blocker
*   **Affected Scope:** ganzer Bereich; Risiken verstreut in Z. 40, 306, 450, 624–634
*   **Discrepancy Description:** Kein Abschnitt „Risks & Mitigations“, den der Feature-Spec-Standard verlangt.
*   **Rationale:** Tabelle Risiko | Art | Auswirkung | Gegenmaßnahme | Verweis einfügen.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Berechtigt. Die Risiken sind im Text verteilt und ohne Gegenmaßnahme oder Abbruchkriterium gesammelt nicht auffindbar.
*   ***Proposed Edit:***
    *   Location: neuer Abschnitt `## Risks & Mitigations` vor „Non-Functional Requirements“.
    *   Change: Tabelle mit mindestens diesen Zeilen: R1 Vanguard sperrt oder meldet (S1, Abbruch des Projekts bei Nicht-Bestehen); R2 exklusives Vollbild liefert kein Bild (S1/Q4, Ausweg DDA); R3 SignPath kann kein MSIX signieren (Prüfung vor S1, Ausweg Alternative V oder Azure Trusted Signing); R4 Export stört das laufende Spiel (S9 mit PresentMon); R5 Index-Verlust nimmt Sterne mit (Schutz siehe Fix 11); R6 OneDrive-Ordner (Fix 15); R7 MSIX-Deinstallation löscht Einzelspuren und Index (Hinweis in der Deinstallation); R8 NVENC-Sitzungslimit (Ausweichen auf x264); R9 RAM-Obergrenze 25 % begrenzt Pufferlänge (Warnung in Einstellungen); R10 G1 verfehlt (Umfang überdenken); R11 ein Export gleichzeitig (Warteschlange sichtbar); R12 IPC-Höchstgröße 8 MB (`GetFrameTimes` für 5 min ≈ 18 000 Werte ≈ 300 KB, weit darunter).

---

## PROPOSE FIX – Major

### [2] [`egm-structural`] structural-1-2
*   **Affected Scope:** Z. 460–618 (alle 26 Alternativen)
*   **Discrepancy Description:** „Rejected by author“ ohne Verweis auf die Entscheidung.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Das Entscheidungsregister im Brief ist die Quelle. Der Verweis macht die Ablehnung prüfbar.
*   ***Proposed Edit:*** Jede Begründung beginnt mit „Rejected by author ([D-ID](../../concepts_brief.md#entscheidungsregister)):“, Zuordnung A→D6, B→D6, C→D9, D→D11, E/H→D3, F→D1, G→Non-Goal Brief, I/J→D2, K→D4, L→D12, M/N/O→D7, P→Non-Goal Brief, Q/R→D10, S→D24, T→D5, U→D8, V–Z→T1–T5. „nach Review“ wird zu „(vorgeschlagen in Review-Runde 1)“, damit kein Sprachmix bleibt (löst auch style-1-21).

### [3] [`egm-structural`] structural-1-3
*   **Affected Scope:** Z. 478, 496, 516, 522, 536, 542, 548, 562, 568, 588
*   **Discrepancy Description:** Begründungen wiederholen die Entscheidung, statt die Abwägung zu nennen.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Umformulieren, z. B.:
    *   D: „Ohne Galerie und Kürzen fehlt der Ablauf, der laut G1 der Vorteil gegenüber Game Bar ist; der doppelte Umfang wird dafür in Kauf genommen.“
    *   G: „Im Spielmodus verwaltet Steam die Eingaben, globale Hotkeys funktionieren dort nicht zuverlässig; der Aufwand steht in v1 in keinem Verhältnis.“
    *   J: „Ein 5-min-Ringpuffer bei 14 Mbit/s schreibt pro Spielstunde ~6 GB auf die SSD; dafür sinkt der RAM-Bedarf. v1 begrenzt stattdessen die Länge.“
    *   K: „Ohne passende GPU gäbe es gar keine Aufnahme; höhere CPU-Last mit Warnung ist das kleinere Übel.“
    *   M/N: „Einzelspuren kosten ~1 MB RAM je Spur und Minute; dafür ist getrenntes Exportieren jederzeit möglich, ohne vorher daran zu denken.“
    *   O: „Mehrspurige Dateien spielen auf Handys und in WhatsApp nur die erste Spur und sind größer; die interne `.m4a` kostet dafür eine Zuordnung über den Index.“
    *   Q: „Ein Modifier verhindert Kollisionen mit Spieltasten, ist im Spiel aber umständlicher; da der Hotkey frei belegbar ist, bleibt F8.“
    *   R: „Hooks lassen die Taste ans Spiel durch, wirken aber für Anti-Cheat wie ein Keylogger und sehen nichts, wenn ein Spiel mit Admin-Rechten den Fokus hat.“
    *   U: „Ein Prozess spart IPC, aber ein Hänger der WebView stoppt den Puffer; der Puffer hat Vorrang.“

### [4] [`egm-structural`] structural-1-4
*   **Affected Scope:** Z. 192–205, 211, 279–318, 410–425, 436, Alternativen
*   **Discrepancy Description:** Tragende Entscheidungen ohne Alternative.
*   ***AI Suggestion:*** PROPOSE FIX (Teil P2 siehe E2)
*   ***Proposed Edit:*** Neue Gruppe „Weitere technische Alternativen“ mit:
    *   **AA – DDA statt WGC als Hauptweg:** liefert auch bei manchen Vollbild-Modi Bilder, aber nur ganze Monitore, kein Mauszeiger-Schalter, gelber Rahmen entfällt erst gar nicht nötig; WGC ist unter Windows 11 die vorgesehene Schnittstelle. DDA bleibt Ausweg, wenn S1 es nötig macht.
    *   **AB – Normales Endpoint-Loopback statt Prozess-Loopback:** einfacher und ohne PID, nimmt aber GameClips eigenen Ton und bei aktiver Discord-Spur Discord doppelt auf.
    *   **AC – HEVC oder AV1 statt H.264:** bei 10 MB sichtbar bessere Qualität, aber Wiedergabe und Hardware-Encoding nicht auf allen Zielgeräten (ältere Handys, WhatsApp-Vorschau, Discord-Einbettung im Browser je nach Codec, prüfen); H.264 ist überall abspielbar. AV1-Export als spätere Option.
    *   **AD – Hersteller-SDKs (NVENC SDK, AMF, oneVPL) oder Media Foundation direkt statt FFmpeg:** weniger Abhängigkeit, aber drei bis vier getrennte Anbindungen und ein eigener Muxer; FFmpeg deckt alle mit einer Schnittstelle ab.
    *   **AE – Variable Bildrate statt festem 60-fps-Takt:** spart Bitrate bei Standbildern, macht aber RAM-Prognose, A/V-Abgleich und bildgenaues Schneiden unzuverlässig.
    *   **AF – NTFS-Alternate-Data-Stream oder Begleitdatei statt MP4-Tag als Eigentumsmerkmal:** ADS geht beim Kopieren auf FAT/exFAT, in Cloud-Ordnern und beim Upload verloren; eine Begleitdatei verdoppelt die Dateien im Ordner. Der MP4-Tag reist mit der Datei.
    *   **AG – Plattformneutrale Adapter-Abstraktion für Puffer, Muxer und Prognose** (aus dem Brief übernommen): gpu-screen-recorder verwaltet seinen Puffer unter Linux selbst, die Abstraktion würde nie wiederverwendet.

### [5] [`egm-structural`] structural-1-5
*   **Affected Scope:** Z. 89–91
*   **Discrepancy Description:** Q11–Q13 sind im Text bereits beantwortet.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:* **Q11 umformulieren auf das, was offen ist: „Erlaubt Windows mehrere ausgeschlossene Prozessbäume pro Loopback-Strom? Bis zur Antwort aus S4 gilt die Stummschaltung (siehe Audio-Pipeline).“ Q12 und Q13 in eine Liste „Beantwortet im Entwurf“ verschieben, mit Verweis auf „Speicherlimit und Löschen“ und „Clip-Index“. Hinweis „Q8–Q10 im Brief erledigt (D7, D21, D22)“ ergänzen (löst structural-1-9).

### [6] [`egm-structural`] structural-1-6 (auch comprehension #3, #4)
*   **Affected Scope:** Z. 347–367
*   **Discrepancy Description:** Fehlende Übergänge, „Eingeschränkt“ bündelt unabhängige Ursachen, Neukonfiguration bei jeder Einstellung.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Zustandsmaschine neu fassen:
    *   Hauptzustände: Leerlauf, Startet, Puffert, Pausiert, Konfiguriert neu, Erholt sich, Fehler.
    *   Einschränkungen als unabhängige Flags: `hotkey_inactive`, `cpu_encoder`, `hdr_active`, `mic_paused`. Das Tray zeigt „Eingeschränkt“, wenn mindestens ein Flag gesetzt ist.
    *   Speichern ist kein Zustand, sondern ein Zähler; Speichervorgänge laufen **nacheinander**, höchstens einer wartet (löst zoom-out N1 und comprehension #3).
    *   Neue Zeilen: Fehler → Startet (Nutzer wählt „Neu starten“ im Tray oder ändert eine Einstellung); `hotkey_inactive` fällt weg bei erfolgreichem `TestRegister`/neuer Taste; beliebig → Pausiert über `StopBuffer`, Pausiert → Startet über `StartBuffer`; beliebig → Leerlauf bei „Beenden“; Geräteverlust während Speichern: laufender Speichervorgang wird mit den bereits gehaltenen Blöcken beendet, dann Erholt sich; Platte voll beim Speichern bleibt `SaveFailed`, der Zustand ändert sich nicht.
    *   Neukonfiguration nur bei Pufferlänge, Auflösung, Bildrate, Qualitätsstufe, Monitor und Audioquellen.

### [7] [`egm-structural`] structural-1-7
*   **Affected Scope:** Z. 338–342
*   **Discrepancy Description:** Keine Befehle zum Starten/Stoppen des Puffers; Zusammenlegung von Trim- und Export-Auftrag nicht erklärt.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** `StartBuffer` und `StopBuffer` ergänzen (Tray-Menüpunkt „Aufnahme pausieren“). Satz ergänzen: „Kürzen und Größen-Export sind ein Befehl `StartExportJob` mit Profil, weil beide denselben Ablauf im Worker nutzen.“

### [8] [`egm-design`] design-1-1
*   **Affected Scope:** Z. 227, 232
*   **Discrepancy Description:** Neueste Pakete mitten in einer Bildgruppe verwerfen macht alle folgenden Bilder bis zum nächsten Keyframe unbrauchbar.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Technisch zutreffend.
*   ***Proposed Edit:*** Ersetzen durch: „Ist die Reserve erschöpft, verwirft der Encoder alle Pakete bis zum nächsten Keyframe und fordert sofort einen neuen Keyframe an (IDR). Audio im selben Zeitraum wird ebenfalls verworfen. Der Index markiert die Lücke; ein Clip über eine Lücke wird an der Lücke getrennt wie bei einem neuen Abschnitt. Die Reserve wird für den schlimmsten Fall bemessen: ein gehaltener Speichervorgang über den vollen Puffer plus die Dauer eines Speichervorgangs auf HDD (Annahme 30 s). Ein Test in `ring_tests.rs` prüft das.“ „wird nie beschädigt“ entfällt (löst style-1-11).

### [9] [`egm-design`] design-1-2
*   **Affected Scope:** Z. 56, 224, 228
*   **Discrepancy Description:** G3 (±5 %) widerspricht der 15-%-Reserve.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Die Anzeige zeigt den reservierten Speicher einschließlich Reserve. G3: „Der Puffer belegt höchstens den angezeigten Wert (Prognose plus Reserve), gemessen als Private Bytes des Dienstes.“ Die drei Zuschläge werden erklärt: +1 s für die halbe Bildgruppe am Anfang, ×1,05 für Container-Overhead der Pakete, +15 % Reserve für gehaltene Schnappschüsse; angezeigt wird die Summe (löst comprehension #11).

### [10] [`egm-design`] design-1-3
*   **Affected Scope:** Z. 234–237, 300, 309
*   **Discrepancy Description:** Zeitbezug zwischen MP4 und `.m4a` undefiniert.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** „Beide Dateien nutzen denselben Nullpunkt t0 = PTS des ersten Keyframes. Audio vor t0 wird verworfen, die AAC-Anlaufverzögerung wird in beiden Dateien als Edit-List geschrieben. Zusätzlich speichert der Index `audio_offset_samples`. S4 prüft: Mischung aus der MP4 und Summe der Einzelspuren aus der `.m4a` liegen auf ±1 Sample übereinander.“

### [11] [`egm-design`] design-1-4
*   **Affected Scope:** Z. 246, 275, 277, 449, 606
*   **Discrepancy Description:** Nach Index-Verlust gehen Sterne verloren, übernommene Clips können gelöscht werden; Z. 277 widerspricht sich.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Berechtigt und ein echtes Datenverlust-Risiko. SQLite bleibt (T3), aber die Sterne brauchen eine zweite Kopie außerhalb des Paketordners.
*   ***Proposed Edit:*** (1) Übernommene Clips sind standardmäßig **geschützt** (Stern an), der Nutzer entfernt den Stern selbst. (2) Der Dienst schreibt nach jeder Stern-Änderung eine versteckte Datei `.gameclip-stars.json` (Liste der `gameclip_id`s mit Stern) in den jeweiligen Clip- bzw. Export-Ordner. Beim Neuaufbau werden die Sterne daraus wiederhergestellt. (3) Z. 277 klarstellen: „Der Neuaufbau stellt `id`, `path`, `size`, `created_at`, `duration` aus den Tags und `starred` aus `.gameclip-stars.json` wieder her; die Clips gelten als `foreign`, bis der Nutzer sie übernimmt; Löschen ist bis dahin ausgesetzt.“ Alternative X wird um diesen Punkt ergänzt.

### [12] [`egm-design`] design-1-5
*   **Affected Scope:** Z. 38, 238
*   **Discrepancy Description:** `QueryFullProcessImageName` öffnet ein Handle auf den Spielprozess.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:*** Berechtigt und wichtig für G4.
*   ***Proposed Edit:*** „Die PID kommt von `GetWindowThreadProcessId`, der Programmname aus einem `CreateToolhelp32Snapshot` (Eintrag `szExeFile`). Es wird kein Handle auf den Spielprozess geöffnet.“ Dasselbe gilt für die Discord-Suche. In die Regeln zu Vanguard aufnehmen: „Kein `OpenProcess` auf fremde Prozesse.“

### [13] [`egm-design`] design-1-6
*   **Affected Scope:** Z. 111, 285, 297–306
*   **Discrepancy Description:** Der Export konkurriert mit dem Spiel um die GPU, ohne Kriterium.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** S9 erhält das Kriterium „Bildrate des Spiels sinkt während eines Exports um höchstens 10 % (PresentMon, vorläufig)“. Der Worker setzt `IDXGIDevice::SetGPUThreadPriority` auf den niedrigsten erlaubten Wert. Kein automatisches Aufschieben in v1; falls S9 scheitert, wird „Export erst nach dem Spiel“ als Option nachgerüstet. Eintrag R4 in der Risikotabelle.

### [14] [`egm-design`] design-1-7 (auch zoom-out M5, style-1-6)
*   **Affected Scope:** Z. 83, 439, 444–450, 594, 629
*   **Discrepancy Description:** MSIX hängt an einer ungeprüften Signatur, kein Plan B; fremde FFmpeg-DLLs und SignPath-Regeln.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Neue Open Question **Q14 – Signatur (Prüfung vor S1):** „Signiert die SignPath Foundation MSIX-Pakete, die fremde Binärdateien (BtbN-FFmpeg) enthalten? Wie lautet das Zertifikatssubjekt (= Publisher)?“ Ausweg in dieser Reihenfolge: Azure Trusted Signing (kostenpflichtig, MSIX-fähig), dann Alternative V (MSI plus Sparse Package). Zusätzlich: FFmpeg wird in CI selbst aus der festen Quellversion gebaut, falls SignPath vorgebaute Binärdateien ablehnt. Eintrag R3 in der Risikotabelle.

### [15] [`egm-design`] design-1-8
*   **Affected Scope:** Z. 245, 272–274, 377–378
*   **Discrepancy Description:** OneDrive-Umleitung und Cloud-Platzhalter nicht bedacht.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Standardordner über `SHGetKnownFolderPath(FOLDERID_Videos)`. Liegt der gewählte Ordner in einem OneDrive- bzw. Cloud-Ordner (Attribute `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`/`OFFLINE` oder Pfad unter `%OneDrive%`), zeigt die App beim Auswählen eine Warnung („Gelöschte Clips werden auch in der Cloud gelöscht; Speichern kann langsamer sein“). Beim Abgleich und beim Löschen werden Platzhalter (Datei nicht lokal) **nie** gelesen oder automatisch gelöscht; sie zählen zum Limit nur mit der im Index gespeicherten Größe. Eintrag R6.

### [16] [`egm-design`] design-1-10
*   **Affected Scope:** Z. 449, 635–639
*   **Discrepancy Description:** Rückweg über ältere MSIX-Version funktioniert nicht wie beschrieben.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** „Rückweg und Einführung“ ersetzen: „MSIX erlaubt standardmäßig kein Downgrade. Ein Rückweg bedeutet Deinstallieren und ältere Version installieren; dabei werden Einstellungen, Index und Einzelspuren gelöscht, die Clips bleiben. Die Sterne werden aus `.gameclip-stars.json` wiederhergestellt (Fix 11). Trifft eine ältere Version dennoch auf ein neueres Schema (Einstellungen oder Index), öffnet sie beides nur lesend und setzt das automatische Löschen aus.“

### [17] [`egm-style`] style-1-1
*   **Affected Scope:** Z. 40, 46, 52
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Titel G1: „Besser als Game Bar in mindestens 2 von 4 Messgrößen“. Je Messgröße die Richtung (kleiner ist besser bei 1, 2, 3, 4) und „besser“ = mindestens 10 % Abstand (vorläufig, Q1). Z. 40: „Ob GameClip messbar besser ist, …“. Zeitpunkt der Messung siehe zoom-out B1 (wird in Stufe 8 erneut geprüft).

### [18] [`egm-style`] style-1-2
*   **Affected Scope:** Z. 33, 34, 36, 38
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Prüfbare Aussagen: Game Bar „bietet Kürzen nur über die Fotos-App und keinen Export auf eine Zielgröße“; OBS „braucht für den Replay Buffer eigene Einstellungen zu Ausgabe, Encoder und Hotkey und hat keinen Editor“; „heute braucht man dafür meist zwei oder drei Programme“ → „ohne GameClip braucht man dafür neben dem Rekorder einen Editor und ein Werkzeug zum Verkleinern“; „Viele Spieler spielen Valorant“ → „Valorant ist ein Zielspiel des Autors“.

### [19] [`egm-style`] style-1-3 (auch comprehension? zoom-out N2)
*   **Affected Scope:** Z. 62, 111, 302, 312, 315
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** G8: „Der Größen-Export liefert eine Datei unter der Zielgröße oder bricht mit klarer Meldung ab. Abnahme: 20 Testclips von 5 bis 300 s, keiner über der Zielgröße.“ Ablauf am Ende: bis zu drei Versuche, ab dem zweiten mit der gemessenen Überschreitung als Korrekturfaktor; danach nächste Auflösungsstufe; reicht auch 480p30 nicht, `JobFailed{code: TooLarge}`. Bei weniger als 0,03 Bit pro Pixel und Bild zeigt der Dialog vorab „Qualität wird sehr niedrig“. MB = 1 000 000 Bytes (löst Teil von style-1-22).

### [20] [`egm-style`] style-1-4
*   **Affected Scope:** Z. 59, 117
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** G5: „Die Standardspur jeder Datei im Clip- und Export-Ordner enthält alle aktiven Quellen. Geprüft mit Discord (Desktop), WhatsApp (Android und iOS) und der Galerie-App auf Android und iOS.“ Z. 117: „Player, die nur die Standardspur abspielen, spielen die Mischung.“

### [21] [`egm-style`] style-1-5
*   **Affected Scope:** Z. 21, 460, 420, 424–425
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Z. 21: „Kernlogik (`gameclip-core`), Protokoll (`gameclip-protocol`) und das Frontend (TypeScript) enthalten keinen Windows-Code; geprüft durch `cargo deny` und den Linux-CI-Job. Die Rust-Seite der Oberfläche nutzt Windows nur über `gameclip-ui-platform`.“ Gleiche Formulierung in Alternative A.

### [22] [`egm-style`] style-1-6
*   **Affected Scope:** Z. 437, 439, 445, 450
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Neue Q-Einträge: Q14 Signatur (siehe Fix 14); Q15 „Sind `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` im MSIX nötig und ohne Store-Freigabe erlaubt? Antwort: Recherche vor S1“; Q16 „Lizenz- und Pflegeprüfung der Rust-Bibliotheken, bis M1“. Z. 450 „wie erwartet“ ersetzen durch die Liste: Autostart startet gameclipd beim Login; Pipe ist von gameclip-ui erreichbar; Prozess-Loopback liefert Ton; WGC nimmt ohne gelben Rahmen und ohne Rückfrage auf.

### [23] [`egm-style`] style-1-7
*   **Affected Scope:** Z. 55, 57
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** „Ziel ≤ 80 MB (vorläufig, Q1)“ und „Ziel ≤ 3 % (vorläufig, Q1)“ (löst comprehension #10).

### [24] [`egm-style`] style-1-8 (+ structural-1-15)
*   **Affected Scope:** siehe Bericht
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Einheitlich: „Kürzen“ (nicht Trim, außer in Bezeichnern), „Worker“ (`gameclip-worker`), „Spur ‚Mischung‘“, „Drift“, „Benachrichtigung“. Neuer Abschnitt `## Begriffe` nach „Overview“ mit: WGC (Windows Graphics Capture), DDA (DXGI Desktop Duplication API), AUMID, GOP/Bildgruppe, Keyframe/IDR, QPC (QueryPerformanceCounter), TDR, LUID, PTS, CF_HDROP, Abschnitt (Pufferteil zwischen zwei Neustarts der Pipeline; nach einem Neustart lässt sich nichts von davor speichern), Mischung, Einzelspuren, Worker (löst comprehension #9, #15).

### [25] [`egm-style`] style-1-9
*   **Affected Scope:** Z. 262, 264, 281–284, 287, 291, 335
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Proposed Edit:*** Alle Bezeichner in Protokoll, Datenbank und Enums englisch und ASCII: `protocol_major/minor`, `tracks_path`, `target_part_path`, `duration_ms`, `bytes`; Profile `Mix`, `SeparateTracks`, `TargetSize{bytes}`; Felder der Worker-Meldungen englisch. Texte in der Oberfläche bleiben deutsch. Gilt auch für die DetailedRange.

---

## PROPOSE FIX – Nits

| Nr. | Befund | Vorgeschlagene Änderung |
| --- | --- | --- |
| 26 | structural-1-8 | „Future Improvements“ als `##`. |
| 27 | structural-1-9 | Hinweis Q8–Q10 (in Fix 5 enthalten). |
| 28 | structural-1-10 | Spalte „Optionen“ → „Entscheidung“, Verweis auf Alternativen V–Z; „[T2]“ in Komponenten-Tabelle entfernen. |
| 29 | structural-1-11 | Gruppen umbenennen: „Architektur“ für T, U, V–Z; S (Papierkorb) in neue Gruppe „Speicher“. |
| 30 | structural-1-12 | Äußeren Rahmen im ASCII-Diagramm entfernen, damit nichts mehr verrutscht. |
| 31 | structural-1-13 | Wiederholung (10-%-Versuch, Smart-Cut, Audio) in „Kürzen und Export“ auf je eine Stelle kürzen. |
| 32 | structural-1-14 | „Anhang A des Briefs“ verlinken. Bei der Kommandozeile vermerken: „abweichend vom Brief (`gameclip --save-clip`) heißt der Aufruf `gameclipd.exe --save-clip`, weil es kein eigenes CLI-Programm gibt“. |
| 33 | design-1-11 | Discord-Neustart: beide Ströme neu binden; Wurzelprozess = Prozess `Discord.exe`, `DiscordPTB.exe` oder `DiscordCanary.exe`, dessen Elternprozess nicht denselben Namen hat. |
| 34 | design-1-12 | Drift: Regelgröße = Abstand zwischen Gerätezeit (Sample-Zähler) und QPC; Regler passt das Resampling-Verhältnis um höchstens ±0,1 % an, über ein gleitendes Mittel von 10 s. |
| 35 | design-1-13 | Pipe mit `FILE_FLAG_FIRST_PIPE_INSTANCE`; der Client prüft über `GetNamedPipeServerProcessId`, dass der Server `gameclipd.exe` aus dem eigenen Paket ist. Grenze: Schutz gegen andere Benutzer und fremde Programme ohne Admin-Rechte. |
| 36 | design-1-14 | Abweichung zum Brief vermerken: „Gespeichert wird nur der neueste Abschnitt; der Brief sah mehrere Dateien vor. Einfacher; die Benachrichtigung sagt ‚Clip verkürzt (Neustart der Aufnahme um hh:mm:ss)‘.“ |
| 37 | design-1-15 | Diagnose ergänzen: Wechsel von Aufnahmeweg und Encoder, Ergebnis der Hotkey-Registrierung mit Fehlercode. |
| 38 | design-1-16 | `_2`-Regel auch für `_trim`/`_10MB`; `StartExportJob` antwortet mit `job_id`; alle Job-Ereignisse tragen `job_id`. |
| 39 | design-1-17 (+ comprehension #5) | Die Oberfläche startet `gameclipd`, wenn keine Pipe erreichbar ist (über die AUMID `…!GameClipService`), und zeigt „Dienst wird gestartet …“. Der erste Start ruft `StartupTask.RequestEnableAsync` auf. |
| 40 | design-1-18 | RAM-Ziel und Range-Unterstützung als „zu bestätigen in S1/S5“ markieren. |
| 41 | style-1-10 | „höchstens wenige Bilder“ → „ohne B-Frames höchstens 2 Bilder (~33 ms)“. |
| 42 | style-1-11 | In Fix 8 enthalten. |
| 43 | style-1-13 | Alternative V: „Saubere Identität“ → „Paket-Identität mit fester AUMID“; „Verknüpfungstricks“ → „ohne eigene Startmenü-Verknüpfung“; „schwer zu debuggen“ → „wenig Dokumentation, Fehler nur über Ereignisprotokoll sichtbar“. |
| 44 | style-1-14 | „deckt das Risiko ab“ → „weist auf das Risiko hin“. |
| 45 | style-1-15 | Alternative E: Verweis auf D3 statt G4; „ersetzen das Overlay“ → „übernehmen die Bestätigung“. |
| 46 | style-1-16 | G10 nur mit dem Drei-Schritte-Kriterium; „Mikrofon prüfen“ → „Mikrofon bestätigen oder wechseln“; „ruhig“ entfernen. |
| 47 | style-1-17 | „deutlich“ ersetzen durch „Tray-Symbol ‚Eingeschränkt‘ plus Benachrichtigung“ bzw. konkrete Form. |
| 48 | style-1-18 | „klein“ und „einfach“ an Z. 390, 391, 419 streichen. |
| 49 | style-1-19 | Löschregel: „Liegt die Summe aller eigenen Dateien über dem Limit, …“. |
| 50 | style-1-20 | CUJ 5 umformulieren, ein „trotzdem“ weniger. |
| 51 | style-1-21 | In Fix 2 enthalten. |
| 52 | style-1-22 | MB/GB dezimal (1 MB = 1 000 000 Bytes, 1 GB = 10⁹ Bytes) einmal in „Begriffe“ festhalten; Z. 18 „Er“ → „Der Dienst“. |
| 53 | comprehension-Nits #6, #12, #13, #14, #16 | Zahlen werden *nach* S1 bestätigt (Z. 44 angleichen); 720p30 = 36 % der 1080p60-Werte; Smart-Cut ist **nicht** v1, nur Option nach S9; F12 gesperrt, weil Windows die Taste für Debugger reserviert; S8 prüft SendInput, weil Makro-Tasten (F13–F24) oft synthetisch gesendet werden. |

---

## DECLINE

| Befund | Grund |
| --- | --- |
| structural-1-16 (Trennlinien, Revisionsverlauf) | Kosmetisch; der Revisionsverlauf liegt im Brief, das Design hat Status und Datum. |
| style-1-12 (Übertreibungen in Alternativen) | Die meisten Stellen werden durch Fix 3 ohnehin neu formuliert; „Jahre an Treiber-Sonderfällen“ wird dabei zu „Treiber-Sonderfälle, die gpu-screen-recorder bereits behandelt“. Kein eigener Fix nötig. |
| zoom-out-/comprehension-Befunde außerhalb dieser Liste (z. B. zoom-out M1 Worker vs. ffmpeg.exe, M2 Watcher, M3 Attrappe, M4 `clip://`, B1 Zeitpunkt G1) | Betreffen vor allem den Umsetzungsplan bzw. die Struktur von G1; sie werden in Stufe 8 und 9 erneut geprüft und dort gelöst. |
