# GameClip v1 – Clip-Rekorder für Windows 11

*   **Author:** Pold
*   **Status:** Proposed (Review Stufen 5–9 eingearbeitet)
*   **Date:** 2026-11-04
*   **Grundlage:** [concepts_brief.md](../../concepts_brief.md), Revision 9 (Entscheidungen D1–D25, [Entscheidungsregister](../../concepts_brief.md#entscheidungsregister))

--------------------------------------------------------------------------------

## Overview

GameClip ist eine Open-Source-Desktop-App (GPL-3.0-or-later) für Windows 11. Sie nimmt im Hintergrund ständig den Bildschirm und den Ton auf. Die letzten 15 Sekunden bis 5 Minuten (Standard 1 Minute) hält sie als bereits kodierte Video- und Audiodaten im Arbeitsspeicher. Drückt der Spieler einen Hotkey (Standard F8), schreibt GameClip diesen Puffer als MP4-Datei in einen gewählten Ordner.

In einer Galerie kann der Spieler seine Clips ansehen, bildgenau kürzen, auf eine Zielgröße verkleinern (z. B. 10 MB für Discord) und per Zwischenablage oder Drag-and-drop in einen Messenger geben. Die App funktioniert ohne Konto, ohne Cloud und ohne Internetverbindung.

GameClip besteht aus drei Programmen:

*   `gameclipd` läuft dauerhaft im Hintergrund. Der Dienst nimmt auf, hält den Puffer, reagiert auf den Hotkey und zeigt ein Tray-Symbol.
*   `gameclip-ui` ist das Fenster mit Galerie und Einstellungen. Es startet nur, wenn der Nutzer es öffnet, und startet den Dienst, falls er nicht läuft.
*   `gameclip-worker` ist ein kurzlebiger Prozess, den der Dienst für jeden Kürz- oder Export-Auftrag startet.

Linux (Ubuntu) folgt in einer späteren Version. Dort übernimmt das bestehende Projekt gpu-screen-recorder die Aufnahme. Deshalb enthalten die Kernlogik (`gameclip-core`), das Protokoll (`gameclip-protocol`) und das Frontend (TypeScript) keinen Windows-Code; geprüft wird das durch eine `cargo tree`-Prüfung und einen Linux-CI-Job. Die Rust-Seite der Oberfläche ruft Windows nur über das Crate `gameclip-ui-platform` auf (Zwischenablage, Drag-out, Explorer, Vorschaubilder, Pipe-Client, Start des Dienstes).

## Begriffe

| Begriff | Bedeutung |
| --- | --- |
| WGC | Windows Graphics Capture: Windows-Schnittstelle, die Bildschirm oder Fenster von außen aufnimmt, ohne Code im Spiel |
| DDA | DXGI Desktop Duplication API: ältere Schnittstelle, die einen ganzen Monitor aufnimmt; möglicher Ausweg, wenn WGC in einem Modus kein Bild liefert |
| Prozess-Loopback | WASAPI-Aufnahme des Tons bestimmter Prozesse bzw. aller Prozesse außer einem Prozessbaum |
| GOP / Bildgruppe | Folge von Bildern ab einem Keyframe bis vor den nächsten |
| Keyframe / IDR | Bild, das ohne vorherige Bilder dekodierbar ist; ab ihm kann ein Clip beginnen |
| PTS | Anzeigezeitpunkt eines Bildes oder Audiopakets |
| QPC | QueryPerformanceCounter: gemeinsame Uhr für Video und Audio |
| TDR | Treiber-Neustart der GPU nach einem Hänger |
| LUID | Kennung einer GPU (Adapter) im System |
| AUMID | Application User Model ID: Kennung, über die Windows Benachrichtigungen und Starts einer App zuordnet |
| CF_HDROP | Zwischenablage-Format für Dateien; damit lässt sich ein Clip per Strg+V einfügen |
| Abschnitt | Teil des Puffers zwischen zwei Neustarts der Aufnahme-Pipeline; nach einem Neustart lässt sich nichts von davor speichern |
| Mischung | Audiospur aus allen aktiven Quellen mit den eingestellten Lautstärken; immer die Standardspur |
| Einzelspuren | System, Mikrofon und Discord ohne Lautstärkeänderung, intern als `.m4a` gespeichert |
| Worker | `gameclip-worker`, der Prozess für Kürzen und Export |
| Kürzen | Clip auf Start- und Endpunkt schneiden (in Bezeichnern `trim`) |
| MB, GB | dezimal: 1 MB = 1 000 000 Bytes, 1 GB = 10⁹ Bytes |

## The Problem

Spieler wollen besondere Momente festhalten, ohne vorher zu wissen, wann sie passieren. Dafür gibt es „Instant Replay“: Die Software nimmt ständig auf und speichert erst auf Tastendruck die letzten Minuten.

Die vorhandenen Lösungen haben jeweils einen Haken:

| Lösung | Haken |
| --- | --- |
| Medal | Konto, Cloud-Funktionen und Werbung; nicht Open Source |
| NVIDIA App Instant Replay / AMD ReLive | nur für die GPU des jeweiligen Herstellers |
| Xbox Game Bar | vorinstalliert und ohne Konto; Kürzen nur über eine andere App, kein Export auf eine Zielgröße |
| OBS Replay Buffer | braucht eigene Einstellungen für Ausgabe, Encoder und Hotkey; kein Editor |

Der Ablauf, den GameClip verkürzen soll: **Hotkey drücken → Clip kürzen → auf unter 10 MB bringen → in Discord oder WhatsApp einfügen.** Ohne GameClip braucht man dafür neben dem Rekorder einen Editor und ein Werkzeug zum Verkleinern.

Dazu kommt eine harte Bedingung: Valorant ist ein Zielspiel. Dessen Anti-Cheat Riot Vanguard läuft im Kernel und blockiert oder meldet Software, die Code in das Spiel einschleust. GameClip darf deshalb nur Windows-Schnittstellen nutzen, die von außen auf den Bildschirm zugreifen. Es öffnet kein Handle auf den Spielprozess und lädt keinen Code hinein ([D3](../../concepts_brief.md#entscheidungsregister)).

**Risiko für das ganze Projekt:** Die Aufnahme allein ist kein Alleinstellungsmerkmal. Game Bar ist auf jedem Windows-Rechner schon vorhanden. Ob GameClip messbar besser ist, prüft Ziel G1 in zwei Schritten: G1a am Ende von M0 mit einem schmalen Durchstich, G1b nach M3. Fällt G1a schlecht aus, wird der Umfang vor dem Bau der Oberfläche überdacht (siehe Risiken R10).

## Goals

Die mit *vorläufig* markierten Zahlen werden auf einem benannten Referenzrechner nach dem Prototyp S1 bestätigt (Q1).

*   **G1 – Besser als Game Bar in mindestens 2 von 4 Messgrößen.** GameClip, Xbox Game Bar und OBS Replay Buffer werden auf demselben Rechner verglichen. Bei allen vier Größen ist kleiner besser; „besser“ heißt mindestens 10 % Abstand *(vorläufig)*:
    1.  Minuten von der Installation bis zum ersten Clip
    2.  RAM und CPU im Leerlauf, mit laufendem Puffer und Spiel
    3.  Anzahl der Einstellungen bis zu einem brauchbaren Clip
    4.  Zeit vom Hotkey bis zu einer Datei unter 10 MB in der Zwischenablage

    *   **G1a (Ende M0):** Ausgangswerte von Game Bar und OBS für alle vier Größen. Für GameClip werden Größe 2 und 4 gemessen, Größe 4 über einen schmalen Durchstich: Hotkey-Speichern (S1) → Worker-Grundgerüst mit Zielgröße (S9) → Datei per Tray-Aktion in die Zwischenablage (S6). Ist GameClip bei Größe 2 *und* 4 nicht besser, wird der Umfang überdacht, bevor M2 beginnt.
    *   **G1b (nach M3, vor M4):** der vollständige Vergleich aller vier Größen.
*   **G2 – Speichern in höchstens 3 Sekunden.** Gemessen vom Tastendruck bis zur fertig geschriebenen MP4-Datei. Der Wert gilt in 95 % der Fälle, mit Standardeinstellungen, auf einer SSD und mit Hardware-Encoder.
*   **G3 – Wenig Verbrauch im Hintergrund.**
    *   `gameclipd` ohne Puffer: Ziel ≤ 80 MB Private Bytes *(vorläufig, Q1)*.
    *   Puffer: belegt höchstens den in den Einstellungen angezeigten Wert (Prognose einschließlich Reserve).
    *   CPU: Ziel ≤ 3 % *(vorläufig, Q1)*, gemittelt über 10 Minuten mit Hardware-Encoder.
*   **G4 – Funktioniert mit Valorant.** Im Prototyp S1 laufen 48 Stunden ohne Vanguard-Fehler und ohne schwarzes Bild, danach 14 Tage ohne Kontosperre. Der Test wird vor jedem Release wiederholt.
*   **G5 – Die Standardspur enthält alle aktiven Quellen.** Jede Datei im Clip- und Export-Ordner hat als Standardspur die Mischung aus System-Ton, Mikrofon und (falls aktiviert) Discord. Geprüft mit Discord (Desktop), WhatsApp (Android und iOS) und der Galerie-App auf Android und iOS.
*   **G6 – Einzelspuren für den Schnitt.** System, Mikrofon und Discord werden zusätzlich getrennt aufgenommen. Sie lassen sich auf Wunsch mit exportieren.
*   **G7 – Bildgenaues Kürzen.** Ein Schnitt bei Bild N ergibt eine Datei, deren erstes Bild laut ffprobe Bild N ist.
*   **G8 – Export auf Zielgröße.** Der Größen-Export liefert eine Datei unter der Zielgröße (Standard 10 MB) oder bricht mit klarer Meldung ab. Abnahme: 20 Testclips von 5 bis 300 s, keiner über der Zielgröße.
*   **G9 – Begrenzter Speicherplatz.** Clip-Ordner und Export-Ordner haben je ein Limit (Standard 10 GB). Bei Überschreitung löscht GameClip die ältesten eigenen Clips. Mit Stern markierte Clips und fremde Dateien bleiben unangetastet.
*   **G10 – Höchstens drei Einrichtungsschritte.** Ein neuer Nutzer kommt mit drei Schritten zum ersten Clip: Mikrofon bestätigen oder wechseln, Hotkey testen, Ordner wählen.

## Non-Goals

*   **Linux in Version 1.** Kommt später mit gpu-screen-recorder als Aufnahmekern ([Anhang A des Briefs](../../concepts_brief.md#anhang-a-später--linux)).
*   **Windows 10.** Der Support endete im Oktober 2025.
*   **Steam OS, macOS.**
*   **Cloud-Upload, Konten, Social-Funktionen, Updates, die von selbst ins Internet gehen.**
*   **Overlay im Spiel.** Bräuchte Code-Injektion und verträgt sich nicht mit Vanguard.
*   **HDR-Aufnahme.** v1 nimmt in SDR auf und warnt, wenn Windows-HDR aktiv ist.
*   **Nur den Ton des Spiels aufnehmen.** v1 nimmt den System-Ton auf, Discord optional getrennt.
*   **Mischregler im Editor.** Lautstärkeregler gibt es nur in den Einstellungen und nur für die Mischung.
*   **Bearbeitung über das Kürzen hinaus.** Kein Text, keine Übergänge, kein Zusammenfügen.
*   **Smart-Cut.** Nicht in v1; Option nach S9 (Q7).
*   **Automatische Highlight-Erkennung, Webcam, Livestreaming.**
*   **Öffentliche Kommandozeile.** `gameclipd.exe --save-clip` existiert nur intern, ohne Dokumentation, Alias und Stabilitätszusage. Abweichend vom Brief (D23: `gameclip --save-clip`) heißt der Aufruf so, weil es kein eigenes CLI-Programm gibt.

## Open Questions

*   **Q1 – Zahlen und Referenzrechner.** Konkrete Grenzwerte für RAM (Dienst, Puffer, offenes Fenster, Spitze beim Export), CPU, GPU-Last und Stromverbrauch sowie Bildverluste im Spiel. Bitraten der Qualitätsstufen. Welcher Rechner und welches Spiel dienen als Referenz? Festgelegt nach S1.
*   **Q2 – MSIX-Publisher.** Paketformat (MSIX) und App-ID (`io.github.poldion.GameClip`) sind entschieden. Der `Publisher` im Manifest muss genau dem Subjekt des Signaturzertifikats entsprechen. Er steht erst fest, wenn das Zertifikat da ist (Q14), und darf sich danach nicht mehr ändern, sonst gilt jedes Update als neue App.
*   **Q3 – Mindest-Build von Windows 11.** Zum Beispiel 24H2 (26100). Hängt davon ab, ab wann `MinUpdateInterval` und die randlose Aufnahme verfügbar sind. Recherche vor S1.
*   **Q4 – Exklusives Vollbild.** Liefert WGC dort ein Bild? Wird DDA als Ausweg gebraucht? Antwort aus S1 (eigenes Kriterium dort).
*   **Q5 – Media Foundation als Zwischenstufe.** Gibt es Hardware, auf der NVENC/AMF/QSV versagen, Media Foundation aber funktioniert? Antwort aus S2 (eigenes Kriterium dort). Mit FFmpeg 8.1 nimmt `h264_mf` vermutlich keine D3D11-Frames an (prüfen); die Stufe bräuchte dann eine Kopie in den Hauptspeicher.
*   **Q6 – Sieht das Spiel die Hotkey-Taste noch?** Bestimmt den Warnungstext im Auswahldialog. Antwort aus S8.
*   **Q7 – Smart-Cut.** Ist das vollständige Neukodieren beim Kürzen zu langsam (Ziel: 60-s-Clip in ≤ 15 s, *vorläufig*)? Nur dann wird Smart-Cut nachgerüstet. Antwort aus S9.
*   **Q11 – Mehrere ausgeschlossene Prozessbäume pro Loopback-Strom.** Erlaubt Windows das? Bis zur Antwort aus S4 gilt die Stummschaltung des eigenen Tons (siehe Audio-Pipeline).
*   **Q14 – Signatur (Prüfung vor S1).** Signiert die SignPath Foundation MSIX-Pakete, die fremde Binärdateien (BtbN-FFmpeg) enthalten? Wie lautet das Zertifikatssubjekt? Ausweg siehe Risiko R3.
*   **Q15 – MSIX-Fähigkeiten.** Sind `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` nötig und ohne Store-Freigabe erlaubt? Recherche vor S1.
*   **Q16 – Rust-Bibliotheken.** Lizenz- und Pflegeprüfung der Bibliotheken unter „Abhängigkeiten und Lizenz“. Für die in M0 genutzten (`windows`, `rsmpeg`, `wasapi`, `rusqlite`) vor M0, für die übrigen bis M1.
*   **Q17 – Automatische Pausen (entschieden, D27):** Die Aufnahme pausiert automatisch, solange der PC gesperrt ist, und läuft beim Entsperren wieder an. Keine automatische Pause im Akkubetrieb oder außerhalb von Vollbild-Apps.
*   **Q18 – P2 (entschieden, D26):** GameClips eigener Bestätigungston landet nicht im Clip (Prozess-Loopback mit Ausschluss bzw. Stummschaltung, siehe „Audio-Pipeline“).

Q8–Q10 wurden im Brief erledigt (D7, D21, D22). Q12 und Q13 sind im Entwurf beantwortet: siehe „Speicherlimit und Löschen“ und „Clip-Index“.

## Critical User Journeys (CUJs)

### CUJ 1: Erster Start bis zum ersten Clip

*   **Given**: GameClip ist frisch installiert. Ein Mikrofon ist angeschlossen, eine NVIDIA-, AMD- oder Intel-GPU ist vorhanden.
*   **When**: Der Nutzer startet GameClip zum ersten Mal und folgt der Checkliste. Erst bestätigt oder wechselt er das Mikrofon (standardmäßig an). Dann drückt er den Hotkey einmal zum Test (Standard F8). Zuletzt wählt er den Clip-Ordner.
*   **Then**: Die Statuszeile zeigt z. B. „Puffert 1:00 in 1080p60 über NVENC“. Ein Druck auf „Jetzt Test-Clip erstellen“ legt eine MP4 im Clip-Ordner ab, und der Erfolgston ertönt.

### CUJ 2: Clip im Spiel speichern

*   **Given**: `gameclipd` puffert. Der Nutzer spielt Valorant im Vollbild.
*   **When**: Er drückt den Hotkey.
*   **Then**: Innerhalb von 3 Sekunden liegt eine MP4 nur mit der Spur „Mischung“ im Clip-Ordner. Die Einzelspuren liegen intern. Der Erfolgston ertönt, Vanguard meldet nichts. Ist der Clip-Ordner über seinem Limit, wurden die ältesten nicht geschützten GameClip-Clips gelöscht.

### CUJ 3: Clip kürzen und in Discord teilen

*   **Given**: Ein Clip liegt im Clip-Ordner.
*   **When**: Der Nutzer öffnet die Galerie, setzt Start- und Endpunkt bildgenau und wählt „Für Discord exportieren (10 MB)“.
*   **Then**: Während das Spiel weiterläuft, erzeugt der Worker eine Datei unter 10 MB im Export-Ordner. Der Puffer verliert dabei keine Bilder, und die Bildrate des Spiels sinkt um höchstens 10 % *(vorläufig)*. Die Datei liegt per Knopfdruck in der Zwischenablage, Strg+V in Discord fügt sie ein.

### CUJ 4: Clip mit getrennten Spuren für den Schnitt exportieren

*   **Given**: Ein Clip liegt im Clip-Ordner, seine Einzelspuren liegen intern.
*   **When**: Der Nutzer wählt im Export-Dialog „Mit getrennten Spuren exportieren“.
*   **Then**: Im Export-Ordner entsteht eine MP4 mit den Spuren „Mischung“ (Standard), „System“, „Mikrofon“ und gegebenenfalls „Discord“. Player, die nur die Standardspur abspielen, spielen die Mischung. Ein Schnittprogramm zeigt alle Spuren getrennt.

### CUJ 5: Speicherlimit erreicht

*   **Given**: Der Clip-Ordner enthält 10 GB GameClip-Clips, einige davon mit Stern.
*   **When**: Der Nutzer speichert einen neuen Clip.
*   **Then**: GameClip löscht die ältesten Clips ohne Stern samt ihren Einzelspuren endgültig, bis der neue Clip ins Limit passt. Fremde Dateien bleiben unangetastet. Ist alles geschützt, wird nichts gelöscht; der neue Clip wird gespeichert, und eine Warnung erscheint.

### CUJ 6: Hotkey ist schon belegt

*   **Given**: Eine andere App hat F8 bereits per RegisterHotKey belegt.
*   **When**: `gameclipd` startet beim Login.
*   **Then**: Das Tray zeigt „Eingeschränkt: Hotkey nicht aktiv“, es kommen ein Fehlerton und eine Benachrichtigung. Im Auswahldialog lässt sich eine andere Taste wählen und sofort testen, auch F13 bis F24. Ist die neue Taste registriert, verschwindet die Einschränkung ohne Neustart.

## Technical Plan (Architecture)

### Architekturentscheidungen

Diese Punkte hatten mehr als eine vertretbare Lösung und wurden vom Autor entschieden.

| ID | Frage | Entscheidung | Verworfen |
| --- | --- | --- | --- |
| T1 | Paketformat und Installer | volles MSIX (siehe „Paket und Installation“) | [Alternative V](#alternative-v-nsismsi-über-tauri-oder-msi-plus-sparse-package-t1) |
| T2 | Nachrichtenformat auf der Named Pipe | Längenpräfix + JSON (siehe „Dienst-Schnittstelle“) | [Alternative W](#alternative-w-json-pro-zeile-oder-messagepack-auf-der-pipe-t2) |
| T3 | Wo der Clip-Index liegt | SQLite im App-Ordner (siehe „Clip-Index“) | [Alternative X](#alternative-x-index-nur-in-metadaten-tags-oder-als-json-datei-t3) |
| T4 | Arbeitsprozess für Kürzen und Export | eigenes Programm `gameclip-worker` (siehe „Kürzen und Export“), nach Review bestätigt | [Alternative Y](#alternative-y-mitgelieferte-ffmpegexe-als-arbeitsprozess-t4) |
| T6 | Abgleich des Clip-Index mit den Ordnern | bei Bedarf, Watcher nur bei offener Galerie (siehe „Clip-Index“) | dauerhafter Watcher; nur bei Bedarf ohne Watcher |
| T7 | Test-Ersatz für den Dienst | Mock-Modus im Frontend statt eigener Attrappe | Attrappe `gameclip-fake-daemon` mit eigenem Transport |
| T8 | Wie der Player Dateien lädt | eigenes Protokoll `clip://` mit Range, korrigiert für WebView2 (siehe „Oberfläche“) | Tauri-Asset-Protokoll |
| T5 | Frontend-Framework in Tauri | React (siehe „Oberfläche“) | [Alternative Z](#alternative-z-svelte-oder-solidjs-t5) |

### Überblick

```text
gameclipd (immer aktiv, kein WebView)

  WGC-Monitor ─► GPU-Kopie ─► Taktgeber 60 fps ─► Skalieren + BGRA→NV12 ─► HW-Encoder (H.264) ─┐
                                                                                                │
  WASAPI: System-Loopback ─┐                                                                    │
          Mikrofon ────────┼─► Resampler 48 kHz ─┬─► AAC je Quelle ─────────────────────────────┤
          Discord-Loopback ┘                    └─► Mixer (Gain, Limiter) ─► AAC Mischung ──────┤
                                                                                                ▼
                                                     Ringpuffer (Blöcke + Paketindex)
                                                              │
  Hotkey (RegisterHotKey) ──► Zustandsmaschine ──► Speichern ─┴─► MP4 (Mischung) → Clip-Ordner
                                     │                         └─► .m4a (Einzelspuren) → tracks\
  Tray · Töne · Benachrichtigungen ◄─┤
                                     ├──► Auftragswarteschlange ──► gameclip-worker ──► Export-Ordner
  Clip-Index (SQLite) · Einstellungen┤
                                     └──► Named Pipe (nur aktueller Benutzer, Längenpräfix + JSON)
                                                │
                         gameclip-ui (Tauri + WebView2, bei Bedarf)
                         Galerie · Kürzen · Export · Einstellungen
                         Zwischenablage · Drag-out · Explorer
```

### Komponenten

| Komponente | Aufgabe | Läuft in |
| --- | --- | --- |
| Aufnahme Video | WGC-Monitoraufnahme, GPU-Kopie, Taktgeber, Skalierung, NV12-Umwandlung | gameclipd, eigener Thread |
| Encoder | FFmpeg mit Hardware-Frames, Encoder-Auswahl und Ausweichkette | gameclipd, eigener Thread |
| Aufnahme Audio | WASAPI-Quellen, Resampler, Mixer, AAC-Encoder | gameclipd, ein Thread je Quelle plus Mixer |
| Ringpuffer | Pakete speichern, alte Bildgruppen verwerfen, Schnappschüsse beim Speichern | gameclipd |
| Muxer | MP4 und .m4a aus einem Schnappschuss schreiben | gameclipd, ein Speicher-Thread |
| Zustandsmaschine | Zustände, Einschränkungen und Übergänge, ohne Betriebssystem-Code | gameclip-core |
| Hotkey | RegisterHotKey auf einem eigenen Nachrichten-Thread | gameclipd |
| Tray, Töne, Benachrichtigungen | Status, Menü, Erfolgs-/Fehlerton, Toasts | gameclipd |
| Clip-Index und Speicherlimit | Eigentum, Stern, Zuordnung, Löschen der ältesten Clips | gameclipd (SQLite) |
| Auftragswarteschlange | Kürzen und Export, höchstens ein Auftrag gleichzeitig | gameclipd, startet `gameclip-worker` |
| Worker | Dekodieren, Schneiden, Kodieren, Muxen, Größen-Prüfung | `gameclip-worker`, ein Prozess je Auftrag |
| IPC-Server | Named Pipe, Befehle und Ereignisse | gameclipd |
| Oberfläche | Galerie, Player, Kürzen, Export-Dialog, Einstellungen, erster Start | gameclip-ui (React) |
| UI-Plattformdienste | Zwischenablage, Drag-out, Explorer, Vorschaubilder | gameclip-ui, Rust-Seite von Tauri |

### Video-Pipeline

1.  **Aufnahme.** `GraphicsCaptureItem` für den gewählten Monitor (Standard: Hauptmonitor). `IsBorderRequired = false` nach `RequestAccessAsync(Borderless)`, `IsCursorCaptureEnabled` laut Einstellung. Der Frame-Pool liefert BGRA-Texturen auf einem gemeinsamen `ID3D11Device` mit Video-Unterstützung und Multithread-Schutz. Dieses Gerät wird auch an FFmpeg übergeben.
2.  **Kopie.** WGC verwendet seine Texturen wieder, deshalb wird jedes Bild einmal auf der GPU in eine eigene Textur kopiert. Ein Zurücklesen in den Hauptspeicher gibt es nicht. Die einzige Ausnahme ist der x264-Weg (siehe 5).
3.  **Taktgeber.** Ein Thread läuft im festen 60-fps-Takt nach QPC. Bei jedem Takt nimmt er das neueste Bild. Ist keines neu, wiederholt er das letzte. Kommen mehrere Bilder an, verwirft er alle außer dem neuesten. Wo es der Windows-Build erlaubt, begrenzt `MinUpdateInterval` die Aufnahmerate schon vorher.
4.  **Skalieren und Umwandeln.** Der D3D11-VideoProcessor skaliert auf die feste Ausgabeauflösung (Standard 1080p) und wandelt BGRA in NV12 um (BT.709, begrenzter Bereich). Wechselt die Quelle die Auflösung, ändert sich nur die Eingabe des VideoProcessors. Der Encoder läuft weiter.
5.  **Kodieren.** Die NV12-Textur geht als `AV_PIX_FMT_D3D11`-Frame an den Encoder. Die Ausweichkette prüft beim Start mit einer echten Testsitzung:
    1.  `h264_nvenc`, `h264_amf`, `h264_qsv` (QSV über abgeleitete Hardware-Frames) – je nach GPU am Monitor
    2.  `h264_mf` (Media Foundation), nur wenn S2 einen Nutzen zeigt (Q5)
    3.  `libx264` mit Preset `superfast`, Einschränkung `cpu_encoder` (Tray-Symbol „Eingeschränkt“ plus Benachrichtigung). Nur hier wird das Bild in den Hauptspeicher kopiert.

    Einstellungen für alle Encoder: H.264 High, geschlossene Bildgruppen, ein Keyframe pro Sekunde (60 Bilder), keine B-Frames (dadurch höchstens 2 Bilder, ~33 ms, Verzögerung beim Speichern), begrenzte Maximalbitrate (CBR oder VBR mit `maxrate`), Keyframe auf Anforderung (für den Ringpuffer). Aufnahme und Export nutzen getrennte Encoder-Profile (siehe Datei 27).
6.  **Neustart.** Bei Geräteverlust (TDR, Standby, Monitorwechsel) baut die Pipeline Gerät und Encoder neu auf. Im Puffer beginnt ein neuer Abschnitt. Die Zustandsmaschine wechselt nach „Erholt sich“.
7.  **HDR.** Ist Windows-HDR auf dem gewählten Monitor aktiv, setzt die App die Einschränkung `hdr_active` mit dem Text „HDR wird nicht unterstützt, Farben können blass wirken“.

### Audio-Pipeline

| Quelle | Mechanismus | Wann |
| --- | --- | --- |
| System | Prozess-Loopback mit `EXCLUDE_TARGET_PROCESS_TREE`. Ausgeschlossen ist gameclipd, oder Discord, wenn die Discord-Spur an ist (Q11) | immer |
| Mikrofon | WASAPI-Aufnahme im Shared Mode, Gerät wählbar (Standard: Windows-Standardmikrofon) | standardmäßig an |
| Discord | Prozess-Loopback mit `INCLUDE_TARGET_PROCESS_TREE` auf den Discord-Wurzelprozess | optional |

*   **Discord finden:** Der Dienst liest alle 2 s eine Prozessliste (`CreateToolhelp32Snapshot`). Wurzelprozess ist ein Prozess `Discord.exe`, `DiscordPTB.exe` oder `DiscordCanary.exe`, dessen Elternprozess nicht denselben Namen hat. Startet oder endet Discord oder ändert sich die PID, werden **beide** Ströme neu gebunden: die Discord-Spur (`INCLUDE`) und die Spur „System“ (`EXCLUDE`), damit Discord nicht doppelt in der Mischung landet. Es wird kein Handle auf Discord geöffnet.
*   Jede Quelle wird auf 48 kHz Stereo Float umgerechnet. Lücken (WASAPI liefert bei Stille keine Pakete) werden mit Stille aufgefüllt.
*   **Drift:** Zeitstempel kommen von derselben QPC-Uhr wie beim Video. Regelgröße ist der Abstand zwischen der Gerätezeit (Sample-Zähler) und QPC. Der Regler passt das Resampling-Verhältnis um höchstens ±0,1 % an, über ein gleitendes Mittel von 10 s.
*   **Einzelspuren:** jede Quelle wird ohne Lautstärkeänderung als AAC-LC mit 128 kbit/s kodiert (FFmpeg-Encoder `aac`).
*   **Mischung:** Summe der Quellen mit den Lautstärkereglern (Standard 0 dB), danach ein Limiter gegen Übersteuern, kodiert als AAC-LC mit 160 kbit/s.
*   **Eigener Ton:** Ist gameclipd nicht ausgeschlossen (weil Discord ausgeschlossen ist und Windows nur einen ausgeschlossenen Prozessbaum erlaubt, Q11), merkt sich der Dienst Beginn und Dauer seines Erfolgs- bzw. Fehlertons. Die Spur „System“ wird in diesem Zeitfenster stummgeschaltet.
*   Wird das Mikrofon pausiert, liefert seine Quelle Stille. Die Spur bleibt also erhalten.

### Ringpuffer

*   **Speicher:** Ein Pool aus Blöcken zu 2 MB, beim Start reserviert und belegt (committed). Die Größe ist die Prognose plus Reserve (siehe unten). Ändert der Nutzer eine Einstellung, die die Pipeline betrifft, wird der Puffer neu angelegt („Puffer wird zurückgesetzt“).
*   **Paketindex:** Je Paket ein Eintrag `{track, pts, dts, keyframe, block, offset, len, section}`. Spuren: Video, Mischung, System, Mikrofon, Discord.
*   **Verwerfen:** Gilt eine Zeitgrenze (Pufferlänge) und eine Bytegrenze (Poolgröße). Verworfen wird immer die älteste ganze Video-Bildgruppe. Audiopakete vor dem neuen ältesten Keyframe fallen mit weg.
*   **Schnappschuss:** Beim Speichern werden die betroffenen Blöcke per Referenzzählung festgehalten und nicht überschrieben. Der Encoder schreibt in freie Blöcke weiter.
*   **Reserve erschöpft:** Gibt es keinen freien Block, verwirft der Encoder alle Videopakete bis zum nächsten Keyframe und fordert sofort einen neuen Keyframe an (IDR). Audio im selben Zeitraum wird ebenfalls verworfen. Der Index markiert die Lücke; ein späterer Clip über die Lücke wird an ihr getrennt wie bei einem neuen Abschnitt. Die Reserve ist für den schlimmsten Fall bemessen: ein gehaltener Schnappschuss über den vollen Puffer plus 30 s weiterlaufende Aufnahme (Speichern auf HDD).
*   **Prognose (eine Funktion für Dienst und Oberfläche):**
    *   `Prognose = (V_max + Σ Audiobitraten der aktiven Spuren) × (T + 1 s) / 8 × 1,05`, aufgerundet auf ganze Blöcke. Die `+1 s` decken die angefangene Bildgruppe am Anfang ab, `×1,05` den Overhead des Paketindex.
    *   `Reserve = (V_max + Σ Audiobitraten) × 30 s / 8`.
    *   Angezeigt wird `Prognose + Reserve + Grundbedarf`. Beispiel mit Standardwerten (1080p60 „Mittel“ 14 Mbit/s, Mischung und zwei Einzelspuren, 60 s): Prognose ≈ 116 MB, Reserve ≈ 54 MB, Grundbedarf 80 MB, angezeigt „ca. 250 MB RAM“.
    *   Ab 10 % des eingebauten RAM gibt es eine gelbe Warnung mit Rückfrage, ab 25 % wird die Einstellung abgelehnt.

### Speichern per Hotkey

1.  WM_HOTKEY kommt an. Die Zustandsmaschine prüft, ob gepuffert wird. Speichervorgänge laufen **nacheinander** auf einem Speicher-Thread; läuft einer, wird höchstens ein weiterer eingereiht, weitere Tastendrücke werden ignoriert.
2.  Der Dienst nimmt den QPC-Zeitstempel des Tastendrucks und wartet, bis das Videopaket mit diesem Zeitpunkt im Puffer liegt (ohne B-Frames höchstens 2 Bilder).
3.  Schnappschuss: vom letzten Keyframe vor `Tastendruck − Pufferlänge` bis zum Tastendruck, nur aus dem neuesten Abschnitt. Ist der Puffer noch nicht voll, wird gespeichert, was da ist. Ist der Clip wegen eines Abschnittswechsels kürzer, sagt die Benachrichtigung „Clip verkürzt (Aufnahme neu gestartet um hh:mm:ss)“. Der Brief sah hier mehrere Dateien vor; eine Datei ist einfacher und reicht.
4.  Der Clip wird mit Status `pending` in den Index eingetragen, bevor eine Datei sichtbar wird. So hält kein Abgleich den eigenen Clip für fremd.
5.  Es werden zwei Dateien geschrieben, jeweils zuerst als `.part`, dann nach dem Schreiben erneut geöffnet und mit `FlushFileBuffers` gesichert, dann umbenannt:
    *   `<Clip-Ordner>\<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4` mit Video und der Spur „Mischung“. Das moov-Atom kommt ans Ende, damit die Datei in einem Durchgang geschrieben wird. Die Datei trägt die Metadaten-Tags `gameclip_id`, `gameclip_created` und `gameclip_version` (FFmpeg-Option `movflags +use_metadata_tags`, sonst schreibt der MP4-Muxer eigene Schlüssel nicht).
    *   `%LOCALAPPDATA%\GameClip\tracks\<UUID>.m4a` mit den Einzelspuren „System“, „Mikrofon“ und gegebenenfalls „Discord“, Spurtitel als `handler_name`, ebenfalls mit `gameclip_id`.
    *   Schlägt nur die `.m4a` fehl, bleibt die MP4 gültig; der Clip hat dann keine Einzelspuren, und die Benachrichtigung sagt das. Schlägt die MP4 fehl, wird auch die `.m4a` verworfen.
6.  **Gemeinsamer Nullpunkt:** Beide Dateien nutzen t0 = PTS des ersten Keyframes. Audio vor t0 wird verworfen, die AAC-Anlaufverzögerung wird in beiden Dateien als Edit-List geschrieben. Damit liegen MP4 und `.m4a` ohne gespeicherten Versatz übereinander.
7.  **Programmname:** Die PID kommt von `GetWindowThreadProcessId` für das Vordergrundfenster, der Name aus einer Prozessliste (`CreateToolhelp32Snapshot`, Feld `szExeFile`). Es wird kein Handle auf den Spielprozess geöffnet. Ersatzweise heißt der Clip „Clip“. Unerlaubte Zeichen werden ersetzt, bei Namensgleichheit wird `_2` angehängt.
8.  Der Index-Eintrag wechselt auf `ok`, danach greift das Speicherlimit (siehe unten).
9.  Erst dann ertönt der Erfolgston und das Ereignis `SaveCompleted` geht an verbundene Oberflächen. Bei einem Fehler (`DiskFull`, `FolderMissing`, `BufferEmpty`, `EncoderFailed`, `Timeout`) werden die `.part`-Dateien und der `pending`-Eintrag gelöscht, der Fehlerton ertönt und `SaveFailed{code}` wird gesendet. Kommt das Videopaket aus Schritt 2 nicht innerhalb von 500 ms, endet der Vorgang mit `Timeout`. Der Zustand der Pipeline ändert sich dadurch nicht.

### Speicherlimit und Löschen

*   Je Ordner (Clip-Ordner, Export-Ordner) gilt ein Limit, Standard 10 GB. Gezählt werden nur Dateien direkt im Ordner, nicht in Unterordnern.
*   **Eigene Datei:** Eine Datei gilt nur dann als GameClip-eigen, wenn sie im Index mit Status `ok` steht **und** ihr Metadaten-Tag `gameclip_id` mit dem Index übereinstimmt. Vor dem Löschen wird der Tag erneut gelesen. Fremde Dateien zählen weder zum Limit noch werden sie gelöscht.
*   **Nach jedem neuen Clip:** Liegt die Summe aller eigenen Dateien über dem Limit, werden die ältesten Clips ohne Stern endgültig gelöscht (`DeleteFileW`, kein Papierkorb), bis die Summe passt. Der gerade geschriebene Clip wird nie gelöscht.
*   Zu jedem gelöschten Clip im Clip-Ordner wird auch seine `.m4a` gelöscht.
*   Reicht das nicht, weil alles geschützt ist oder ein Clip allein größer ist als das Limit, wird nichts gelöscht. Es erscheint eine Warnung im Tray und in der Galerie.
*   Sind Clip- und Export-Ordner derselbe Ordner, gilt ein gemeinsames Limit, und zwar das kleinere der beiden eingestellten. Die Einstellungen weisen darauf hin.
*   **Wann gelöscht wird:** nach jedem neuen Clip oder Export, wenn ein Limit verringert wird und wenn ein Ordner gewechselt wird.
*   **Ordnerwechsel:** Clips im bisherigen Ordner bleiben im Index und in der Galerie, mit ihren Einzelspuren. Sie zählen nicht mehr zum Limit und werden nie automatisch gelöscht.
*   **Cloud-Platzhalter (z. B. OneDrive):** Eine Datei mit dem Attribut `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` liegt nicht lokal vor. Sie zählt mit 0 Bytes zum Limit, wird nie automatisch gelöscht, und ihr Tag wird nie gelesen, damit kein Herunterladen ausgelöst wird. Die Attribute werden vor jedem Lesen geprüft.
*   **Cloud-Ordner wählen:** Wählt der Nutzer einen Ordner unterhalb des OneDrive-Stammordners (Umgebungsvariable `%OneDrive%`), zeigt die App einmal: „Gelöschte Clips werden auch in der Cloud gelöscht; Speichern kann langsamer sein.“ Liegt der Windows-Videos-Ordner selbst in OneDrive, wird als Standard stattdessen `%USERPROFILE%\GameClip` vorgeschlagen, damit der erste Start ohne Warnung auskommt.
*   **Verwaiste Einzelspuren:** Fehlt die zugehörige MP4, werden die `.m4a` entfernt (siehe „Clip-Index“). Einzelspuren zählen nicht zum Ordner-Limit, weil sie im App-Ordner liegen. Sie belegen etwa 1 MB pro Minute und Spur.

### Clip-Index (T3: SQLite)

*   **Datei und Zugriff:** `index.sqlite` mit `rusqlite` im WAL-Modus. Nur gameclipd schreibt, und zwar über einen einzigen Thread. Die Oberfläche fragt über IPC ab.
*   **Tabelle `clips`:**

    | Spalte | Bedeutung |
    | --- | --- |
    | `row_id` | Zeilennummer; ist die `clip_id` im Protokoll |
    | `tag_id` | UUID aus dem Metadaten-Tag `gameclip_id`; nicht eindeutig, weil Kopien denselben Tag tragen |
    | `folder` | `clips` oder `exports` |
    | `path` | absoluter Pfad |
    | `size_bytes` | Dateigröße |
    | `created_at` | Zeitpunkt der Erstellung |
    | `duration_us` | Dauer in µs |
    | `starred` | Stern ja/nein |
    | `has_tracks` | ob `tracks\<id>.m4a` existiert (Pfad folgt aus der ID) |
    | `source_row_id` | bei Exporten die Zeile des Original-Clips |
    | `placeholder` | Datei ist ein Cloud-Platzhalter |
    | `missing_count` | Anzahl aufeinanderfolgender Abgleiche, bei denen die Datei fehlte |
    | `status` | `pending`, `ok` oder `foreign` |

*   **Tabelle `meta`:** `schema_version`. In v1 gibt es nur Version 1; Migrationen werden mit der ersten Schema-Änderung eingeführt. Eine App, die ein neueres Schema findet, öffnet den Index nur lesend und setzt das automatische Löschen aus.
*   **Abgleich mit dem Dateisystem (T6):**
    *   Bei Bedarf: beim Start, vor jedem Löschlauf und bei `ListClips`. Eingelesen werden nur Dateien direkt im Ordner; `.part`-Dateien werden immer übersprungen.
    *   Nur solange die Galerie offen ist, beobachtet ein Watcher (`ReadDirectoryChangesW`) Clip- und Export-Ordner. Er enthält keine eigene Logik, sondern löst nach 1 s Ruhe einen Abgleich aus. Meldet Windows einen Überlauf (`ERROR_NOTIFY_ENUM_DIR`), folgt ebenfalls ein Abgleich.
    *   Fehlt eine Datei, steigt `missing_count`. Fehlt sie bei zwei Abgleichen hintereinander, verschwinden Eintrag und `.m4a`.
    *   Ein neuer Pfad mit bekannter ID (umbenannt oder verschoben im selben Ordner) aktualisiert `path`.
    *   Einträge mit Status `pending` (gerade im Speichern) werden beim Abgleich nicht angefasst.
*   **Unbekannte Clips:** Trägt eine MP4 einen `gameclip_id`-Tag, steht aber nicht im Index (z. B. nach einer Neuinstallation), übernimmt der Abgleich sie automatisch **mit Stern**. So geht nach einer Neuinstallation nichts verloren, und kein Clip muss einzeln übernommen werden. Die Galerie zeigt einmal: „N Clips übernommen und geschützt. Stern entfernen, damit sie zum automatischen Löschen zählen.“
*   **Doppelte IDs:** Wird eine Datei kopiert, gibt es zwei Dateien mit derselben ID. Die Datei, deren Pfad im Index steht, bleibt der Eintrag. Die Kopie wird `foreign`: Sie zählt nicht zum Limit, wird nie gelöscht, und die Galerie bietet „Als eigenen Clip übernehmen“ an (der Dienst schreibt dann den Tag der Datei mit einer neuen UUID neu, ohne neu zu kodieren). Pro `tag_id` besitzt höchstens eine Zeile ihre Datei; die vollständigen Regeln stehen in Datei 45.
*   **Beschädigung:** Lässt sich die Datenbank nicht öffnen, wird sie umbenannt (`index.sqlite.broken`) und durch einen Abgleich neu aufgebaut. Dabei gilt die Regel für unbekannte Clips: Alle getaggten Clips werden mit Stern übernommen. Die vorherigen Sterne gehen verloren, gelöscht wird aber nichts.

### Kürzen und Export

*   Die Oberfläche sendet `StartExportJob{clip_id, in_pts, out_pts, profile}` und erhält als Antwort eine `job_id`. Kürzen und Größen-Export sind ein Befehl mit Profil, weil beide denselben Ablauf im Worker nutzen. Profile:
    *   `Mix` (Standard)
    *   `SeparateTracks`
    *   `TargetSize{bytes}` (Standard 10 000 000)
*   Der Dienst reiht den Auftrag ein, höchstens einer läuft gleichzeitig. Er startet `gameclip-worker.exe` mit Priorität `BELOW_NORMAL` und übergibt nur Dateipfade, die er selbst bestimmt.
*   **Vertrag mit dem Worker (T4):**
    *   **Eingabe:** Der Auftrag kommt als JSON über stdin, z. B. `{source, tracks_path, target_part_path, output_tag_id, in_us, out_us, profile, encoder_order, adapter_luid}`. `output_tag_id` ist die neue `gameclip_id` der Ausgabedatei; Zeitangaben siehe „Zeitachse“ unten.
    *   **Ausgabe:** Eine JSON-Zeile pro Meldung über stdout:
        *   `{"progress": {"percent": 42.0}}` (0,0–100,0)
        *   `{"encoder": "h264_nvenc"}`
        *   `{"done": {"bytes": …, "duration_us": …}}`
        *   `{"error": {"code": "…", "message": "…"}}`
    *   **Exit-Codes:** 0 bei Erfolg, 1 bei Fehler (Details in der letzten Meldung). Jeder andere Code gilt als Absturz.
    *   **Bibliotheken:** Der Worker lädt dieselben FFmpeg-DLLs wie der Dienst. Die Encoder-Prüfung teilt er sich mit dem Dienst über das Crate `gameclip-media`.
    *   **Zeitlimit:** Der Dienst beendet den Worker, wenn 30 s lang keine Meldung kommt.
    *   **GPU-Priorität:** Der Worker setzt `IDXGIDevice::SetGPUThreadPriority` auf den niedrigsten erlaubten Wert.
*   **Zeitachse:** Alle Zeitangaben zwischen Oberfläche, Dienst und Worker sind ganzzahlige **Mikrosekunden der Anzeigezeit** ab dem ersten sichtbaren Bild, also nach Anwendung der Edit-List. Das entspricht `<video>.currentTime × 1 000 000`. `GetFrameTimes` liefert die Anzeigezeit jedes Bildes in dieser Einheit. Ein Schnitt `in_us`/`out_us` umfasst alle Bilder mit `in_us ≤ t < out_us` (Ende ausschließlich).
*   **Ablauf im Worker:**
    1.  Quelle mit Hardware-Decoder öffnen (`d3d11va`), sonst Software-Decoder.
    2.  Zum Keyframe vor `in_us` springen und bis `in_us` dekodieren, ohne Ausgabe.
    3.  Video ab `in_us` bis vor `out_us` mit dem Hardware-Encoder neu kodieren, bei `TargetSize` vorher auf der GPU skalieren (gemeinsames Crate `gameclip-gpu`). Lässt sich keine zweite Hardware-Encoder-Sitzung öffnen, nimmt der Auftrag `libx264` mit dem Export-Profil (`veryfast`, ohne `zerolatency`) und meldet `JobProgress{encoder: "libx264"}`; die Oberfläche zeigt „Export über CPU“.
    4.  Audio sample-genau zuschneiden und immer neu kodieren: `Mix` und `TargetSize` nehmen die Spur „Mischung“ der MP4; `SeparateTracks` schreibt zuerst „Mischung“ als Standardspur, danach „System“, „Mikrofon“ und „Discord“ aus der `.m4a`.
    5.  Muxen mit `movflags +faststart+use_metadata_tags` in `target_part_path`, Tag `gameclip_id = output_tag_id`, danach Datei mit Schreibrecht neu öffnen und `FlushFileBuffers`. Das Umbenennen übernimmt der Dienst.
*   **Zielgröße:**
    *   Ton: nur die Mischung mit 96 kbit/s. Videobitrate = `(Ziel × 8 × 0,92 − 96 000 × Dauer_s − 64 000 × 8) / Dauer_s` (64 KB Container-Reserve), mit `maxrate` = Videobitrate.
    *   **Auflösung wird einmal vorab gewählt:** die höchste Stufe aus 1080p60, 720p60, 720p30, 480p30, bei der die Videobitrate mindestens 0,05 Bit pro Pixel und Bild ergibt (Quellen in 1440p oder 2160p starten bei 1080p60). Erreicht keine Stufe 0,05, wird 480p30 genommen; unter 0,03 zeigt der Export-Dialog vorab „Qualität wird sehr niedrig“. Dieselbe Funktion liegt in `gameclip-core`, damit die Oberfläche die Warnung vor dem Start berechnen kann.
    *   Nach dem Kodieren wird die Größe geprüft. Ist die Datei zu groß, folgt **ein** Korrekturversuch mit `Videobitrate × Ziel / tatsächliche Größe × 0,97`. Ist sie dann noch zu groß, endet der Auftrag mit `JobFailed{code: TooLarge}`.
*   **Ausgabe:** `<Export-Ordner>\<Originalname>_trim.mp4`, `_tracks.mp4` bzw. `_<N>MB.mp4`, bei Namensgleichheit mit `_2`, `_3` usw. Neue `gameclip_id`, Eintrag im Index (erst `pending`, nach dem Umbenennen `ok`), dann greift das Speicherlimit des Export-Ordners. Das Original bleibt unverändert.
*   **Rückmeldung:** Ereignisse `JobProgress{job_id, percent, encoder}`, `JobCompleted{job_id, clip_id}` und `JobFailed{job_id, code, message}`. `CancelJob{job_id}` beendet den Worker und löscht die `.part`-Datei. `ListJobs` liefert nach einem Neuverbinden den laufenden und die wartenden Aufträge.

### Hotkey

*   Ein eigener Thread mit einem Message-only-Fenster ruft `RegisterHotKey` mit `MOD_NOREPEAT` auf.
*   Erlaubt sind F1–F24 außer F12, allein oder mit Strg, Alt und Shift, sowie A–Z und 0–9 nur zusammen mit Strg oder Alt (sonst würde die Taste systemweit abgefangen). F12 ist gesperrt, weil Windows die Taste für Debugger reserviert. Kombinationen mit der Windows-Taste sind gesperrt.
*   `SuspendHotkey` gibt die Taste frei, solange der Auswahldialog offen ist. `TestRegister{hotkey}` prüft nur, ob die neue Taste frei ist; übernommen wird sie erst mit `SetSettings`.
*   Schlägt die Registrierung fehl, setzt der Dienst die Einschränkung `hotkey_inactive` und meldet den Fehlercode. Wird später eine Taste erfolgreich registriert, fällt die Einschränkung weg.

### Dienst-Schnittstelle (IPC)

*   **Transport:** Named Pipe `\\.\pipe\<AppID>-<SID>` mit `FILE_FLAG_FIRST_PIPE_INSTANCE`. Ihre Sicherheitsbeschreibung erlaubt nur die SID des aktuellen Benutzers. Der Client prüft über `GetNamedPipeServerProcessId`, dass der Server `gameclipd.exe` aus dem eigenen Paket ist. Ein benannter Mutex `Local\<AppID>` stellt sicher, dass nur eine Instanz läuft. Schutzgrenze: andere Benutzer und fremde Programme ohne Administratorrechte.
*   **Nachrichtenformat (T2):** Jede Nachricht beginnt mit 4 Bytes Länge (u32, Little Endian), danach folgt UTF-8-JSON.
    *   Höchstgröße einer Nachricht: 8 MB. Größere Nachrichten werden abgelehnt und die Verbindung wird getrennt. Die größte erwartete Nachricht, `GetFrameTimes` für 5 min, hat ~300 KB.
    *   Hülle:
        *   Anfrage: `{"id": <u32>, "type": "<Befehl>", "body": {…}}`
        *   Antwort: `{"id": <u32>, "ok": true, "body": {…}}` oder `{"id": <u32>, "ok": false, "error": {"code": "<Code>", "message": "…"}}`
        *   Ereignis: `{"event": "<Name>", "body": {…}}`
    *   Felder, die der Empfänger nicht kennt, ignoriert er. Neue Felder brauchen deshalb keine neue Protokollversion.
    *   Alle Bezeichner in Protokoll, Datenbank und Enums sind englisch und ASCII. Texte in der Oberfläche sind deutsch.
    *   Fehlercodes: siehe Datei 23 des Umsetzungsplans.
*   **Handshake:** `Hello{protocol, client}`, Antwort `{protocol, service_version}`. Weicht `protocol` ab, zeigt die Oberfläche „Bitte GameClip neu starten“ und sendet nichts weiter.
*   **Befehle:**
    *   `GetStatus`, `GetSettings`, `SetSettings{partial}`, `GetForecast{settings}`, `ListDevices`, `GetCapabilities`
    *   `StartBuffer`, `StopBuffer`, `Restart`, `SaveClip`, `PauseMic{on}`, `SuspendHotkey{on}`, `TestRegister{hotkey}`, `SubscribeMicLevel{on}`, `WatchFolders{on}`
    *   `ListClips{folder}`, `GetClip{clip_id}`, `SetStar{clip_id, on}`, `AdoptClip{clip_id}`, `DeleteClip{clip_id}`, `GetFrameTimes{clip_id}`, `GetExportEstimate{clip_id, in_us, out_us, target_bytes}`
    *   `StartExportJob{…}` (Antwort `job_id`), `CancelJob{job_id}`, `ListJobs`, `CopyDiagnostics`
    *   Felder, Typen und Antworten aller Nachrichten stehen in der Nachrichtentabelle im Umsetzungsplan (Datei 24).
*   **Ereignisse:** `StateChanged`, `SaveCompleted`, `SaveFailed`, `NewClip`, `ClipDeleted`, `FormatChanged`, `JobProgress`, `JobCompleted`, `JobFailed`, `StorageWarning`.
*   **Fähigkeiten:** `GetCapabilities` meldet `{separate_tracks, max_buffer_s, forecast, encoders, hdr_supported}`. Für den späteren Linux-Adapter (gpu-screen-recorder) sind alle Statusfelder optional.
*   **Sicherheit:** Befehle enthalten nie freie Dateipfade, nur `clip_id`s. Speicherordner werden nur über `SetSettings` geändert, und der Dienst prüft sie (existiert, beschreibbar, kein Systemordner).

### Zustandsmaschine

**Hauptzustände:** Startet (Anfangszustand beim Prozessstart), Puffert, Pausiert, Konfiguriert neu, Erholt sich, Fehler. Beim Beenden endet der Prozess; dafür gibt es keinen eigenen Zustand.

**Einschränkungen** sind unabhängige Schalter neben dem Hauptzustand: `hotkey_inactive`, `cpu_encoder`, `hdr_active`. Das Tray zeigt „Eingeschränkt“, sobald mindestens einer gesetzt ist, mit den Gründen im Tooltip.

**Mikrofon pausiert** (`mic_paused`) ist eine bewusste Nutzeraktion und keine Einschränkung. Es hat ein eigenes Tray-Symbol.

**Speichern** ist kein Zustand, sondern ein Zähler `saves_in_flight` (0, 1 oder 2: einer läuft, einer wartet). Es ist in „Puffert“ möglich, unabhängig von den Einschränkungen.

**Nicht aufgeführte Kombinationen** aus Zustand und Ereignis werden ignoriert und protokolliert.

| Zustand | Ereignis | Neuer Zustand | Wirkung |
| --- | --- | --- | --- |
| Startet | alles bereit | Puffert | Tray „Puffert“ |
| Startet | Hotkey fehlgeschlagen | Puffert + `hotkey_inactive` | Tray-Warnung, Fehlerton, Benachrichtigung |
| Startet | kein Encoder außer x264 | Puffert + `cpu_encoder` | Tray-Warnung, Benachrichtigung |
| Startet | HDR aktiv | Puffert + `hdr_active` | Tray-Warnung |
| Startet | keine Aufnahme möglich | Fehler | Tray-Fehler, Benachrichtigung |
| Puffert | Hotkey, `saves_in_flight` < 2 | Puffert | Speichervorgang starten oder einreihen |
| Puffert | Hotkey, `saves_in_flight` = 2 | Puffert | ignoriert |
| Puffert | Speichern fertig / fehlgeschlagen | Puffert | Erfolgs- bzw. Fehlerton, `SaveCompleted` / `SaveFailed` |
| Puffert | pipeline-relevante Einstellung geändert | Konfiguriert neu | laufende Speichervorgänge abschließen, dann Puffer neu anlegen |
| Konfiguriert neu | fertig | Puffert | Tray „Puffert“ |
| Puffert | Geräteverlust | Erholt sich | laufender Speichervorgang wird mit den gehaltenen Blöcken abgeschlossen; Neuaufbau mit steigender Wartezeit |
| Startet / Konfiguriert neu / Pausiert | Geräteverlust | Erholt sich | Neuaufbau mit steigender Wartezeit |
| Erholt sich | Erfolg | Puffert | neuer Abschnitt |
| Erholt sich | Neuaufbau mehrmals fehlgeschlagen | Fehler | Tray-Fehler, Benachrichtigung |
| Fehler | „Neu starten“ im Tray, `Restart` oder Einstellung geändert | Startet | – |
| Puffert | `StopBuffer` (Tray „Aufnahme pausieren“) | Pausiert | Puffer leeren, Tray-Symbol |
| Pausiert | `StartBuffer` | Startet | – |
| Puffert | PC gesperrt (`SessionLocked`) | Pausiert | Puffer leeren, Tray-Symbol; Merker `paused_by_lock` |
| Pausiert (mit `paused_by_lock`) | PC entsperrt (`SessionUnlocked`) | Startet | – |
| beliebig | „Beenden“ im Tray | Prozess endet | laufende Speichervorgänge abschließen, laufenden Export abbrechen |
| Startet / Konfiguriert neu / Erholt sich / Pausiert / Fehler | Hotkey | unverändert | Fehlerton „Puffer nicht bereit“ |
| beliebig | neue Taste per `SetSettings` erfolgreich registriert (`HotkeyRegistered`) | unverändert | `hotkey_inactive` fällt weg |
| beliebig | Registrierung fehlgeschlagen (`HotkeyFailed`) | unverändert | `hotkey_inactive` setzen, Fehlerton |
| beliebig | HDR an/aus, nur CPU-Encoder nach Neuaufbau | unverändert | Einschränkung setzen bzw. löschen, Tray aktualisieren |
| beliebig | Speichervorgang fertig (auch während „Konfiguriert neu“ und „Erholt sich“) | unverändert | Zähler verringern, Ton, Ereignis, wartenden Vorgang starten |
| beliebig | Mikrofon pausiert / fortgesetzt | unverändert | `mic_paused` setzen/löschen, Mikrofon-Quelle liefert Stille |
| beliebig | Oberfläche trennt die Verbindung, während `SuspendHotkey` aktiv ist | unverändert | Hotkey wieder registrieren |

*   Pipeline-relevant sind: Pufferlänge, Auflösung, Bildrate, Qualitätsstufe, Monitor, Mauszeiger und Audioquellen. Andere Einstellungen (Ordner, Limits, Töne, Hotkey) gelten ohne Neuaufbau.
*   Platte voll oder fehlender Ordner beim Speichern führt nur zu `SaveFailed`, nicht zu einem Zustandswechsel.
*   Exportaufträge sind unabhängig von der Zustandsmaschine und laufen in jedem Zustand.
*   Wartezeiten und Anzahl der Versuche beim Neuaufbau sind Konstanten im Umsetzungsplan.

### Speicherorte

| Was | Ort |
| --- | --- |
| Einstellungen | `%APPDATA%\GameClip\settings.toml` mit `schema_version` |
| Clip-Index | `%LOCALAPPDATA%\GameClip\index.sqlite` |
| Einzelspuren | `%LOCALAPPDATA%\GameClip\tracks\<UUID>.m4a` |
| Logs | `%LOCALAPPDATA%\GameClip\logs\` (täglich rotierend, höchstens 7 Dateien) |
| Clips | vom Nutzer gewählter Clip-Ordner (Standard `<Videos>\GameClip`) |
| Exporte | vom Nutzer gewählter Export-Ordner (Standard `<Videos>\GameClip Export`) |

*   `<Videos>` wird zur Laufzeit über `SHGetKnownFolderPath(FOLDERID_Videos)` ermittelt. Liegt er in OneDrive, ist der Standard `%USERPROFILE%\GameClip` bzw. `%USERPROFILE%\GameClip Export` (siehe „Speicherlimit und Löschen“).
*   **Einstellungen:** Schema in Datei 16 des Umsetzungsplans.
    *   Unbekannte Felder bleiben erhalten.
    *   Ist `schema_version` neuer als die App, liest der Dienst die Datei nur und überschreibt sie nicht.
    *   Migrationen werden mit der ersten Schema-Änderung eingeführt; vor jeder Migration wird dann `settings.toml.bak` angelegt.
*   **Standardordner:** Clip- und Export-Ordner sind standardmäßig zwei Ordner nebeneinander, damit jeder sein eigenes Limit hat.

### Oberfläche

*   Tauri 2 mit WebView2. Die Oberfläche spricht nur über die Named Pipe mit dem Dienst und lädt keine FFmpeg-Bibliothek.
*   **Dienst starten:** Erreicht die Oberfläche keine Pipe, startet sie `gameclipd` über dessen AUMID (`…!GameClipService`) und zeigt „Dienst wird gestartet …“. Beim ersten Start ruft sie `StartupTask.RequestEnableAsync` auf, damit der Autostart aktiv ist.
*   **Frontend (T5): React mit TypeScript, gebaut mit Vite.**
    *   **Kein Router.** Es gibt drei Ansichten (Galerie, Editor, Einstellungen) und den Ablauf für den ersten Start. Gewechselt wird über einen Zustand.
    *   **IPC-Anbindung.** Die Rust-Seite von Tauri hält die Pipe-Verbindung und leitet Ereignisse als Tauri-Events weiter. Im Frontend kapselt ein Modul `ipc.ts` die Befehle als typisierte Funktionen. Ein Store auf Basis von `useSyncExternalStore` hält Status und Clipliste, es gibt keine zusätzliche State-Bibliothek. Die TypeScript-Typen werden aus `gameclip-protocol` erzeugt (`ts-rs`), damit beide Seiten dieselben Felder kennen.
    *   **Kürzen-Leiste ohne Neu-Rendern.** Die Wiedergabeposition läuft über `requestVideoFrameCallback` und `ref`s direkt ins DOM. React rendert nur neu, wenn Start- oder Endpunkt geändert werden.
    *   **Gestaltung.** Eigene CSS-Variablen für Farben, Abstände und Schrift (helles und dunkles Thema nach Windows-Einstellung), CSS Modules je Komponente, keine UI-Bibliothek. Jede Ansicht zeigt nur die Bedienelemente, die sie braucht.
*   **Galerie:** Zwei Reiter, „Clips“ und „Exporte“. Vorschaubilder kommen von der Windows-Shell (`IShellItemImageFactory`). Angezeigt werden Stern, Dauer, Größe und belegter Platz im Verhältnis zum Limit.
*   **Player und Kürzen (T8):** `<video>` lädt die Datei über ein eigenes Tauri-Protokoll `clip`. Unter Windows erreicht WebView2 es als `http://clip.localhost/<clip_id>`; die CSP erlaubt dafür `media-src http://clip.localhost`. Die Rust-Seite fragt den Dienst nach dem Pfad und liefert die Datei mit HTTP-Range aus (206 mit `Content-Range`, 404 für unbekannte IDs, 416 für ungültige Bereiche). Spulen in einem 5-min-Clip wird in S5 geprüft. Für bildgenaues Setzen holt die Oberfläche mit `GetFrameTimes` die Zeitstempel aller Bilder und rastet Start und Ende auf diese ein. `requestVideoFrameCallback` zeigt das aktuelle Bild an. Pfeiltasten springen ein Bild weiter oder zurück.
*   **Export-Dialog:** Wahl zwischen „Mischung“ (Standard), „Mit getrennten Spuren“ und „Zielgröße“ mit einstellbaren MB.
*   **Teilen:**
    *   Datei in die Zwischenablage als `CF_HDROP`, über `clipboard-win`.
    *   Drag-and-drop aus der Galerie über das Crate `drag` in `gameclip-ui-platform`.
    *   „Im Ordner anzeigen“ über `SHOpenFolderAndSelectItems`.
*   **Erster Start:** Mikrofon bestätigen oder wechseln (standardmäßig an, mit dem Hinweis, dass es dauerhaft aufgenommen wird), dann Hotkey mit Testdruck und Warnung, dann Clip-Ordner. Danach „Jetzt Test-Clip erstellen“.
*   **Windows 11 N/KN:** Fehlt das Media Feature Pack, zeigt der erste Start, wie es installiert wird.

### Tray, Töne und Benachrichtigungen

*   **Tray:** Sechs Symbole. Gilt mehr als eines, gewinnt das erste in dieser Reihenfolge: Fehler, Speichert (während `saves_in_flight` > 0), Pausiert bzw. nicht bereit (Startet, Konfiguriert neu, Erholt sich), Eingeschränkt, Mikrofon pausiert, Puffert. Der Tooltip nennt immer alle zutreffenden Punkte. Menü: Clip speichern, Aufnahme pausieren/fortsetzen, Mikrofon pausieren, Galerie öffnen, Einstellungen, Neu starten (nur im Zustand Fehler), Beenden.
*   **Töne:** Zwei kurze WAV-Dateien, eine für Erfolg und eine für Fehler. Der Dienst spielt sie über WASAPI ab, die Lautstärke ist einstellbar und sie lassen sich abschalten.
*   **Benachrichtigungen:** Windows-Toasts über die AUMID des MSIX-Pakets, Text „Clip gespeichert (1:00, 142 MB)“ bzw. der Fehlergrund. Fokus-Assistent kann Benachrichtigungen unterdrücken. Der Ton bleibt deshalb das Hauptsignal.

### Plattformgrenze und Crates

| Crate | Inhalt | Windows-Code erlaubt |
| --- | --- | --- |
| `gameclip-core` | Zustandsmaschine, Einstellungsschema, Prognoseformel, Speicherlimit-Logik, Dateinamen | nein |
| `gameclip-protocol` | Befehle, Ereignisse, Versionen, Rahmung (Längenpräfix + JSON) | nein |
| `gameclip-gpu` | D3D11-Gerät, Adapterwahl, VideoProcessor (Skalieren, BGRA/NV12); genutzt von Aufnahme und Worker | ja |
| `gameclip-media` | FFmpeg-Anbindung über `rsmpeg`, Encoder-Prüfung und Ausweichkette, Muxer, Dekoder, Hilfen für Hardware-Frames; genutzt von Dienst und Worker | ja |
| `gameclip-capture-win` | WGC, Taktgeber, WASAPI, Mixer, Ringpuffer | ja |
| `gameclipd` | Dienstprogramm: Hotkey, Tray, Töne, Index, Warteschlange, IPC-Server | ja |
| `gameclip-ui-platform` | Windows-Funktionen der Oberfläche: Pipe-Client, Start des Dienstes, Autostart, Zwischenablage, Drag-out, Explorer, Vorschaubilder | ja |
| `gameclip-ui` | Tauri-App (Rust) und Frontend (React, TypeScript, Vite) | Rust-Seite nur über `gameclip-ui-platform`; Frontend nein |
| `gameclip-worker` | Worker für Kürzen und Export | ja |

Durchgesetzt wird das so:
*   Ein CI-Job auf `ubuntu-latest` testet `gameclip-core`, `gameclip-protocol` und das Frontend (`npm test` mit gemocktem `ipc.ts`).
*   Ein CI-Schritt listet alle Abhängigkeiten von `gameclip-core` und `gameclip-protocol` für das Windows-Ziel (`cargo tree … --prefix none --format "{p}"`) und bricht ab, wenn darin `windows` oder `windows-sys` vorkommt; dasselbe für die direkten Abhängigkeiten von `gameclip-ui` (genauer Befehl in Datei 7).

### Diagnose

*   **Logs:** `tracing` mit rotierenden Dateien.
*   **Je Speichervorgang:** Dauer, Bytes, Anzahl der Bildgruppen, Encoder, Ergebnis.
*   **Je Minute:** aufgenommene, wiederholte und verworfene Bilder, Länge der Encoder-Warteschlange, Puffergröße im Vergleich zur Prognose, Audio-Drift und aufgefüllte Stille je Quelle, Lücken im Puffer.
*   **Ereignisse:** Wechsel des Aufnahmewegs und des Encoders, Ergebnis der Hotkey-Registrierung mit Fehlercode.
*   **„Diagnose kopieren“:** Windows-Build, GPU und Treiber, Ergebnis der Encoder-Prüfung, Audiogeräte, die letzten 200 Logzeilen. Benutzerpfade werden unkenntlich gemacht. Nichts davon wird gesendet.

### Abhängigkeiten und Lizenz

*   **FFmpeg:** FFmpeg 8.1.x als GPL-Shared-Build (gepinnt in `packaging/ffmpeg/ffmpeg.lock`), mitgeliefert als DLLs, mit Quellcode-Angebot. Ohne `--enable-nonfree`, ohne fdk-aac. Anbindung über `rsmpeg`. Weil BtbN alte Release-Dateien entfernt, wird das gewählte Archiv als Release-Datei im eigenen Repository gespiegelt (zugleich Quellcode-Angebot). Lehnt die Signatur-Stelle vorgebaute Binärdateien ab (Q14), wird FFmpeg in CI aus der festen Quellversion gebaut. Skaliert wird mit dem D3D11-VideoProcessor aus `gameclip-gpu`, nicht mit FFmpeg-Filtern.
*   **Rust-Bibliotheken (Q16):** Versionen und Features in Datei 1 des Umsetzungsplans. `windows-capture` dient nur als Vorlage, weil die Pipeline ein eigenes, mit FFmpeg geteiltes D3D11-Gerät braucht. `global-hotkey` wird nicht genutzt, weil `SuspendHotkey`, `TestRegister` und der QPC-Zeitstempel des Tastendrucks einen eigenen Hotkey-Thread brauchen.
*   **Lizenz des Projekts:** GPL-3.0-or-later.
*   **Signatur:** SignPath Foundation (Q14); Ausweg siehe Risiko R3. Für Entwicklungs-Builds ein selbst erzeugtes Testzertifikat.
*   **Updates:** über winget. Ein Tauri-Updater ist nicht eingebaut.

### Paket und Installation (T1: volles MSIX)

*   **Inhalt des Pakets:** `gameclipd.exe`, `gameclip-ui.exe`, `gameclip-worker.exe`, FFmpeg-DLLs, Töne und Lizenztexte. Beide Programme sind als Anwendungen mit eigener ID im Manifest eingetragen. Nur `gameclip-ui` erscheint im Startmenü.
*   **Fähigkeiten im Manifest:** `rescap:Capability runFullTrust` (Win32-Desktop-App), `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` (Namespace laut Q15 zu prüfen), `DeviceCapability microphone`.
*   **Benachrichtigungen mit Knopf:** „Im Ordner anzeigen“ in der Benachrichtigung braucht eine Aktivierung über das Protokoll `gameclip:` (`uap:Protocol` im Manifest), die `gameclip-ui` mit `--reveal <clip_id>` startet.
*   **Autostart:** `windows.startupTask` für `gameclipd`. Der Nutzer kann ihn in den Windows-Einstellungen und in GameClip abschalten. Ist er aus, startet die Oberfläche den Dienst bei Bedarf.
*   **App-ID:** `io.github.poldion.GameClip`. Die AUMIDs lauten `<PackageFamilyName>!GameClip` (Oberfläche) und `<PackageFamilyName>!GameClipService` (Dienst). Pipe-Name und Mutex leiten sich aus der App-ID ab. Die App-ID wird nie geändert.
*   **Gebaut wird außerhalb von Tauri.** Tauri baut nur die Programme. Ein eigenes Skript `packaging/build-msix.ps1` erzeugt `AppxManifest.xml` aus einer Vorlage und packt mit `MakeAppx.exe`. Signiert wird mit `SignTool.exe`.
*   **Virtualisierung:** Schreibzugriffe auf `%APPDATA%` und `%LOCALAPPDATA%` leitet Windows bei MSIX-Apps in einen paketeigenen Ordner um. Einstellungen, Index, Einzelspuren und Logs landen dort, und die Deinstallation entfernt sie. Clip- und Export-Ordner sind nicht betroffen, Clips bleiben also nach der Deinstallation erhalten. Nach einer Neuinstallation werden sie automatisch mit Stern übernommen (siehe „Clip-Index“). „Diagnose kopieren“ zeigt den tatsächlichen Pfad.
*   **Prüfung in S1:** Weil sich MSIX-Apps anders verhalten können, prüft S1 im Paket: Autostart startet gameclipd beim Login; die Pipe ist von gameclip-ui erreichbar; Prozess-Loopback liefert Ton; WGC nimmt ohne gelben Rahmen und ohne Rückfrage auf. S1 läuft deshalb von Anfang an als MSIX.

## Alternatives Considered

Alle Ablehnungen verweisen auf das [Entscheidungsregister im Brief](../../concepts_brief.md#entscheidungsregister) (D-IDs) bzw. auf die Architekturentscheidungen T1–T8 oben. „Rejected by author (Review Stufe 7)“ heißt: In der Review vorgeschlagen und vom Autor per Auto-Accept übernommen; dafür gibt es keine eigene D-ID.

### Produkt und Umfang

#### Alternative A: Windows und Ubuntu gleichzeitig in Version 1

*   **Pros**: Keine spätere Portierung; die gemeinsame Schicht wird von Anfang an an zwei Systemen geprüft.
*   **Cons**: Zwei Aufnahmekerne (eigener Windows-Kern, gpu-screen-recorder unter Linux) gleichzeitig vor dem Vanguard-Test. Unter Linux kommen Wayland-Freigaben, Hotkey-Portale, fehlende Trays und NVIDIA ohne VA-API hinzu.
*   **Reason for Rejection**: Rejected by author (D6): Mit gpu-screen-recorder unter Linux betrifft eine spätere Portierung nur Kernlogik, Protokoll, Frontend und Einstellungen. Diese Teile bleiben per CI frei von Windows-Code. Zwei Plattformen vor dem Vanguard-Nachweis wären das größere Risiko.

#### Alternative B: Eigener Linux-Aufnahmekern

*   **Pros**: Gleiches Puffer- und Spurmodell auf beiden Systemen.
*   **Cons**: PipeWire, Portal, DMA-BUF, VA-API und NVENC unter Linux selbst anbinden, mit allen Treiber-Sonderfällen, die gpu-screen-recorder bereits behandelt.
*   **Reason for Rejection**: Rejected by author (D6): gpu-screen-recorder (GPL-3) deckt Replay-Puffer, Wayland, X11, VA-API und NVENC ab und passt zur Projektlizenz; der eigene Aufwand stünde in keinem Verhältnis.

#### Alternative C: Windows 10 unterstützen

*   **Pros**: Mehr mögliche Nutzer.
*   **Cons**: Der gelbe Aufnahmerahmen lässt sich nicht abschalten, für Prozess-Loopback sind zusätzliche Versionsprüfungen nötig, und es gibt eine zweite Testplattform.
*   **Reason for Rejection**: Rejected by author (D9): Der Support für Windows 10 endete im Oktober 2025; der zusätzliche Test- und Code-Aufwand lohnt sich für ein auslaufendes System nicht.

#### Alternative D: Erste Version nur mit Tray, ohne Galerie und Kürzen

*   **Pros**: Etwa halber Umfang, schnellere erste Version.
*   **Cons**: Der Ablauf Kürzen → Discord-Export → Zwischenablage ist der wahrscheinliche Vorteil gegenüber Game Bar (G1) und fehlte dann.
*   **Reason for Rejection**: Rejected by author (D11): Ohne Galerie und Kürzen fehlt der Ablauf, der laut G1 der Vorteil gegenüber Game Bar ist; der doppelte Umfang wird dafür in Kauf genommen.

#### Alternative E: Overlay im Spiel

*   **Pros**: Sichtbare Bestätigung direkt im Spiel.
*   **Cons**: Braucht Code-Injektion oder Grafik-Hooks; Vanguard blockiert oder meldet das.
*   **Reason for Rejection**: Rejected by author (D3): Verletzt die Regel „keine Injektion“. Ton und Benachrichtigung übernehmen die Bestätigung.

#### Alternative F: Cloud-Upload zum Teilen

*   **Pros**: Teilen per Link, keine Größenlimits von Messengern.
*   **Cons**: Braucht Server, Konten und Internet.
*   **Reason for Rejection**: Rejected by author (D1): GameClip bleibt offline und ohne Konto; geteilt wird über Zwischenablage und Drag-and-drop, die Größenlimits deckt der Größen-Export ab.

#### Alternative G: Steam OS in Version 1

*   **Pros**: Steam-Deck-Nutzer als Zielgruppe.
*   **Cons**: Im Spielmodus kontrolliert Steam die Eingaben, globale Hotkeys funktionieren dort nicht zuverlässig, und Steam hat eine eigene Hintergrundaufnahme.
*   **Reason for Rejection**: Rejected by author (Non-Goal im Brief): Der Aufwand für eine Plattform ohne zuverlässige globale Hotkeys und mit eigener Aufnahme steht in v1 in keinem Verhältnis.

### Aufnahme und Encoding

#### Alternative H: Aufnahme per Code-Injektion („Game Capture“)

*   **Pros**: Funktioniert auch bei exklusivem Vollbild, etwas geringere Latenz.
*   **Cons**: Lädt Code in den Spielprozess, Anti-Cheat erkennt und blockiert das.
*   **Reason for Rejection**: Rejected by author (D3): Das Risiko einer Kontosperre in Valorant wiegt schwerer als die bessere Abdeckung von exklusivem Vollbild.

#### Alternative I: Puffer aus Rohbildern

*   **Pros**: Kein Encoder im Hintergrund nötig.
*   **Cons**: 1080p60 in NV12 sind ~186 MB pro Sekunde, eine Minute also über 11 GB.
*   **Reason for Rejection**: Rejected by author (D2, vorgeschlagen in Review-Runde 1): passt nicht in den RAM.

#### Alternative J: Ringpuffer als Datei auf der SSD

*   **Pros**: Beliebig lange Puffer ohne RAM-Kosten.
*   **Cons**: Ständige Schreiblast auf der SSD, mehr Fehlerfälle (Platte voll, langsame Datenträger).
*   **Reason for Rejection**: Rejected by author (D2): Ein 5-min-Ringpuffer bei 14 Mbit/s schreibt pro Spielstunde ~6 GB auf die SSD; dafür sinkt der RAM-Bedarf. v1 begrenzt stattdessen die Länge über die RAM-Prognose.

#### Alternative K: Fehler statt CPU-Encoder, wenn keine Hardware-Unterstützung da ist

*   **Pros**: Kein unerwarteter CPU-Verbrauch im Spiel.
*   **Cons**: Ohne passende GPU funktioniert die App gar nicht.
*   **Reason for Rejection**: Rejected by author (D4): Ohne passende GPU gäbe es gar keine Aufnahme; höhere CPU-Last mit sichtbarer Einschränkung ist das kleinere Übel.

#### Alternative L: Verlustfreies Kürzen nur an Keyframes

*   **Pros**: Sehr schnell, kein Qualitätsverlust, kein Encoder nötig.
*   **Cons**: Schnitt nur auf 1 bis 2 Sekunden genau.
*   **Reason for Rejection**: Rejected by author (D12): Kürzen soll bildgenau sein; die längere Exportzeit wird in Kauf genommen. Smart-Cut bleibt als spätere Beschleunigung (Q7).

#### Alternative AA: DDA statt WGC als Hauptweg

*   **Pros**: Liefert in manchen Vollbild-Modi ein Bild, in denen WGC versagen könnte.
*   **Cons**: Nur ganze Monitore, Mauszeiger muss selbst gezeichnet werden, Verhalten bei mehreren GPUs schwieriger.
*   **Reason for Rejection**: Rejected by author (Review Stufe 7): WGC ist unter Windows 11 die vorgesehene Schnittstelle und deckt Monitor-Aufnahme mit Mauszeiger ab. DDA bleibt Ausweg, falls S1 es nötig macht (Q4).

#### Alternative AC: HEVC oder AV1 statt H.264

*   **Pros**: Bei 10 MB sichtbar bessere Qualität.
*   **Cons**: Wiedergabe und Hardware-Encoding nicht auf allen Zielgeräten gesichert (ältere Handys, WhatsApp-Vorschau, Discord-Einbettung im Browser je nach Codec, zu prüfen); ältere GPUs haben keinen AV1-Encoder.
*   **Reason for Rejection**: Rejected by author (Review Stufe 7): G5 verlangt, dass Clips überall abspielbar sind; H.264 erfüllt das. Ein AV1-Export ist eine spätere Option.

#### Alternative AD: Hersteller-SDKs oder Media Foundation direkt statt FFmpeg

*   **Pros**: Eine Abhängigkeit weniger.
*   **Cons**: Drei bis vier getrennte Anbindungen (NVENC SDK, AMF, oneVPL, Media Foundation) und ein eigener Muxer.
*   **Reason for Rejection**: Rejected by author (Review Stufe 7, folgt aus D5): FFmpeg deckt alle Encoder, den Muxer und den Worker mit einer Schnittstelle ab.

#### Alternative AE: Variable Bildrate statt festem 60-fps-Takt

*   **Pros**: Weniger Bitrate bei Standbildern.
*   **Cons**: RAM-Prognose, A/V-Abgleich und bildgenaues Kürzen werden unzuverlässig.
*   **Reason for Rejection**: Rejected by author (Review Stufe 7): Vorhersagbarer Speicher und bildgenaues Kürzen wiegen schwerer als die gesparte Bitrate.

### Audio

#### Alternative M: Nur eine gemischte Spur, keine Einzelspuren

*   **Pros**: Einfacherer Puffer und Muxer, weniger Speicher.
*   **Cons**: Mikrofon und System lassen sich später nicht getrennt bearbeiten.
*   **Reason for Rejection**: Rejected by author (D7): Einzelspuren kosten ~1 MB RAM je Spur und Minute; dafür ist getrenntes Exportieren jederzeit möglich.

#### Alternative N: Einzelspuren nur über eine Aufnahme-Einstellung

*   **Pros**: Etwas weniger RAM, wenn die Einstellung aus ist.
*   **Cons**: Wer die Einstellung vergisst, kann später nicht mehr getrennt exportieren.
*   **Reason for Rejection**: Rejected by author (D7): Die Ersparnis (~1 MB je Spur und Minute) ist kleiner als der Nachteil, getrennte Spuren nicht nachträglich zu haben.

#### Alternative O: Einzelspuren im Original-Clip statt intern

*   **Pros**: Alles in einer Datei, keine interne Zuordnung nötig.
*   **Cons**: Dateien im Clip-Ordner hätten mehrere Spuren und wären größer.
*   **Reason for Rejection**: Rejected by author (D7): Mehrspurige Dateien spielen auf Handys und in WhatsApp nur die erste Spur und sind größer; die interne `.m4a` kostet dafür eine Zuordnung über den Index.

#### Alternative P: Mischregler im Editor

*   **Pros**: Lautstärke pro Clip nachträglich anpassen.
*   **Cons**: Braucht eine eigene Vorschau-Mischung, weil WebView2 nur eine Spur abspielt.
*   **Reason for Rejection**: Rejected by author (Non-Goal im Brief): Die eigene Vorschau-Mischung geht über das Ziel von v1 hinaus. Lautstärkeregler gibt es nur in den Einstellungen und nur für die Mischung.

#### Alternative AB: Normales Endpoint-Loopback statt Prozess-Loopback

*   **Pros**: Einfacher, keine Prozess-IDs nötig.
*   **Cons**: Nimmt GameClips eigenen Bestätigungston auf und bei aktiver Discord-Spur Discord doppelt.
*   **Reason for Rejection**: Rejected by author (D26): Prozess-Loopback hält eigene Töne und doppelten Discord-Ton aus der Mischung.

### Bedienung

#### Alternative Q: Standard-Hotkey mit Modifier (z. B. Alt+F8)

*   **Pros**: Geringere Gefahr, eine Spieltaste zu belegen.
*   **Cons**: Umständlicher im Spiel.
*   **Reason for Rejection**: Rejected by author (D10): Ein Modifier verhindert Kollisionen, ist im Spiel aber umständlicher; da der Hotkey frei belegbar ist und der Auswahldialog auf das Risiko hinweist, bleibt F8.

#### Alternative R: Tastatur-Hooks (`WH_KEYBOARD_LL`)

*   **Pros**: Die Taste erreicht weiterhin das Spiel.
*   **Cons**: Wirkt auf Anti-Cheat wie ein Keylogger und empfängt nichts, wenn ein Spiel mit Administratorrechten den Fokus hat.
*   **Reason for Rejection**: Rejected by author (D10, vorgeschlagen in Review-Runde 1): Das Anti-Cheat-Risiko und die Blindheit bei Admin-Spielen wiegen schwerer als die durchgereichte Taste.

### Speicher

#### Alternative S: Automatisches Löschen in den Papierkorb

*   **Pros**: Versehentlich gelöschte Clips lassen sich wiederherstellen.
*   **Cons**: Der Papierkorb liegt auf derselben Platte, der Platz wird nicht frei, und das Limit wirkt nicht.
*   **Reason for Rejection**: Rejected by author (D24): Ein Limit, das keinen Platz freigibt, erfüllt G9 nicht; Schutz bietet stattdessen der Stern.

#### Alternative AF: NTFS-Alternate-Data-Stream oder Begleitdatei statt MP4-Tag als Eigentumsmerkmal

*   **Pros**: Die Videodatei bleibt unverändert.
*   **Cons**: ADS geht beim Kopieren auf FAT/exFAT, in Cloud-Ordnern und beim Upload verloren; eine Begleitdatei je Clip verdoppelt die Dateien im Ordner.
*   **Reason for Rejection**: Rejected by author (Review Stufe 7): Der MP4-Tag reist mit der Datei und wird beim Schreiben ohnehin gesetzt.

### Architektur

#### Alternative T: Electron statt Tauri

*   **Pros**: Mitgeliefertes Chromium, auf allen Systemen gleiches Verhalten.
*   **Cons**: Etwa 100 bis 200 MB mehr RAM und ein um ~100 MB größeres Paket.
*   **Reason for Rejection**: Rejected by author (D5): G3 verlangt wenig Verbrauch; Tauri nutzt das vorhandene WebView2.

#### Alternative U: Ein einziger Prozess für Dienst und Oberfläche

*   **Pros**: Keine IPC, weniger Code.
*   **Cons**: Ein Hänger oder Absturz der WebView reißt Aufnahme und Puffer mit. WebView2-Prozesse bleiben im Speicher, auch wenn das Fenster zu ist.
*   **Reason for Rejection**: Rejected by author (D8, vorgeschlagen in Review-Runde 1): Ein Prozess spart IPC, aber ein Hänger der WebView stoppt den Puffer; der Puffer hat Vorrang.

#### Alternative AG: Plattformneutrale Adapter-Abstraktion für Puffer, Muxer und Prognose

*   **Pros**: Gleiche Schnittstelle für Windows und Linux.
*   **Cons**: gpu-screen-recorder verwaltet seinen Puffer unter Linux selbst; die Abstraktion würde nie wiederverwendet.
*   **Reason for Rejection**: Rejected by author (D6, Review-Runde 2 im Brief): Puffer, Muxer und Prognose sind normaler Windows-Code ohne Adapter-Schicht.

#### Alternative V: NSIS/MSI über Tauri oder MSI plus Sparse Package (T1)

*   **Pros**: NSIS/MSI ist der Standardweg von Tauri mit dem geringsten Aufwand. Das Sparse Package gibt eine App-Identität bei klassischer Installation.
*   **Cons**: NSIS/MSI hat keine Paket-Identität; ob randlose Aufnahme ohne Rückfrage geht, ist ungeprüft, und Benachrichtigungen brauchen eine Startmenü-Verknüpfung mit AUMID. Das Sparse Package ist wenig dokumentiert, Fehler sind nur über das Ereignisprotokoll sichtbar.
*   **Reason for Rejection**: Rejected by author (T1): Volles MSIX gibt eine Paket-Identität mit fester AUMID, deklarierbare Fähigkeiten und Benachrichtigungen ohne eigene Verknüpfung. Der Preis sind ein eigener Build-Schritt, Pflicht zur Signatur und ein virtualisierter App-Ordner. Bleibt als Ausweg, falls Q14 scheitert (R3).

#### Alternative W: JSON pro Zeile oder MessagePack auf der Pipe (T2)

*   **Pros**: JSON pro Zeile lässt sich von Hand testen. MessagePack ist kompakt und schnell.
*   **Cons**: JSON pro Zeile verlässt sich auf Zeilenumbrüche als Grenze. MessagePack ist ohne Hilfswerkzeug nicht lesbar.
*   **Reason for Rejection**: Rejected by author (T2): Längenpräfix plus JSON ist lesbar, hat eindeutige Nachrichtengrenzen und eine prüfbare Höchstgröße.

#### Alternative X: Index nur in Metadaten-Tags oder als JSON-Datei (T3)

*   **Pros**: Tags überleben eine Neuinstallation. Eine JSON-Datei braucht keine Datenbank.
*   **Cons**: Um einen Stern per Tag zu setzen, muss die Videodatei umgeschrieben werden, und kopierte Tags machen fremde Dateien zu „eigenen“. Die JSON-Datei wird bei jeder Änderung komplett neu geschrieben.
*   **Reason for Rejection**: Rejected by author (T3): SQLite mit atomaren Schreibvorgängen und schnellen Abfragen. Nach Neuinstallation oder Beschädigung werden alle getaggten Clips automatisch mit Stern übernommen; die alten Sterne gehen verloren, gelöscht wird nichts.

#### Alternative Y: Mitgelieferte `ffmpeg.exe` als Arbeitsprozess (T4)

*   **Pros**: Wenig eigener Code, FFmpeg kann alles Nötige bereits.
*   **Cons**: Weniger Kontrolle über Encoder-Auswahl und Fehlercodes, Smart-Cut ist schwer umsetzbar, Fehler kommen als Text.
*   **Reason for Rejection**: Rejected by author (T4, in Review Stufe 5, 8 und 9 erneut vorgeschlagen, vom Autor bestätigt): Eigenes Programm `gameclip-worker` mit denselben Encoder-Regeln wie der Dienst. Der Preis ist mehr eigener Code (Dekodieren, Schneiden, Muxen); ein Grundgerüst entsteht schon in M0 für S9.

#### Alternative Z: Svelte oder SolidJS (T5)

*   **Pros**: Kleineres Bundle, weniger Neu-Rendern bei sich ständig ändernden Werten.
*   **Cons**: Kleineres Ökosystem, weniger fertige Bausteine, weniger mögliche Mitwirkende.
*   **Reason for Rejection**: Rejected by author (T5): React wegen Ökosystem und Bekanntheit. Das Risiko des Neu-Renderns in der Kürzen-Leiste wird durch direkte DOM-Updates abgefangen.

--------------------------------------------------------------------------------

## Risks & Mitigations

| ID | Risiko | Art | Auswirkung | Gegenmaßnahme | Verweis |
| --- | --- | --- | --- | --- | --- |
| R1 | Vanguard blockiert, meldet oder sperrt | Anti-Cheat | Kernziel G4 verfehlt | Keine Injektion, keine Hooks, kein Handle auf den Spielprozess; Test mit separatem Konto; Abbruch des Projekts, wenn S1 nicht besteht | G4, S1 |
| R2 | Exklusives Vollbild liefert über WGC kein Bild | Technik | schwarze Clips in manchen Spielen | Erkennung schwarzer Bilder, Ausweg DDA | Q4, S1 |
| R3 | SignPath kann kein MSIX mit fremden DLLs signieren | Verteilung | T1 nicht umsetzbar | Prüfung vor S1; Ausweg 1: FFmpeg selbst bauen; Ausweg 2: Azure Trusted Signing; Ausweg 3: Alternative V. Weil der Publisher am Zertifikat hängt, ist die Wahl vor dem ersten öffentlichen Release (M5) endgültig | Q14, Q2 |
| R4 | Export stört das laufende Spiel | Leistung | Ruckeln im Spiel | Niedrige CPU- und GPU-Priorität; Kriterium in S9; falls nicht erreicht: Option „Export erst nach dem Spiel“ | S9 |
| R5 | Index-Verlust nimmt Sterne mit | Datenverlust | geschützte Clips würden gelöscht | Getaggte Clips werden beim Neuaufbau automatisch mit Stern übernommen; gelöscht wird nichts, nur die alten Sterne gehen verloren | Clip-Index |
| R6 | Clip-Ordner liegt in OneDrive | Datenverlust, Leistung | Löschen synchronisiert in die Cloud, Platzhalter werden geladen | Einmalige Warnung bei Auswahl; Platzhalter zählen mit 0 Bytes, werden nie gelesen oder automatisch gelöscht; liegt der Videos-Ordner in OneDrive, ist der Standard ein lokaler Ordner | Speicherlimit |
| R7 | MSIX-Deinstallation löscht Einzelspuren und Index | Datenverlust | getrennter Export alter Clips nicht mehr möglich | Clips bleiben und werden nach Neuinstallation mit Stern übernommen; Hinweis in der Dokumentation | Paket |
| R8 | NVENC-Sitzungslimit oder belegter Encoder | Technik | Export langsamer | Ausweichen auf x264 mit Meldung | Kürzen und Export |
| R9 | RAM-Obergrenze (25 %) begrenzt die Pufferlänge | Bedienung | lange Puffer in hoher Auflösung nicht möglich | Prognose und Warnung in den Einstellungen | Ringpuffer |
| R10 | G1 verfehlt | Produkt | kein messbarer Vorteil gegenüber Game Bar | Umfang überdenken, bevor M4 beginnt | G1 |
| R11 | Nur ein Export gleichzeitig | Bedienung | Wartezeit bei mehreren Exporten | Export-Dialog zeigt „wartet“ für eingereihte Aufträge (`ListJobs`) | Kürzen und Export |
| R12 | Puffer voll während eines langen Speichervorgangs | Technik | Lücke im Puffer | Verwerfen bis zum nächsten Keyframe, neuer IDR, Lücke markiert | Ringpuffer |
| R13 | Dauerhafte Aufnahme von Bildschirm und Mikrofon | Datenschutz, Akku | Aufnahme sensibler Inhalte, Akkuverbrauch | Mikrofon-Hinweis beim ersten Start, Pause im Tray, automatische Pause bei gesperrtem PC (D27) | Datenschutz |

## Non-Functional Requirements

*   **Sicherheit:**
    *   Die Named Pipe ist nur für die SID des aktuellen Benutzers offen, mit `FILE_FLAG_FIRST_PIPE_INSTANCE` und Prüfung des Server-Prozesses.
    *   Befehle enthalten keine freien Pfade. Speicherordner prüft der Dienst (existiert, beschreibbar, kein Windows-, Programm- oder Systemordner).
    *   Der Worker bekommt nur Pfade vom Dienst.
    *   Gelöscht wird nur nach doppelter Eigentumsprüfung (Index und Tag).
    *   Kein `OpenProcess` auf fremde Prozesse (Spiel, Discord).
    *   Das MSIX-Paket ist signiert.
*   **Datenschutz:**
    *   Keine Netzwerkzugriffe. Das Manifest deklariert keine `internetClient`-Fähigkeit.
    *   Das Mikrofon ist standardmäßig an. Der erste Start weist darauf hin, das Tray zeigt den Mikrofonstatus, und eine Pause ist jederzeit möglich.
    *   Solange der PC gesperrt ist, nimmt GameClip nichts auf, und der Puffer wird geleert (D27).
    *   „Diagnose kopieren“ macht Benutzerpfade unkenntlich.
*   **Leistung:** Ziele G2 und G3. Gemessen wird mit PresentMon (Bildverluste im Spiel), `typeperf` (CPU und Private Bytes des Dienstes) und den Speicherzeiten aus dem eigenen Log.
*   **Einführung und Rückweg:**
    *   Verteilung über winget und GitHub Releases.
    *   MSIX erlaubt standardmäßig kein Downgrade. Ein Rückweg bedeutet Deinstallieren und ältere Version installieren; dabei werden Einstellungen, Index und Einzelspuren gelöscht, Clips bleiben und werden nach der Neuinstallation automatisch mit Stern übernommen.
    *   Trifft eine Version auf ein neueres Schema (Einstellungen oder Index), öffnet sie beides nur lesend und setzt das automatische Löschen aus.
    *   Die Datenbank wird vor jeder Migration gesichert.

--------------------------------------------------------------------------------

## Future Improvements

*   **Linux-Version** mit gpu-screen-recorder ([Anhang A des Briefs](../../concepts_brief.md#anhang-a-später--linux)).
*   **Smart-Cut** beim Kürzen (Q7).
*   **Nur den Ton des Spiels aufnehmen** (eigener Vanguard-Test nötig).
*   **HDR-Aufnahme** mit Tone-Mapping auf der GPU.
*   **AV1-Export** für bessere Qualität bei kleinen Zielgrößen.
*   **Öffentliche Kommandozeile** für Stream Deck und Skripte.
*   **Ringpuffer auf der SSD** für Puffer über 5 Minuten.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

Alle Pfade sind relativ zur Wurzel des Repositorys. Das Repository enthält noch keinen Code; alle folgenden Dateien werden neu angelegt, außer Datei 10 (dieses Dokument). Bezeichner im Code, im Protokoll, in der Datenbank und in Einstellungsschlüsseln sind englisch und ASCII; Texte in der Oberfläche sind deutsch.

**Maßgeblich für Feldnamen, Typen und Einheiten** ist die Nachrichtentabelle in Datei 24. Wo der Technische Plan abweicht, gilt Datei 24.

**Typ-Regeln für das Protokoll:** Zeiten sind `u64` Mikrosekunden (`*_us`), Größen `u64` Bytes (`*_bytes`), Fortschritt `percent: f32` von 0,0 bis 100,0, Clip-IDs `clip_id: u32` (Zeilennummer im Index, nicht die UUID aus dem Datei-Tag), Job-IDs `job_id: u32`. Alle `u64`-Felder tragen `#[ts(type = "number")]`, weil `JSON.parse` Zahlen liefert; Werte bis 2⁵³ sind dabei exakt. Zeitpunkte werden als RFC-3339-`String` übertragen.

### Feste Kennungen (Konstanten, keine Einstellungen)

| Name | Wert | Ort |
| --- | --- | --- |
| `APP_ID` | `io.github.poldion.GameClip` | `crates/gameclip-core/src/ids.rs` |
| `PIPE_NAME` | `\\.\pipe\io.github.poldion.GameClip-<SID>` (SID zur Laufzeit) | dto. |
| `MUTEX_NAME` | `Local\io.github.poldion.GameClip` | dto. |
| `STARTUP_TASK_ID` | `GameClipServiceStartup` | dto. und Manifest |
| `PROTOCOL_SCHEME` | `gameclip` | dto. und Manifest |
| `PROTOCOL_VERSION` | `1` (eine Zahl; Oberfläche und Dienst müssen gleich sein) | `crates/gameclip-protocol/src/version.rs` |
| `MAX_MESSAGE_BYTES` | 8 000 000 | dto. |
| `BLOCK_BYTES` | 2 097 152 (2 MiB) | `crates/gameclip-capture-win/src/ring.rs` |
| `RESERVE_SECONDS` | 30 | `crates/gameclip-core/src/forecast.rs` |
| `BASE_RAM_BYTES` | 80 000 000 (Grundbedarf in der Anzeige; gleich dem vorläufigen Ziel G3) | dto. |
| `RAM_WARN_FRACTION` / `RAM_REJECT_FRACTION` | 0,10 / 0,25 | dto. |
| `SAVE_WAIT_TIMEOUT_MS` | 500 | `crates/gameclipd/src/save.rs` |
| `RECOVERY_DELAYS_S` | `[1, 2, 4, 8, 8]` (5 Versuche, dann Fehler) | `crates/gameclip-core/src/state.rs` |
| `WORKER_SILENCE_TIMEOUT_S` | 30 | `crates/gameclipd/src/jobs.rs` |
| `DISCORD_POLL_S` | 2 | `crates/gameclip-capture-win/src/audio/discord.rs` |
| `BPP_MIN` / `BPP_WARN` | 0,05 / 0,03 *(vorläufig, S11)* | `crates/gameclip-core/src/export_plan.rs` |
| `AUDIO_TRACK_BPS` / `AUDIO_MIX_BPS` / `AUDIO_EXPORT_BPS` | 128 000 / 160 000 / 96 000 | `crates/gameclip-core/src/forecast.rs` |
| Mindest-Build | Windows 11 24H2 (10.0.26100.0) *(vorläufig, Q3)* | Manifest |

### Einstellungen (`settings.toml`, Schema-Version 1)

Gespeichert in `%APPDATA%\GameClip\settings.toml`. Typ `Settings` in `crates/gameclip-core/src/settings.rs`. „Pipeline“ = Änderung legt den Puffer neu an.

| TOML-Schlüssel | Rust-Typ | Einheit / Werte | Standard | Erlaubt | Pipeline |
| --- | --- | --- | --- | --- | --- |
| `schema_version` | `u32` | – | 1 | 1 | – |
| `first_run_done` | `bool` | – | `false` | – | nein |
| `capture.buffer_seconds` | `u32` | s | 60 | 15–300 | ja |
| `capture.resolution` | `enum Resolution` | `"720p"`, `"1080p"`, `"1440p"`, `"2160p"` | `"1080p"` | – | ja |
| `capture.fps` | `u32` | Bilder/s | 60 | 30, 60 | ja |
| `capture.quality` | `enum Quality` | `"low"`, `"medium"`, `"high"` | `"medium"` | – | ja |
| `capture.monitor` | `Option<String>` | Gerätename des Monitors (`DISPLAY_DEVICE.DeviceID` des Monitors), `None` = Hauptmonitor | `None` | – | ja |
| `capture.cursor` | `bool` | – | `true` | – | ja |
| `audio.mic_enabled` | `bool` | – | `true` | – | ja |
| `audio.mic_device` | `Option<String>` | WASAPI-Endpunkt-ID, `None` = Windows-Standardgerät | `None` | – | ja |
| `audio.discord_enabled` | `bool` | – | `false` | – | ja |
| `audio.gain_db.system` / `.mic` / `.discord` | `f32` | dB | 0,0 | −30,0 bis +12,0 | nein (wirkt sofort auf den Mixer) |
| `hotkey` | `Hotkey { code: String, #[serde(default)] ctrl: bool, #[serde(default)] alt: bool, #[serde(default)] shift: bool }` | `code` = `KeyboardEvent.code`, z. B. `"F8"` | `{ code = "F8" }` | F1–F11 und F13–F24 allein oder mit Strg/Alt/Shift; A–Z und 0–9 nur mit mindestens einem von Strg/Alt; keine Win-Taste | nein |
| `storage.clip_folder` | `Option<PathBuf>` | `None` = Standard zur Laufzeit | `None` | existiert, beschreibbar | nein |
| `storage.export_folder` | `Option<PathBuf>` | dto. | `None` | dto. | nein |
| `storage.clip_limit_bytes` / `.export_limit_bytes` | `u64` | Bytes | 10 000 000 000 | 1 000 000 000–2 000 000 000 000 | nein |
| `export.target_bytes` | `u64` | Bytes | 10 000 000 | 1 000 000–2 000 000 000 | nein |
| `sound.enabled` | `bool` | – | `true` | – | nein |
| `sound.volume` | `f32` | 0,0–1,0 | 0,7 | – | nein |

*   Unbekannte Schlüssel bleiben erhalten: `merge_into(existing_toml, &settings)` schreibt die bekannten Felder in die vorhandene Datei als `toml::Table` und lässt alle anderen Schlüssel stehen (Option-Felder, die auf `None` stehen, werden entfernt). Der IPC-Typ enthält deshalb keine unbekannten Schlüssel. (Umgesetzt 2026-11 statt des früher geplanten Feldes `extra`.)
*   `validate()` liefert `Vec<FieldError { key, reason }>`; `SetSettings` antwortet bei Fehlern mit `InvalidArgument` und dieser Liste im Feld `details`.
*   **Autostart** ist keine Einstellung in der Datei. Der Schalter in der Oberfläche steuert direkt den MSIX-StartupTask (`STARTUP_TASK_ID`).

**Bitraten der Qualitätsstufen** *(vorläufig, Q1)*, in Mbit/s, 60 fps; 30 fps = 60 % davon:

| Auflösung | low | medium | high |
| --- | --- | --- | --- |
| 720p | 4,8 | 8,4 | 14,4 |
| 1080p | 8 | 14 | 24 |
| 1440p | 12 | 20 | 32 |
| 2160p | 25 | 40 | 60 |

**Spuren-Regel:** Eine Einzelspur existiert, wenn ihre Quelle eingeschaltet ist (System immer, Mikrofon bei `mic_enabled`, Discord bei `discord_enabled`). Läuft die Quelle nicht oder ist sie pausiert, enthält die Spur Stille. Dieselbe Regel gilt für Prognose, Puffer, `.m4a` und `SeparateTracks`.

### Abhängigkeiten der Crates

```text
gameclip-core         → (keine internen; optionales Feature `ts` → ts-rs)
gameclip-protocol     → gameclip-core (mit Feature `ts`)
gameclip-gpu          → windows
gameclip-media        → gameclip-gpu, windows, rsmpeg
gameclip-capture-win  → gameclip-core, gameclip-gpu, gameclip-media
gameclipd             → gameclip-core, gameclip-protocol, gameclip-capture-win, gameclip-media, clipboard-win, uuid, time
gameclip-worker       → gameclip-core, gameclip-protocol, gameclip-gpu, gameclip-media
gameclip-ui-platform  → gameclip-core, gameclip-protocol
gameclip-ui (Tauri)   → gameclip-core, gameclip-protocol, gameclip-ui-platform
gameclip-testsupport  → (nur dev-dependency) Testquelle `synth`, gemeinsam für media, worker und gameclipd
```

### Reihenfolge der Umsetzung

| Meilenstein | Dateien | Inhalt |
| --- | --- | --- |
| **M0 – Prototypen und G1a** | 1–9, 12 (`test-protocols.md`), 13–17, 19–25, 26–35, 36 (nur Mikrofon), 37–45, 47, 49–53 (Grundgerüst), 55, 56 (Kürzen ohne `TargetSize`), 57 (ohne Größen-Tests), 68–71 | Speichern per Hotkey, Index ohne Watcher, Tray-Aktion „Letzten Clip kopieren“, Kommandozeile. Das M0-Paket enthält nur `gameclipd` und `gameclip-worker`; die Pipe wird in S1 über `gameclipd.exe --save-clip` geprüft. Prototypen S1, S2, S4 (ohne Discord), S6, S8, S9, dann G1a. |
| **M1 – Dienst** | 40–55 vollständig (inkl. 46, 48, 54), 11, 12 (`clip-file-format.md`) | Speicherlimit, Diagnose, alle Dienst-Tests. |
| **M2 – Oberfläche** | 36 (Discord), 58–64, 66, 67 (ohne Editor) | Erster Start, Einstellungen, Prognose, Galerie; Discord-Spur und S4-Discord-Kriterium. |
| **M3 – Galerie und Export** | 18, 56 (`size_target.rs`), 57 (Größen-Tests), 65 (Editor-Komponenten), 67 (Editor-Tests) | Editor, Größen-Export, Zwischenablage, Drag-and-drop; S5, S11; danach G1b. |
| **M4 – Freigabe** | 6, 8 (Signatur über SignPath) | Hardware-Testmatrix, S7. |
| **M5 – Öffentliche Beta** | 5, 10 | Vanguard-Test wiederholen, winget-Manifest. |

### Target Files to Create/Modify

Jeder Eintrag nennt die Änderungen; Art und Begründung stehen dort, wo sie nicht aus Pfad und Inhalt hervorgehen.

#### Repository-Wurzel und Werkzeuge

##### 1. `Cargo.toml` (Create)

*   **Art:** Cargo-Workspace.
*   **Änderungen:**
    *   `[workspace]` mit `resolver = "3"` und den Mitgliedern (alle Ordner unter `crates/` und `apps/gameclip-ui/src-tauri`); `[workspace.package]` mit `edition = "2024"`, `license = "GPL-3.0-or-later"`; Mitglieder nutzen `edition.workspace = true`.
    *   `[workspace.dependencies]` mit diesen Mindestversionen; die exakten Versionen hält das eingecheckte `Cargo.lock` fest (Prüfung Q16):

        | Crate | Version | Features | Genutzt von |
        | --- | --- | --- | --- |
        | `windows` / `windows-core` | 0.62 | je Crate die genutzten Module: `Win32_Foundation`, `Win32_Graphics_Direct3D`, `Win32_Graphics_Direct3D11`, `Win32_Graphics_Dxgi`, `Win32_Graphics_Dxgi_Common`, `Win32_Graphics_Imaging`, `Graphics_Capture`, `Graphics_DirectX_Direct3D11`, `Win32_System_WinRT_Direct3D11`, `Win32_System_WinRT_Graphics_Capture`, `Win32_Media_Audio`, `Win32_System_Pipes`, `Win32_Security`, `Win32_Security_Authorization`, `Win32_Storage_FileSystem`, `Win32_System_Console`, `Win32_System_LibraryLoader`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_UI_Shell`, `Win32_UI_Shell_Common`, `Win32_System_Com`, `Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`, `Win32_System_JobObjects`, `Win32_System_RemoteDesktop`, `ApplicationModel`, `UI_Notifications`, `Data_Xml_Dom` | gpu, media, capture-win, gameclipd, worker, ui-platform |
        | `rsmpeg` | 0.18 | `ffmpeg8` (Standard) | media |
        | `wasapi` | 0.26 | – | capture-win |
        | `rubato` / `audioadapter-buffers` | 5 / 5 | – | capture-win (`Slip` für den Driftausgleich) |
        | `rusqlite` | 0.41 | `bundled` | gameclipd |
        | `serde` | 1 | `derive` | alle |
        | `serde_json` | 1 | – | protocol, gameclipd, worker, ui |
        | `toml` | 0.8 | – | core |
        | `uuid` | 1 | `v4`, `serde` | core, protocol, gameclipd |
        | `time` | 0.3 | `local-offset`, `formatting`, `parsing`, `serde` | core, protocol, gameclipd |
        | `json-patch` | 4 | – | gameclipd (`merge` für `SetSettings`) |
        | `png` | 0.17 | – | gameclipd (Tray-Symbole in RGBA dekodieren) |
        | `thiserror` | 2 | – | alle Bibliotheken |
        | `bitflags` | 2 | `serde` | core |
        | `crossbeam-channel` | 0.5 | – | gameclipd, capture-win |
        | `tracing` / `tracing-subscriber` / `tracing-appender` | 0.1 / 0.3 / 0.2 | `env-filter` | gameclipd, worker, ui |
        | `ts-rs` | 11 | `uuid-impl`, `serde-compat`, `serde-json-impl` | protocol, core (optional, Feature `ts`) |
        | `tray-icon` | 0.21 | – | gameclipd |
        | `hound` | 3 | – | gameclipd (WAV der Töne) |
        | `clipboard-win` | 5 | – | ui-platform, gameclipd (M0-Tray-Aktion) |
        | `drag` | 2 | – | ui-platform |
        | `tauri` / `tauri-build` | 2 | – | ui |
        | `tempfile` | 3 | – | Tests |

    *   Zeittyp überall `time::OffsetDateTime`; im Index als UTC (RFC 3339), in Dateinamen als Ortszeit.
    *   `[profile.release]`: `lto = "thin"`, `panic = "abort"`, `codegen-units = 1`. `panic` gilt für alle Programme des Workspace (Cargo erlaubt es nicht je Paket); das ist gewollt, weil ein Absturz in jedem Programm sofort enden soll statt Threads halb weiterlaufen zu lassen.
*   **Begründung:** Eine Stelle für Versionen und Features.

##### 2. `rust-toolchain.toml` (Create)

*   **Änderungen:** `channel = "1.99.1"`, Ziele `x86_64-pc-windows-msvc` und `x86_64-unknown-linux-gnu`, Komponenten `clippy`, `rustfmt`, `llvm-tools-preview`.
*   **Begründung:** Reproduzierbare Builds; das Linux-Ziel für den Grenz-Job; `llvm-tools` für die Zeilenabdeckung.

##### 3. `.cargo/config.toml` (Create)

*   **Änderungen:** `[env]` mit `FFMPEG_INCLUDE_DIR = { value = "third_party/ffmpeg/include", relative = true }`, `FFMPEG_LIBS_DIR = { value = "third_party/ffmpeg/lib", relative = true }` und `FFMPEG_LINK_MODE = "dynamic"` (Variablen von `rusty_ffmpeg` 0.17, das `rsmpeg` nutzt; geprüft 2026-11: `FFMPEG_DLL_PATH` mit einer Datei linkt nur diese eine Bibliothek, deshalb das Verzeichnis der Import-Bibliotheken). bindgen braucht `libclang` (`LIBCLANG_PATH`); die mitgelieferten fertigen Bindings von `rusty_ffmpeg` sind unter Linux erzeugt (`c_long` = 64 Bit) und für Windows unbrauchbar. `hwcontext_d3d11va.h` fehlt in der Header-Liste von `rusty_ffmpeg`; `gameclip-media` spiegelt `AVD3D11VADeviceContext` und `AVD3D11VAFramesContext` selbst als `#[repr(C)]` und `TS_RS_EXPORT_DIR = { value = "apps/gameclip-ui/src/generated", relative = true }`. Die Typen tragen dann `#[ts(export)]` ohne eigenes `export_to`.
*   **Begründung:** Ein `build.rs` kann keine Umgebungsvariablen für das Build-Skript einer Abhängigkeit setzen; `[env]` kann das.

##### 4. `deny.toml` (Create)

*   **Änderungen:** `[licenses] allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "MPL-2.0", "Unicode-3.0", "BSL-1.0", "GPL-3.0-or-later", "CC0-1.0"]`; eigene Crates mit `license = "GPL-3.0-or-later"`. Nach dem ersten Build mit `cargo deny list` abgleichen. Keine `bans` für die Plattformgrenze (siehe Datei 7).
*   **Begründung:** Lizenzverträglichkeit mit GPL-3.0; `clipboard-win` steht unter BSL-1.0.

##### 5. `LICENSE`, `README.md` (Create)

*   **Änderungen:** `LICENSE` mit GPL-3.0-Volltext; jede Quelldatei trägt `SPDX-License-Identifier: GPL-3.0-or-later`. `README.md` mit Zweck, Voraussetzungen (Windows 11 ab Build 26100), Installation, Bauanleitung, „GameClip injiziert keinen Code; getestet mit Vanguard am <Datum>“, Hinweis auf das dauerhaft aufgenommene Mikrofon, Link zu den Lizenzen.
*   **Begründung:** D1; Einstieg für Nutzer und Mitwirkende.

##### 6. `about.toml`, `about.hbs`, `THIRD_PARTY_LICENSES.md` (Create)

*   **Änderungen:** Konfiguration und Vorlage für `cargo about generate about.hbs > THIRD_PARTY_LICENSES.md`; die Datei wird um FFmpeg, x264 (mit Link auf das gespiegelte Quellarchiv) und die npm-Lizenzen ergänzt (`npx license-checker --production --json`, eingefügt per `packaging/licenses.ps1`).
*   **Begründung:** Pflicht nach GPL und den Lizenzen der Abhängigkeiten.

##### 7. `.github/workflows/ci.yml` (Create)

*   **Änderungen:**
    *   **Werkzeuge installieren:** `cargo install cargo-llvm-cov cargo-deny cargo-about --locked` (Versionen im Workflow fest).
    *   **Job `windows`** in dieser Reihenfolge:
        1.  `pwsh packaging/ffmpeg/fetch-ffmpeg.ps1` (legt `third_party/ffmpeg/bin` auch in `PATH`, damit DLLs und `ffprobe` in Tests gefunden werden).
        2.  `npm ci && npm run build` in `apps/gameclip-ui` (erzeugt `dist`, das `tauri::generate_context!` beim Kompilieren braucht; möglich, weil die generierten Typen eingecheckt sind).
        3.  `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`.
        4.  `cargo test --workspace` (schreibt die ts-rs-Typen neu), danach `git status --porcelain apps/gameclip-ui/src/generated` muss leer sein.
        5.  `npm run typecheck && npm test` in `apps/gameclip-ui`.
        6.  `cargo build --workspace --release`; `cargo deny check licenses`.
        7.  Abdeckung: `cargo llvm-cov --workspace --fail-under-lines 85`; `cargo llvm-cov -p gameclip-core --fail-under-lines 90 --lcov --output-path core.lcov`; `pwsh packaging/check-coverage.ps1 core.lcov retention.rs export_plan.rs 100` prüft die beiden Dateien einzeln.
    *   **Grenz-Prüfung:** `cargo tree -p gameclip-core -p gameclip-protocol -e normal --target x86_64-pc-windows-msvc --prefix none --format "{p}"` wird ausgeführt; der Schritt scheitert, wenn der Befehl selbst scheitert oder eine Zeile auf `^windows(-sys)? ` passt. Dasselbe mit `-p gameclip-ui --depth 1` für die direkten Abhängigkeiten der Tauri-App.
    *   **Job `linux`:** `cargo test -p gameclip-core -p gameclip-protocol`, dann `npm ci && npm test && npm run typecheck` in `apps/gameclip-ui`.
*   **Begründung:** Tauri prüft `frontendDist` beim Kompilieren, deshalb das Frontend vor jedem Cargo-Schritt. Die generierten Typen werden eingecheckt; `git status --porcelain` findet auch neu hinzugekommene Dateien. `packaging/check-coverage.ps1` (neu, liest die lcov-Datei) erzwingt die Grenzen je Datei, weil `--fail-under-lines` nur die Summe prüft.

##### 8. `.github/workflows/release.yml` (Create)

*   **Änderungen:** Bei Tag `v*`: gleiche Schritte wie `windows`, dann `pwsh packaging/build-msix.ps1 -NoSign`, dann die SignPath-GitHub-Action (`signpath/github-action-submit-signing-request`, Projekt und Policy als Repository-Variablen), dann Upload von `.msix`, Prüfsummen und FFmpeg-Quellarchiv als Release.
*   **Begründung:** SignPath signiert als entfernter Dienst; Packen und Signieren sind getrennte Schritte.

##### 9. `.gitignore` (Create)

*   **Änderungen:** `target/`, `node_modules/`, `dist/`, `third_party/ffmpeg/`, `*.msix`, `*.pfx`, `packaging/out/`.
*   **Begründung:** FFmpeg wird geladen, nicht eingecheckt.

#### Dokumentation

##### 10. `docs/designs/gameclip.md` (Modify)

*   **Art / Änderungen:** Dieses Dokument; Status und Ergebnisse der Prototypen werden nachgetragen.

##### 11. `docs/protocol.md` (Create)

*   **Änderungen:** Rahmung, Hülle, die Nachrichtentabelle aus Datei 24, Fehlercodes, Versionsregel. Ein Test in Datei 25 parst jedes JSON-Beispiel aus dieser Datei gegen `gameclip-protocol`. Die interne Kommandozeile (Datei 41) wird nicht beschrieben.
*   **Begründung:** Vertrag für die spätere Linux-Version.

##### 12. `docs/test-protocols.md`, `docs/clip-file-format.md` (Create)

*   **Änderungen:**
    *   `test-protocols.md`: S1–S11 mit Aufbau, Schritten, Kriterien und Ergebnisblock (Datum, Build, Ergebnis); Vanguard-Protokoll; G1a/G1b-Messung; Referenzrechner.
    *   `clip-file-format.md`: MP4 im Clip-Ordner (H.264 High, Spur „Mischung“ mit `handler_name`, Tags `gameclip_id`, `gameclip_created` (RFC 3339 UTC), `gameclip_version`, geschrieben mit `movflags +use_metadata_tags`); `.m4a` mit Spuren „System“, „Mikrofon“, „Discord“ nach der Spuren-Regel und Tag `gameclip_id`; Exporte mit getrennten Spuren in der Reihenfolge Mischung (Standard), System, Mikrofon, Discord; Namensschema.
*   **Begründung:** Abbruchkriterien nachvollziehbar dokumentieren; fester Dateivertrag (D25).

#### `crates/gameclip-core` (kein Windows-Code)

##### 13. `crates/gameclip-core/Cargo.toml`, `crates/gameclip-core/src/lib.rs` (Create)

*   **Änderungen:** Abhängigkeiten `serde`, `toml`, `uuid`, `time`, `thiserror`, `bitflags`; optional `ts-rs` hinter dem Feature `ts`. Alle Typen, die über IPC gehen (`Settings`, `Hotkey`, `Resolution`, `Quality`, `Forecast`, `Level`, `ExportPlan`, `PlanError`, `ClipStatus`, `ServiceState`), tragen `#[cfg_attr(feature = "ts", derive(TS), ts(export))]`. `lib.rs` exportiert `ids`, `state`, `settings`, `forecast`, `export_plan`, `retention`, `naming`, `keymap`.

##### 14. `crates/gameclip-core/src/ids.rs` (Create)

*   **Änderungen:** Die Konstanten aus „Feste Kennungen“, die zu `gameclip-core` gehören.

##### 15. `crates/gameclip-core/src/state.rs` (Create)

*   **Art:** Zustandsmaschine.
*   **Änderungen:**
    *   `enum ServiceState { Starting, Buffering, Paused, Reconfiguring, Recovering { attempt: u8 }, Failed }`.
    *   `bitflags Restrictions { HOTKEY_INACTIVE, CPU_ENCODER, HDR_ACTIVE }`, getrennt davon `mic_paused: bool` und `saves_in_flight: u8`.
    *   `enum StateEvent { PipelineReady, PipelineFailed, HotkeyRegistered, HotkeyFailed, CpuEncoderOnly(bool), HdrChanged(bool), Hotkey, SaveFinished(Result<(), SaveError>), SettingsChanged { pipeline: bool }, DeviceLost, RecoverySucceeded, RecoveryFailed, StopBuffer, SessionLocked, SessionUnlocked, StartBuffer, Restart, MicPause(bool), UiDisconnectedWhileSuspended, Quit }`. (Nicht zu verwechseln mit dem IPC-Typ `ServiceEvent` in Datei 24.)
    *   `enum Effect { StartPipeline, StopPipeline, RebuildBuffer, BeginSave, QueueSave, PlaySound(Sound), Notify(Notice), ScheduleRecovery(Duration), ReRegisterHotkey, UpdateTray, Exit }`.
    *   `fn transition(&Machine, StateEvent) -> (Machine, Vec<Effect>)` setzt die Tabelle aus „Zustandsmaschine“ um. Für alle Zustände gelten zusätzlich: `HdrChanged`, `HotkeyRegistered`, `HotkeyFailed` und `CpuEncoderOnly` setzen bzw. löschen die passende Einschränkung und aktualisieren das Tray; `SaveFinished` verringert `saves_in_flight`, spielt den Ton und startet einen wartenden Vorgang, auch während „Konfiguriert neu“ und „Erholt sich“. Nur danach nicht aufgeführte Paare ergeben `(unverändert, [])` und werden protokolliert.
*   **Begründung:** Testbar ohne Hardware.

##### 16. `crates/gameclip-core/src/settings.rs` (Create)

*   **Änderungen:** Struktur `Settings` genau nach der Tabelle „Einstellungen“, mit `Default`, `validate()`, `fn pipeline_changed(&Settings, &Settings) -> bool` und `merge_into` (siehe „Einstellungen“). Lesen: Datei fehlt → Standard; `schema_version` > 1 → `ReadOnly`. Schreiben über temporäre Datei und Umbenennen.
*   **Begründung:** Eine Quelle für Schlüssel, Typen, Standardwerte und Prüfregeln.

##### 17. `crates/gameclip-core/src/forecast.rs` (Create)

*   **Änderungen:** `fn forecast(s: &Settings, physical_ram_bytes: u64) -> Forecast { buffer_bytes, reserve_bytes, total_bytes, level: Level }` mit `enum Level { Ok, Warning, Rejected }`.
    *   `n = 1 + mic_enabled + discord_enabled` (Einzelspuren), `rate = V(resolution, fps, quality) + n × AUDIO_TRACK_BPS + AUDIO_MIX_BPS` in bit/s.
    *   `buffer_bytes = ceil_blocks(rate × (buffer_seconds + 1) / 8 × 1,05)`, `reserve_bytes = ceil_blocks(rate × RESERVE_SECONDS / 8)`, `total_bytes = buffer_bytes + reserve_bytes + BASE_RAM_BYTES`.
    *   `level` vergleicht `total_bytes` mit `RAM_WARN_FRACTION` bzw. `RAM_REJECT_FRACTION × physical_ram_bytes`.
*   **Begründung:** Dienst und Oberfläche zeigen dieselbe Zahl.

##### 18. `crates/gameclip-core/src/export_plan.rs` (Create)

*   **Änderungen:** `fn plan(duration_us: u64, target_bytes: u64, source: (Resolution, fps)) -> Result<ExportPlan, PlanError>` mit `ExportPlan { width, height, fps, video_bps: u64, warn_low_quality: bool }` und `enum PlanError { TooSmall { min_target_bytes: u64 } }`.
    *   Gerechnet wird in `f64`: `video_bps = (target × 8 × 0,92 − AUDIO_EXPORT_BPS × d − 512 000) / d`.
    *   Liegt `video_bps` unter `MIN_VIDEO_BPS = 50 000`, ergibt sich `TooSmall` mit der kleinsten Zielgröße, die für diese Dauer reicht.
    *   Stufen: 1080p60 (1920×1080), 720p60 (1280×720), 720p30, 480p30 (854×480). Begonnen wird bei der Quellauflösung bzw. -bildrate, höchstens 1080p60; es wird nie hochskaliert. Gewählt wird die erste Stufe mit `bpp ≥ BPP_MIN`, sonst 480p30; `warn_low_quality = bpp < BPP_WARN`.
    *   `fn correct(video_bps, target_bytes, actual_bytes) -> u64` für den einen Korrekturversuch.
*   **Begründung:** Oberfläche (Vorab-Warnung über `GetExportEstimate`) und Worker rechnen gleich.

##### 19. `crates/gameclip-core/src/retention.rs` (Create)

*   **Änderungen:**
    *   `struct ClipInfo { row_id: u32, folder: FolderKind, size_bytes: u64, created_at: OffsetDateTime, starred: bool, status: ClipStatus, placeholder: bool, in_current_folder: bool }` mit `enum ClipStatus { Pending, Ok, Foreign }`.
    *   `fn plan_deletions(clips: &[ClipInfo], limit_bytes: u64, protect: Option<u32>) -> RetentionPlan { delete: Vec<u32>, warning: Option<Warning> }`.
    *   Gezählt werden nur `status == Ok && in_current_folder`; Platzhalter mit 0 Bytes. Löschkandidaten: ohne Stern, kein Platzhalter, nicht `protect`, älteste zuerst. Reicht das nicht: leere Liste und Warnung.
    *   `fn retention_groups(clip_limit, export_limit, same_folder: bool) -> Vec<(Vec<FolderKind>, u64)>`: bei getrennten Ordnern zwei Gruppen mit je ihrem Limit; bei gleichem Ordner **eine** Gruppe aus Clips und Exporten zusammen mit `min(clip_limit, export_limit)`. `retention_exec` ruft `plan_deletions` einmal je Gruppe auf.
*   **Begründung:** Der riskanteste Teil als reine Funktion; 100 % Zeilenabdeckung.

##### 20. `crates/gameclip-core/src/naming.rs` (Create)

*   **Änderungen:**
    *   `fn clip_file_name(program: Option<&str>, local_time: OffsetDateTime) -> String` → `<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4`. Der Programmteil: Endung `.exe` entfernt, Zeichen `< > : " / \ | ? *` und Steuerzeichen werden zu `_`, Punkte und Leerzeichen am Ende entfernt, höchstens 64 Zeichen, reservierte Namen (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`) bekommen `_` angehängt; leer oder `None` → `Clip`.
    *   `fn export_file_name(original_stem, profile) -> String`: `_trim`, `_tracks` oder `_<MB>MB` mit `MB = ceil(target_bytes / 1 000 000)`.
    *   `fn dedupe(name, exists: impl Fn(&str) -> bool) -> String` hängt `_2`, `_3` … vor der Endung an.

##### 21. `crates/gameclip-core/src/keymap.rs` (Create)

*   **Änderungen:** Tabelle `KeyboardEvent.code` → Windows-VK für F1–F24, A–Z, 0–9; `fn to_vk(&Hotkey) -> Result<(u32, u32), KeyError>` (Modifier, VK). `KeyError` bei F12, unbekannten Codes und bei A–Z/0–9 ohne Strg oder Alt (sonst würde die Taste systemweit abgefangen). Modifier: `MOD_CONTROL`, `MOD_ALT`, `MOD_SHIFT`, immer plus `MOD_NOREPEAT`. `Settings::validate()` nutzt dieselbe Funktion.

##### 22. `crates/gameclip-core/tests/{state,retention,forecast,export_plan,naming,keymap,settings}.rs` (Create)

*   **Änderungen:**
    *   `state.rs`: jede Tabellenzeile, plus ein Test, dass nicht aufgeführte Paare nichts ändern.
    *   `retention.rs`: geschützt, `foreign`, Platzhalter, Clip größer als Limit, neuer Clip wird nie gelöscht, Clips aus früherem Ordner. Gemeinsamer Ordner: 6 GB Clips + 6 GB Exporte, Limits 10 GB und 8 GB → eine Gruppe mit 8 GB, die ältesten 4 GB (Clips und Exporte gemischt nach Alter) werden gelöscht.
    *   `forecast.rs` erwartet u. a.: 1080p60 medium, 60 s, n = 2 → `buffer_bytes` = 117 440 512 (56 Blöcke), `reserve_bytes` = 54 525 952 (26 Blöcke), `total_bytes` = 251 966 464; 1440p60 high, 300 s, n = 3 → `buffer_bytes` = 1 287 651 328 (614 Blöcke), `reserve_bytes` = 123 731 968 (59 Blöcke), `total_bytes` = 1 491 383 296. Bei 16 GB RAM: erster Fall `Ok`, zweiter `Ok` (unter 10 %); bei 8 GB: zweiter Fall `Warning`.
    *   `export_plan.rs`: 10 MB bei 30 s → 720p30 (≈ 2,34 Mbit/s, bpp ≈ 0,085); bei 300 s → 480p30 mit Warnung (bpp ≈ 0,012); 1440p-Quelle startet bei 1080p60; 720p30-Quelle wird nicht hochskaliert; 1 MB bei 300 s → `TooSmall`.
    *   `keymap.rs`: F8 allein gültig, `KeyA` allein ungültig, `KeyA` mit Strg gültig, F12 ungültig.
    *   `naming.rs`, `settings.rs` (unbekannte Felder bleiben, Schema 2 → `ReadOnly`, Ungültiges → `FieldError`, `hotkey = { code = "F8" }` ohne Modifier-Felder wird gelesen).

#### `crates/gameclip-protocol` (kein Windows-Code)

##### 23. `crates/gameclip-protocol/Cargo.toml`, `src/lib.rs`, `src/framing.rs`, `src/version.rs`, `src/error.rs` (Create)

*   **Änderungen:**
    *   `framing.rs`: `read_frame(&mut impl Read) -> Result<Vec<u8>>` und `write_frame(&mut impl Write, &[u8])` mit u32-Länge Little Endian; Länge > `MAX_MESSAGE_BYTES` → Fehler, Verbindung wird getrennt.
    *   `error.rs`: `enum ErrorCode { NotReady, InvalidArgument, NotFound, Busy, HotkeyTaken, DiskFull, FolderMissing, BufferEmpty, EncoderFailed, Timeout, TooLarge, VersionMismatch, UnknownCommand, Internal }`.

##### 24. `crates/gameclip-protocol/src/messages.rs` (Create)

*   **Art:** IPC-Vertrag.
*   **Änderungen:**
    *   Hülle: `struct Request { id: u32, #[serde(flatten)] cmd: Command }` mit `#[serde(tag = "type", content = "body")] enum Command`; `struct Response { id: u32, ok: bool, body: Option<serde_json::Value>, error: Option<ErrorBody { code, message, details }> }`; `#[serde(tag = "event", content = "body")] enum ServiceEvent`. Request-IDs sind `u32` und zählen pro Verbindung hoch.
    *   Für jede Antwort gibt es einen benannten Typ (`GetStatusReply`, `ListClipsReply`, …, siehe Tabelle). Der Dienst serialisiert ihn in `body`; `ipc.ts` gibt ihn typisiert zurück.
    *   Unbekannter `type` → Antwort `UnknownCommand`. Unbekannte Felder werden ignoriert.
    *   `#[derive(TS)] #[ts(export)]` an allen Typen (Zielordner über `TS_RS_EXPORT_DIR`, Datei 3); eine Datei je Typ. Typnamen sind im ganzen Workspace eindeutig.
    *   **Handshake:** Erste Nachricht des Clients ist `Hello { protocol: u32, client: String }`. Der Dienst antwortet `{ protocol: u32, service_version: String }`. Bei ungleichem `protocol` antwortet er `VersionMismatch` und schließt die Verbindung.
    *   **Nachrichten** (Zeiten in µs Anzeigezeit, Größen in Bytes):

        | Befehl | Body | Antwort-Body (Typ `<Befehl>Reply`) |
        | --- | --- | --- |
        | `GetStatus` | – | `ServiceStatus { state: ServiceState, restrictions: [String], mic_paused, saves_in_flight, encoder, capture_resolution, buffered_us, forecast_total_bytes }` |
        | `GetSettings` | – | `{ settings: Settings, read_only: bool }` |
        | `SetSettings` | `{ partial: Value }` (JSON-Merge-Patch auf `Settings`) | `{ settings: Settings }` |
        | `GetForecast` | `{ settings: Settings }` | `Forecast` |
        | `ListDevices` | – | `{ monitors: [{ id, name, primary, width, height, hdr }], mics: [{ id, name, default }] }` |
        | `GetCapabilities` | – | `{ separate_tracks, max_buffer_s, forecast, encoders: [String], hdr_supported: false }` |
        | `StartBuffer` / `StopBuffer` / `Restart` / `SaveClip` | – | `{}` |
        | `PauseMic` | `{ on: bool }` | `{}` |
        | `SuspendHotkey` | `{ on: bool }` | `{}` |
        | `TestRegister` | `{ hotkey: Hotkey }` | `{ free: bool, error: Option<u32> }` (registriert kurz und gibt sofort frei; übernommen wird die Taste erst mit `SetSettings`) |
        | `SubscribeMicLevel` | `{ on: bool }` | `{}` |
        | `WatchFolders` | `{ on: bool }` | `{}` (Watcher nur bei offener Galerie) |
        | `ListClips` | `{ folder: "clips" \| "exports" }` | `{ clips: [ClipEntry { clip_id: u32, tag_id: String, path, size_bytes, created_at: String, duration_us, starred, has_tracks, status: ClipStatus, placeholder, in_current_folder }], used_bytes, limit_bytes }` |
        | `GetClip` | `{ clip_id }` | `ClipEntry` (`NotFound`, wenn unbekannt) |
        | `SetStar` | `{ clip_id, on }` | `{}` |
        | `AdoptClip` | `{ clip_id }` | `{}` (nur für `foreign`: Tag der Datei wird neu geschrieben, siehe Datei 45) |
        | `DeleteClip` | `{ clip_id }` | `{}` (endgültig, auch für `foreign`, mit Rückfrage in der Oberfläche) |
        | `GetFrameTimes` | `{ clip_id }` | `{ frames_us: [u64], duration_us }` |
        | `GetExportEstimate` | `{ clip_id, in_us, out_us, target_bytes }` | `{ plan: Option<ExportPlan>, too_small: Option<{ min_target_bytes }> }` |
        | `StartExportJob` | `{ clip_id, in_us, out_us, profile: Mix \| SeparateTracks \| TargetSize { bytes } }` | `{ job_id: u32 }` |
        | `CancelJob` | `{ job_id }` | `{}` |
        | `ListJobs` | – | `{ jobs: [{ job_id, clip_id, profile, state: "running" \| "queued", percent }] }` |
        | `CopyDiagnostics` | – | `{ text: String }` |

        | Ereignis | Body |
        | --- | --- |
        | `StateChanged` | `ServiceStatus` |
        | `SaveCompleted` | `{ clip_id, path, duration_us, size_bytes, has_tracks, shortened: bool }` |
        | `SaveFailed` | `{ code: ErrorCode, message }` |
        | `NewClip` / `ClipDeleted` | `{ clip_id, folder }` |
        | `ClipsChanged` | `{ folder }` (nach einem Abgleich) |
        | `FormatChanged` | `{ width, height, hdr }` |
        | `StorageWarning` | `{ folder, used_bytes, limit_bytes, reason: "all_starred" \| "clip_larger_than_limit" }` |
        | `JobProgress` | `{ job_id, percent: f32 (0,0–100,0), encoder: String }` |
        | `JobCompleted` | `{ job_id, clip_id }` |
        | `JobFailed` | `{ job_id, code: ErrorCode, message }` |
        | `MicLevel` | `{ dbfs: f32 }` (10 Hz, nur nach `SubscribeMicLevel`) |

    *   `src/worker.rs`: Auftrag und Meldungen des Workers (`WorkerJob { source, tracks_path: Option, target_part_path, output_tag_id: String, in_us, out_us, profile, encoder_order: [String], adapter_luid: u64 }`, `WorkerMessage { Progress { percent: f32 (0,0–100,0) }, Encoder { name }, Done { bytes, duration_us }, Error { code: WorkerError, message } }` mit `enum WorkerError { InputMissing, DecodeFailed, EncoderFailed, TooLarge, Io }`).
*   **Begründung:** Gemeinsamer Vertrag; Feldnamen sind ab v1 fest.

##### 25. `crates/gameclip-protocol/tests/roundtrip.rs` (Create)

*   **Änderungen:** Jede Nachricht hin und zurück; unbekannte Felder; `UnknownCommand`; Nachricht über 8 000 000 Bytes; jedes Beispiel aus `docs/protocol.md`.

#### `crates/gameclip-gpu` (Windows)

##### 26. `crates/gameclip-gpu/Cargo.toml`, `src/lib.rs`, `src/device.rs`, `src/convert.rs`, `src/hdr.rs` (Create)

*   **Änderungen:**
    *   `device.rs`: Adapter des gewählten Monitors (`IDXGIOutput` → `IDXGIAdapter1`, LUID); `ID3D11Device` mit `VIDEO_SUPPORT | BGRA_SUPPORT`, Multithread-Schutz an; WinRT-Gerät für WGC über `CreateDirect3D11DeviceFromDXGIDevice`; Geräteverlust über `GetDeviceRemovedReason`.
    *   **Gemessen auf AMD (2026-11):** Eine YCbCr-Hintergrundfarbe für den Rand liest der Treiber als RGB (Y = 105 statt 16). Der Rand wird deshalb als RGB-Schwarz gesetzt.
*   `convert.rs`: `struct Converter` mit `fn new(device, in_format: InFormat { Bgra, Nv12 }, in_size, out_size) -> Self` und `fn run(&self, src: TexRef, dst: TexRef)` mit `struct TexRef<'a> { tex: &'a ID3D11Texture2D, array_slice: u32 }`. `ID3D11VideoProcessor`; Eingabe BGRA (Aufnahme) oder NV12 (Worker, aus d3d11va), Ausgabe NV12, BT.709 begrenzter Bereich, Seitenverhältnis mit Rand. Eingabe-View mit `D3D11_VPIV_DIMENSION_TEXTURE2D` und `ArraySlice`, Ausgabe-View mit `D3D11_VPOV_DIMENSION_TEXTURE2DARRAY` und `FirstArraySlice`, weil FFmpegs Frame-Pools Texture-Arrays sind. Die Zieltextur muss `D3D11_BIND_RENDER_TARGET` tragen.
    *   `hdr.rs`: `IDXGIOutput6::GetDesc1`; HDR aktiv, wenn `ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020`.
    *   Kein Programm linkt Media Foundation zur Ladezeit (siehe Datei 58, `media.rs`). In S2 wird mit `dumpbin /dependents` geprüft, ob die FFmpeg-DLLs `mfplat.dll` importieren; falls ja, wird ein FFmpeg-Build ohne `--enable-mediafoundation` gewählt.
*   **Begründung:** Gemeinsam für Aufnahme und Worker, ohne dass der Worker am Aufnahme-Crate hängt.

#### `crates/gameclip-media` (FFmpeg, Windows)

##### 27. `crates/gameclip-media/Cargo.toml`, `src/lib.rs`, `src/hwdevice.rs`, `src/encoder.rs` (Create)

*   **Änderungen:**
    *   `hwdevice.rs`: `AVHWDeviceContext` (D3D11VA) aus dem vorhandenen `ID3D11Device`; `AVHWFramesContext` NV12 mit `BindFlags = D3D11_BIND_RENDER_TARGET`, `MiscFlags = 0`, `initial_pool_size = 0` (dynamischer Pool, eine Einzeltextur je Frame). **Gemessen auf AMD RX 6750 XT (2026-11):** NV12-Texture-Arrays lehnt der Treiber mit `BIND_RENDER_TARGET` ab (`E_INVALIDARG`), sie gehen nur mit `BIND_DECODER`. Ziele des `Converter` sind deshalb immer Einzeltexturen; Texture-Arrays kommen nur als *Eingabe* vor (Decoder-Frames im Worker, `array_slice`). QSV braucht für abgeleitete Kontexte einen festen Pool; ob QSV mit eigenen Einzeltexturen arbeitet, klärt S2 auf Intel-Hardware.
    *   Jeder Prozess (Dienst, Worker) besitzt seinen eigenen Geräte- und Frames-Kontext; der Worker legt seinen für die Ausgabegröße des Exports an.
    *   `encoder.rs`: `fn probe(gpu: &GpuDevice, cfg: &EncoderConfig { width, height, fps, bitrate_bps, profile }) -> Vec<EncoderCandidate { name, ok, reason }>` mit echter Testsitzung über 10 Bilder (umgesetzt 2026-11 mit `&GpuDevice` statt `adapter_luid`: der Dienst hat das Gerät schon; der Worker erzeugt es aus der LUID über `gameclip-gpu`). Zeitbasis aller Encoder 1/1 000 000 (µs). **Gemessen auf AMD RX 6750 XT (2026-11):** `h264_amf` mit D3D11-Frames aus dem dynamischen Pool läuft; ein erzwungenes IDR mitten in der GOP kommt als Keyframe-Paket an; keine B-Frames, Pakete in Anzeigereihenfolge. Reihenfolge: Hersteller-Encoder der Monitor-GPU, `h264_mf` nur mit Cargo-Feature `media-foundation` (nach S2), `libx264`.
    *   Gemeinsame Optionen: Profil `high`, `g = fps`, `bf = 0`, geschlossene GOP, `maxrate = bitrate`, `bufsize = bitrate`.
    *   **Profil `Capture`:** NVENC `preset p4`, `tune ll`, `rc cbr`, `forced-idr 1`; AMF `usage lowlatency`, `rc cbr`, `forced_idr 1`; QSV `preset medium`, Rate-Control CBR, `forced_idr 1` (Optionen gegen FFmpeg 8.1 in S2 prüfen; AMF auf RX 6750 XT 2026-11 mit `ffmpeg.exe` und D3D11-Frames geprüft: angenommen); x264 `preset superfast`, `tune zerolatency`.
    *   **Profil `Export`:** NVENC `preset p5`, `rc vbr`; AMF `usage transcoding`, `rc vbr_peak`; QSV `preset medium`; x264 `preset veryfast` ohne `tune`.
    *   Keyframe auf Anforderung: `AVFrame.pict_type = AV_PICTURE_TYPE_I` beim nächsten Bild; mit `forced-idr`/`forced_idr` wird daraus ein IDR. Jeder Schnappschuss und jede Lücke beginnt damit auf einem IDR, weil die regulären Keyframes ebenfalls IDR sind (geschlossene GOP).

##### 28. `crates/gameclip-media/src/audio_codec.rs` (Create)

*   **Änderungen:** AAC-LC (`aac`) 48 kHz Stereo, Eingabe interleaved Float in beliebigen Blöcken; intern ein Puffer je Kanal (umgesetzt 2026-11 statt `AVAudioFifo`, gleiche Wirkung ohne `unsafe`), Umwandlung in planares Float (FLTP) und Blöcke zu 1024 Samples; PTS in Samples. Der letzte Block darf kürzer sein. **Gemessen (2026-11):** `initial_padding` = 1024, erstes Paket bei PTS −1024, Summe der Paketdauern = Eingabe + 1024; die Edit-List im Muxer muss diese 1024 Samples ausblenden.

##### 29. `crates/gameclip-media/src/mux.rs`, `src/demux.rs`, `src/decode.rs` (Create)

*   **Änderungen:**
    *   `mux.rs`: `fn write_mp4(path, format: "mp4", video, audio_tracks: &[(title, packets, default)], tags, faststart: bool)` und `write_m4a(path, format: "ipod", …)`. Format wird ausdrücklich übergeben (die Endung `.part` verrät es nicht). `movflags` immer `+use_metadata_tags`, bei Exporten zusätzlich `+faststart`. Spurtitel als `handler_name`. Edit-List für t0 und AAC-Anlaufverzögerung. Nach `av_write_trailer` wird die Datei mit `std::fs::OpenOptions::new().write(true).open(path)?.sync_all()?` gesichert (`FlushFileBuffers` braucht ein Handle mit Schreibrecht). **Umgesetzt 2026-11:** Spuren als `Track { params, time_base, packets, title, default }`; `write_mp4(path, video, audio, tags, t0_us, faststart)` und `write_m4a(path, audio, tags, t0_us)`. Alle Spuren werden um `t0_us` verschoben; Pakete, die danach negativ sind (AAC-Anlauf, Ton kurz vor dem ersten Bild), schneidet der mov-Muxer über die Edit-List ab. Pakete werden nach Dekodierzeit sortiert und mit `av_packet_ref` geschrieben, die Originale im Ringpuffer bleiben unverändert. Gemessen mit ffprobe: Start von Video und Ton jeweils 0, Dauer auf 0,1 s genau, Titel, `default`-Kennzeichen und eigene Tags (`gameclip_tag_id`) kommen an. Der `ipod`-Muxer warnt wegen der Endung `.part`; das ist harmlos. x264 setzt bei Szenenwechseln zusätzliche Keyframes, die Keyframes liegen also nicht an festen Stellen: ein Schnappschuss nimmt den letzten vorhandenen Keyframe.
    *   `demux.rs`: `fn frame_times_us(path) -> Vec<u64>` (Anzeigezeit nach Edit-List) und `fn read_tags(path) -> Tags`.
    *   `decode.rs`: Hardware-Decoder `d3d11va` (liefert NV12-Slices im Texture-Array) mit Ausweichen auf Software; bei Software-Dekodierung lädt `av_hwframe_transfer_data` das Bild in ein D3D11-Frame des Worker-Pools hoch, damit `Converter` und Hardware-Encoder dieselbe Eingabe bekommen. Suche zum Keyframe vor einer Zeit. **Umgesetzt 2026-11:** `Decoder::open(path, Option<&HwDevice>)`, `seek_us`, `next_frame() -> Option<DecodedFrame { frame, pts_us }>`; `Uploader` statt einer freien Funktion, weil der H.264-Software-Dekoder `yuv420p` liefert (gemessen) und der NV12-Pool nur NV12 annimmt: der `Uploader` wandelt mit swscale (`SWS_POINT`, gleiche Größe) um und behält den Kontext. `demux::Tags` ist `BTreeMap<String, String>`. **Gemessen auf AMD RX 6750 XT (2026-11):** D3D11VA dekodiert alle 90 Bilder eines libx264-Clips; Array-Slices gehen als Eingabe in den `Converter`, die Bildnummern stimmen. Hochgeladene Software-Frames (Ziel mit `BIND_RENDER_TARGET`) taugen ebenfalls als Converter-Eingabe.

##### 30. `crates/gameclip-media/tests/{encode_mux,tags}.rs`, `crates/gameclip-testsupport/Cargo.toml`, `crates/gameclip-testsupport/src/synth.rs` (Create)

*   **Änderungen:**
    *   `gameclip-testsupport` ist ein eigenes Crate, das nur als `dev-dependency` von `gameclip-media`, `gameclip-worker` und `gameclipd` genutzt wird.
    *   `synth.rs`: Testquelle mit Parameter Dauer; jedes Bild trägt seine Nummer als 32-Bit-Muster aus 32 schwarz/weißen 16×16-Feldern oben links; dazu bewegtes Rauschen, damit der Encoder das Bild nicht trivial komprimiert; Ton als Sinus. **Umgesetzt 2026-11:** Felder 8 × 4; das Rauschen ist ein festes Muster, das je Bild um 3 Pixel wandert. Neues Zufallsrauschen in jedem Bild macht bei x264 jedes Bild zum Keyframe (Szenenwechsel) und entwertet Tests zum Springen und zu GOPs.
    *   `encode_mux.rs`: 10 s mit `libx264` und `aac`; Prüfung per `ffprobe -show_streams -show_frames`: Dauer, Spuranzahl, `handler_name`, Keyframe am Anfang, A/V-Versatz < 1 Bild.
    *   `tags.rs`: geschriebene Tags werden per `ffprobe -show_format` und per `demux::read_tags` wieder gelesen.

#### `crates/gameclip-capture-win` (Windows)

##### 31. `crates/gameclip-capture-win/Cargo.toml`, `src/lib.rs`, `src/audio/mod.rs` (Create)

*   **Änderungen:** Modul-Gerüst; exportiert `VideoPipeline`, `AudioPipeline`, `Ring`.

##### 32. `crates/gameclip-capture-win/src/wgc.rs` (Create)

*   **Änderungen:** `GraphicsCaptureItem` aus `HMONITOR` über `IGraphicsCaptureItemInterop`; `RequestAccessAsync(Borderless)`, dann `IsBorderRequired = false`; `Direct3D11CaptureFramePool::CreateFreeThreaded` mit 2 Puffern; `IsCursorCaptureEnabled`; `MinUpdateInterval = 1/fps` (ab Build 26100 immer verfügbar). `FrameArrived` kopiert in einen Ring aus 3 eigenen Texturen und vermerkt `SystemRelativeTime` (100-ns-Einheiten, in µs umgerechnet). **Umgesetzt 2026-11:** `acquire()` liefert das neueste Bild und sperrt seinen Platz, bis es freigegeben wird; der Handler schreibt nur in Plätze, die weder neuestes noch gehaltenes Bild sind. Ändert die Quelle die Größe, legt der Handler die eigenen Texturen neu an und ruft `Recreate`. **Gemessen auf RX 6750 XT (2026-11):** Die eigenen BGRA-Texturen brauchen `BIND_SHADER_RESOURCE | BIND_RENDER_TARGET`; mit `BIND_SHADER_RESOURCE` allein lehnt der Treiber die VideoProcessor-Eingabe ab (`E_INVALIDARG`). `RequestAccessAsync(Borderless)` und `IsBorderRequired = false` gelingen auf Build 26300 auch ohne MSIX-Paket (Teilantwort zu Q15; im Paket weiter zu prüfen).

##### 33. `crates/gameclip-capture-win/src/pacer.rs` (Create)

*   **Änderungen:** Thread mit `CreateWaitableTimerExW(CREATE_WAITABLE_TIMER_HIGH_RESOLUTION)` als Einmal-Timer. Vor jedem Takt wird die Wartezeit relativ neu berechnet: `(t0 + n/fps) − QPC_jetzt` (negativer `SetWaitableTimer`-Wert in 100 ns). So summiert sich keine Rundung auf. Je Takt das neueste Bild oder Wiederholung; Zähler für wiederholte und verworfene Bilder.

##### 34. `crates/gameclip-capture-win/src/video_pipeline.rs` (Create)

*   **Änderungen:** Verbindet `wgc`, `pacer`, `gameclip-gpu::Converter` und den Encoder (`Capture`-Profil). Zeitstempel intern in µs aus QPC, Stream-Zeitbasis beim Muxen 1/90 000. Methode `force_keyframe()`. Ereignisse an den Dienst: `DeviceLost`, `HdrChanged`, `FormatChanged`. x264-Weg: Kopie über Staging-Textur in den Hauptspeicher. **Umgesetzt 2026-11:** `VideoPipeline::start(VideoConfig, Sender<VideoEvent>)` richtet Gerät, Encoder-Prüfung und WGC auf dem Aufnahme-Thread ein und meldet `StartInfo` (Encoder, Adapter, Rahmen entfernt, HDR) zurück; `VideoEvent::Packet` trägt `pts_us`/`dts_us` in QPC-µs. PTS ist die **Taktzeit**, nicht die Aufnahmezeit, damit der Strom lückenlos im festen Raster liegt. Die Muxer-Zeitbasis wählt der mov-Muxer selbst (statt fest 1/90 000). **Gemessen (2026-11):** 2 s Aufnahme des Hauptmonitors mit `h264_amf`: 121 Takte, kein verpasster Takt, ein Paket je Takt, Abstände 16 666/16 667 µs; bei ruhigem Bildschirm liefert WGC weniger Bilder (Wiederholungen werden gezählt). **AMF mit `usage lowlatency` (und `ultralowlatency`) ignoriert `gop_size` und setzt nur den ersten Keyframe** (`transcoding`, `lowlatency_high_quality` und `webcam` halten ihn ein). Deshalb verlangt `VideoEncoder::send` selbst spätestens nach `fps` Bildern ein IDR; ein erzwungenes IDR startet die Sekunde neu. Das gilt für alle Encoder. AMF setzt außerdem keine Paketdauer; `mux` ergänzt fehlende Dauern aus dem Abstand der Zeitstempel, sonst fehlt das letzte Bild.

##### 35. `crates/gameclip-capture-win/src/audio/loopback.rs` (Create)

*   **Änderungen:** Über das Crate `wasapi` (einziger API-Weg; es kapselt `ActivateAudioInterfaceAsync` mit `AUDIOCLIENT_ACTIVATION_PARAMS`). Modus `Exclude(pid)` für System, `Include(pid)` für Discord. Format fest 48 kHz Float Stereo mit `AUDCLNT_STREAMFLAGS_LOOPBACK | EVENTCALLBACK | AUTOCONVERTPCM` (ob `GetMixFormat` hier fehlt, wird in S4 geprüft). Zeitstempel aus `pu64QPCPosition` (100 ns) in µs.

##### 36. `crates/gameclip-capture-win/src/audio/mic.rs`, `src/audio/discord.rs` (Create)

*   **Änderungen:**
    *   `mic.rs`: Shared-Mode-Aufnahme vom Gerät aus `audio.mic_device` bzw. dem Standardgerät; Neubinden bei `IMMNotificationClient`; bei Pause Stille; Pegel (RMS in dBFS) für `MicLevel`.
    *   `discord.rs`: alle `DISCORD_POLL_S` per `CreateToolhelp32Snapshot` nach `Discord.exe`, `DiscordPTB.exe`, `DiscordCanary.exe`; Wurzel = Elternprozess hat nicht denselben Namen; kein Handle. Bei Änderung beide Ströme neu binden. Erst ab M2 aktiv.

##### 37. `crates/gameclip-capture-win/src/audio/resample.rs`, `src/audio/mixer.rs` (Create)

*   **Änderungen:**
    *   `resample.rs`: `rubato` auf 48 kHz; Drift-Regler nach der Regel in „Audio-Pipeline“; Lücken mit Stille.
    *   `mixer.rs`: 10-ms-Blöcke, Gain je Quelle, Summe, Limiter (Look-ahead 5 ms, Schwelle −1 dBFS *(vorläufig)*), Stummschaltfenster für den eigenen Ton. Ausgabe an `audio_codec` für Mischung und Einzelspuren.
    *   **Umgesetzt 2026-11:**
        *   **Zeitachse:** Frames zu 48 kHz ab `AudioPipeline::start_us()` (QPC). WASAPI rechnet mit `autoconvert` selbst auf 48 kHz Float Stereo um, `rubato` dient nur dem Driftausgleich. `SourceTrack` legt jeden WASAPI-Block über seinen QPC-Zeitstempel ein. Abweichungen ab 20 ms zum Pufferende gelten als Lücke (mit Stille füllen) bzw. als Überlappung (abschneiden).
        *   **Drift:** Resampler ist `rubato::Slip` mit fester Ausgabe von 480 Frames. Der Regler hält den mittleren Füllstand (gleitendes Mittel über 10 s) auf dem Wert nach 2 s Anlauf und verstellt das Verhältnis um höchstens ±0,1 %. Simuliert: Ein um 0,05 % schnelleres Gerät ergibt das Verhältnis 0,99949, ohne Unterlauf.
        *   **Mixer:** eigener Thread im 10-ms-Takt (`Pacer::with_start(100, start_us)`), 100 ms hinter der Echtzeit. Der Limiter gibt Block k erst mit Blick auf Block k+1 aus. Die Einzelspuren laufen deshalb um denselben Block verzögert, und alle Spuren tragen dieselben Zeitstempel.
        *   **Pegel:** `AudioEvent::Levels` alle 100 ms (System, Mikrofon) für `MicLevel`.
        *   **Gemessen (2026-11):** Über 3 s kamen Mischung, System und Mikrofon mit je 138 Paketen an. Das erste Paket liegt genau 1024 Samples (AAC-Anlauf) vor dem Nullpunkt, die übrigen im festen Abstand von 21 333 µs. Prozess-Loopback mit Ausschluss des eigenen Prozesses liefert Systemton. `wasapi` liefert für Loopback gültige QPC-Zeitstempel; fehlt einer, gilt ersatzweise die Ankunftszeit minus Blockdauer.
        *   **Offen:** Die Lage eines Testtons auf der Zeitachse ist nur teilweise geprüft. Der Test `system_tone_lands_on_timeline` (ignoriert, von Hand) dekodiert die Spur „System“ und sucht einen 1-kHz-Ton per Goertzel. In einem sauberen Lauf lag er zusammenhängend 1 s lang auf der Zeitachse. Bei laufendem Spiel kam der Ton von `ffplay` aber nur in einem von drei Läufen im Loopback an; die Ursache klärt S4. Ebenfalls offen sind das Neubinden des Mikrofons bei Gerätewechsel (`IMMNotificationClient`) und `discord.rs` (M2).
        *   `mic::list_devices()` initialisiert COM selbst, denn ohne MTA schlägt die Geräteliste fehl (`CO_E_NOTINITIALIZED`).

##### 38. `crates/gameclip-capture-win/src/ring.rs` (Create)

*   **Art:** Ringpuffer.
*   **Änderungen:**
    *   Block-Pool aus `Arc<Block>` zu `BLOCK_BYTES`, Größe aus `Forecast.buffer_bytes + reserve_bytes`. Ein Paket, das nicht mehr in den Rest des aktuellen Blocks passt, beginnt im nächsten Block. Pakete größer als ein Block bekommen eine eigene Zuteilung außerhalb des Pools (wird in der Diagnose gezählt).
    *   `PacketRef { track, pts_us, dts_us, keyframe, gap_before: bool, block, offset, len, section }`.
    *   `push`, `evict` (ganze Video-Bildgruppen, Audio vor dem ältesten Keyframe), `snapshot(from_us, to_us) -> Snapshot` (nur neuester Abschnitt, schneidet an Lücken wie an Abschnittsgrenzen).
    *   `pub trait PacketSource { fn wait_for(&self, pts_us: u64, timeout: Duration) -> bool; fn snapshot(&self, from_us: u64, to_us: u64) -> Snapshot; }` wird hier definiert und von `Ring` umgesetzt. `gameclipd::save` nutzt nur diesen Trait, damit Tests eine Attrappe einsetzen können.
    *   **Pool erschöpft:** Videopakete bis zum nächsten Keyframe verwerfen, Audio im selben Zeitraum verwerfen, `video_pipeline.force_keyframe()` aufrufen, beim ersten Paket danach `gap_before = true`.
    *   **Umgesetzt 2026-11:**
        *   **Schnittstelle:** `Ring::new(RingConfig { pool_blocks, duration_us, block_bytes })`, dazu `RingConfig::from_forecast(&Forecast, buffer_seconds)`. `push(Stream, pts_us, dts_us, keyframe, &[u8]) -> PushOutcome { Stored, Dropped, NeedKeyframe }`; bei `NeedKeyframe` ruft der Aufrufer `force_keyframe()`. Außerdem `new_section()` und `PacketSource { wait_for, snapshot }`.
        *   **Speicher:** Blöcke sind `Arc<BlockBuf>` mit Rohzeiger, der Index und die Schnappschüsse halten Klone. Ein Block ist frei, wenn `Arc::strong_count == 1`; das wird unter dem Mutex geprüft, unter dem auch alle Klone entstehen. Beim Anlegen wird jede Seite einmal beschrieben, damit der Speicher wirklich belegt ist.
        *   **Verwerfen nach Platz:** geschieht schon vor „Pool erschöpft“. Bevor eine Lücke entsteht, werden alte Bildgruppen verworfen, deren Blöcke kein Schnappschuss hält.
        *   **Grenzfall (beim Testen gefunden):** Passt die laufende Bildgruppe nicht mehr neben den gehaltenen Schnappschuss, gibt es nur noch diese eine Bildgruppe im Index, und `evict_gop` kann nichts verwerfen. Deshalb darf ein **neuer Keyframe auch die letzte alte Bildgruppe verdrängen**, sonst käme die Aufnahme nach der Lücke nie wieder in Gang.
        *   **Schnappschuss:** beginnt am letzten Keyframe ≤ `from_us` des neuesten Abschnitts. Eine Lücke dahinter schneidet ihn, dann ist `truncated = true`. Ton wird ab dem Paket genommen, das t0 überdeckt.
        *   **Gemessen:** alle sieben Fälle aus Datei 39 grün, darunter 30 s Weiterschreiben bei gehaltenem Schnappschuss (Daten unverändert) und die Lücke bei vollem Pool (genau eine IDR-Anforderung, weiter am nächsten Keyframe, Ton aus der Lücke verworfen).

##### 39. `crates/gameclip-capture-win/tests/ring.rs` (Create)

*   **Änderungen:** Mit synthetischen Paketen: Verwerfen nur ganzer GOPs; Audio passend zum ersten Keyframe; Schnappschuss während des Verwerfens; Pool voll bei gehaltenem Schnappschuss (Lücke, IDR-Anforderung, Audio verworfen); Schnappschuss über Lücke wird getrennt; Pakete über Blockgrenzen und größer als ein Block.

#### `crates/gameclipd` (Windows)

##### 40. `crates/gameclipd/Cargo.toml`, `src/lib.rs`, `src/main.rs` (Create)

*   **Änderungen:**
    *   `lib.rs` enthält alle Module (Dateien 41–54) als `pub mod`; `main.rs` ruft nur `gameclipd::run()`. So können Integrationstests (Datei 55) die Module importieren.
    *   `Cargo.toml`: zusätzlich `[[bin]] name = "fake-worker"`, `path = "tests/support/fake_worker.rs"`, `required-features = ["test-support"]`; Tests nutzen `env!("CARGO_BIN_EXE_fake-worker")`.
    *   `windows_subsystem = "windows"`.
    *   Reihenfolge: **zuerst** Argumente prüfen (`--save-clip` → `cli.rs`, ohne Mutex); dann Mutex `MUTEX_NAME` – existiert er, Ende mit Exit-Code 0 ohne Meldung; dann Logging, Einstellungen, Index, Encoder-Prüfung, Pipeline, Nachrichten-Thread (Hotkey und Tray), IPC-Server.
    *   Hauptschleife: `crossbeam-channel` mit Ereignissen an `gameclip-core::state::transition`; Wirkungen werden ausgeführt, nicht aufgeführte Paare protokolliert.

##### 41. `crates/gameclipd/src/cli.rs` (Create)

*   **Änderungen:** `--save-clip`: mit der Pipe verbinden, Handshake, `SaveClip`, höchstens 10 s auf `SaveCompleted`/`SaveFailed` warten. Exit 0 bei Erfolg, 1 bei Fehler oder ohne Dienst; Meldung auf stderr nach `AttachConsole(ATTACH_PARENT_PROCESS)`. Weil das Programm ein GUI-Subsystem hat, ist der Exit-Code nur mit `start /wait` bzw. `Start-Process -Wait` sichtbar. Kein App-Execution-Alias; die Kommandozeile braucht keine Paket-Identität, weil sie nur die Pipe nutzt. Nicht öffentlich dokumentiert. **Gemessen (2026-11, ohne MSIX):** Dienst gestartet, nach 6 s `--save-clip` aufgerufen: Exit 0 nach 183 ms, MP4 mit 5,4 s in 1080p60 mit „Mischung“, `.m4a` mit „System“ und „Mikrofon“. Der Clip entstand bei laufendem Valorant (Programmname `VALORANT-Win64-Shipping`); das ist kein Vanguard-Test im Sinne von S1.

##### 42. `crates/gameclipd/src/msgthread.rs` (Create)

*   **Änderungen:** Ein Thread mit Message-only-Fenster und Win32-Nachrichtenschleife. Er registriert den Hotkey (`RegisterHotKey` mit `keymap::to_vk`), empfängt `WM_HOTKEY` (QPC-Zeitstempel an die Hauptschleife) und hostet das Tray (`tray-icon`, Ereignisse an die Hauptschleife). Derselbe Thread meldet sich mit `WTSRegisterSessionNotification(NOTIFY_FOR_THIS_SESSION)` an und gibt `WM_WTSSESSION_CHANGE` mit `WTS_SESSION_LOCK`/`WTS_SESSION_UNLOCK` als `SessionLocked`/`SessionUnlocked` weiter. Befehle `Register`, `Suspend`, `Resume`, `TestRegister` kommen über `PostThreadMessage`, die Antwort über einen `crossbeam`-Einmal-Kanal. **Umgesetzt 2026-11:** Die Befehle liegen in einem Kanal, `PostThreadMessageW(WM_APP)` weckt die Schleife. Das Tray (`tray-icon`) lebt auf diesem Thread, seine Menüereignisse werden nach jeder Nachricht abgeholt. **Befund:** Aus der Entwicklungsumgebung per `SendInput`/`keybd_event` eingespeiste Tasten lösen kein `WM_HOTKEY` aus, auch nicht in einem minimalen C#-Gegentest auf demselben Desktop (`WinSta0\Default`). Ob F8 auslöst, prüft deshalb nur ein echter Tastendruck (S8); der Speicherweg wurde über `--save-clip` geprüft.

##### 43. `crates/gameclipd/src/save.rs` (Create)

*   **Änderungen:** Ein Speicher-Thread; Speichervorgänge nacheinander, höchstens einer wartet. Umgesetzt wird der Ablauf aus „Speichern per Hotkey“; die Reihenfolge hier ist maßgeblich:
    1.  Zeitstempel übernehmen.
    2.  Auf das Videopaket warten (`SAVE_WAIT_TIMEOUT_MS`, sonst `Timeout`).
    3.  `PacketSource::snapshot` (t0 = erster Keyframe).
    4.  Programmname (`foreground.rs`), Dateiname mit `naming::clip_file_name` und `naming::dedupe`.
    5.  Index-Eintrag `pending`.
    6.  MP4 und `.m4a` über `gameclip-media::mux` schreiben und umbenennen (`MoveFileExW` mit `MOVEFILE_WRITE_THROUGH`).
    7.  Teilfehler: `.m4a` fehlgeschlagen → ihre `.part` bzw. Datei löschen, MP4 behalten, `has_tracks = false`, Hinweis in der Benachrichtigung; MP4 fehlgeschlagen → MP4-`.part`, die `.m4a` (als `.part` oder bereits umbenannt) und den Eintrag löschen, `SaveFailed`.
    8.  Eintrag `ok`, Speicherlimit (`retention_exec.rs`).
    9.  Ton, Benachrichtigung, `SaveCompleted`.
    *   **Umgesetzt 2026-11 (Schritte 1–8 ohne Speicherlimit):**
        *   `save_clip(&SaveInputs, &SaveRequest) -> Result<Saved, SaveError>`.
        *   **Ohne Kopie:** Die Pakete des Schnappschusses gehen ohne Kopie an FFmpeg (`PacketRef::to_avpacket`, `av_buffer_create` mit einer `Arc` auf den Block). Ein 60-s-Clip braucht beim Speichern also keine ~100 MB zusätzlich.
        *   **Zeitstempel:** Video in µs relativ zu t0. Ton wird relativ zu t0 auf das Sample gerundet (Zeitbasis 1/48 000). Das ist exakt, weil die Ringzeiten aus Samples um weniger als 1 µs abgerundet sind.
        *   **Gemessen mit ffprobe:** Mischung (MP4) und Einzelspuren (`.m4a`) haben denselben Start 0 und dieselbe Länge in Samples. Für den Speicherweg ist das Kriterium aus S4 („±1 Sample“) damit erfüllt.
        *   **Cliplänge:** Ein Clip ist zwischen Pufferlänge und Pufferlänge plus einer Bildgruppe lang. Er beginnt am letzten Keyframe ≤ Tastendruck − Pufferlänge.
        *   **Fehlerzuordnung:**
            *   `ERROR_DISK_FULL`/`ERROR_HANDLE_DISK_FULL`/`ENOSPC` → `DiskFull`
            *   „nicht gefunden“/„Zugriff verweigert“ → `FolderMissing`
            *   alles andere → `EncoderFailed`
        *   **Offen:** Speicherlimit, Ton, Benachrichtigung, Test „Platte voll“.
    *   **Loopback-Befund (2026-11):** `EXCLUDE_TARGET_PROCESS_TREE` schließt auch alle **Kindprozesse** von gameclipd aus (etwa den Worker). Der Ton eines Programms, das gameclipd startet, landet also nie im Clip.

##### 44. `crates/gameclipd/src/foreground.rs` (Create)

*   **Änderungen:** `GetForegroundWindow` → `GetWindowThreadProcessId` → Eintrag mit dieser PID in `CreateToolhelp32Snapshot` → `szExeFile`. Kein Handle auf den Prozess. Ohne Treffer `None`.

##### 45. `crates/gameclipd/src/index.rs`, `crates/gameclipd/sql/schema.sql` (Create)

*   **Änderungen:**
    *   `schema.sql` (Schema 1):

        ```sql
        CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO meta VALUES ('schema_version', '1');
        CREATE TABLE clips (
          row_id        INTEGER PRIMARY KEY,        -- = clip_id im Protokoll
          tag_id        TEXT NOT NULL,              -- UUID aus dem Datei-Tag gameclip_id; nicht eindeutig (Kopien)
          folder        TEXT NOT NULL CHECK (folder IN ('clips','exports')),
          path          TEXT NOT NULL UNIQUE,
          size_bytes    INTEGER NOT NULL,
          created_at    TEXT NOT NULL,              -- RFC 3339, UTC
          duration_us   INTEGER NOT NULL,
          starred       INTEGER NOT NULL DEFAULT 0,
          has_tracks    INTEGER NOT NULL DEFAULT 0,
          source_row_id INTEGER,                    -- bei Exporten: Original-Clip
          placeholder   INTEGER NOT NULL DEFAULT 0,
          missing_count INTEGER NOT NULL DEFAULT 0,
          status        TEXT NOT NULL CHECK (status IN ('pending','ok','foreign'))
        );
        CREATE INDEX clips_folder_created ON clips(folder, created_at);
        CREATE INDEX clips_tag ON clips(tag_id);
        ```

    *   **Eigentum:** Eine Zeile besitzt ihre Datei, wenn `status = 'ok'` und der Tag der Datei gleich `tag_id` ist. Pro `tag_id` hat höchstens eine Zeile `status = 'ok'`; weitere Zeilen mit derselben `tag_id` sind `foreign`.
    *   Die Einzelspuren liegen unter `tracks\<tag_id>.m4a` und gehören zur `ok`-Zeile.
    *   `in_current_folder` ist keine Spalte, sondern wird bei jeder Abfrage berechnet: Elternordner von `path` = aktuell eingestellter Ordner der Zeile.
    *   `index.rs`: `rusqlite` im WAL-Modus, ein Schreib-Thread; `insert_pending`, `mark_ok`, `remove`, `set_star`, `list(folder)`, `get(row_id)`, `adopt(row_id)`, `reconcile(folder_path, folder)`. Ist `schema_version` > 1: nur lesend, Löschen ausgesetzt. Lässt sich die Datei nicht öffnen: umbenennen nach `index.sqlite.broken`, neu anlegen, `reconcile`.
    *   `reconcile(folder_path, folder)`:
        1.  **Bestehende Zeilen** (alle, auch aus früheren Ordnern): für jede Zeile außer `pending` wird `Path::exists(path)` geprüft. Fehlt die Datei, steigt `missing_count`; bei 2 werden Zeile und (bei `ok`) ihre `.m4a` entfernt. Ist sie da, wird `missing_count = 0` gesetzt. Clips in früheren Ordnern bleiben so erhalten, solange ihre Datei existiert.
        2.  **Neue Dateien** nur in `folder_path` (nicht rekursiv), `.part` übersprungen. Vor jedem Lesen wird über `FileProbe::is_placeholder(path)` das Platzhalter-Attribut geprüft; Platzhalter werden ohne Tag-Lesen mit `placeholder = 1` geführt. Sonst Tag lesen (`demux::read_tags`):
            *   Pfad ist schon bekannt → Größe aktualisieren.
            *   `tag_id` bekannt, aber die `ok`-Zeile hat einen anderen Pfad, der nicht mehr existiert → Umbenennen: Pfad der `ok`-Zeile aktualisieren.
            *   `tag_id` bekannt und die `ok`-Zeile existiert weiter → Kopie: neue Zeile `foreign`.
            *   `tag_id` unbekannt → neue Zeile `ok`, `starred = 1`.
        3.  Danach `ClipsChanged`.
    *   `adopt(row_id)` (nur `foreign`): Der Dienst schreibt die Datei mit neuer `gameclip_id` um (Umpacken ohne Neukodieren über `gameclip-media::mux`, erst `.part`, dann ersetzen), setzt `tag_id` auf die neue UUID und `status = 'ok'`, `starred = 1`, `has_tracks = 0`.
    *   `trait FileProbe { fn is_placeholder(&self, path: &Path) -> bool; }` mit der Windows-Umsetzung über `GetFileAttributesW` und `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS`; Tests setzen eine Attrappe ein, weil sich das Attribut nicht per `SetFileAttributesW` setzen lässt.

##### 46. `crates/gameclipd/src/watcher.rs` (Create)

*   **Änderungen:** Läuft nur zwischen `WatchFolders{on: true}` und `{on: false}` bzw. dem Trennen der Oberfläche. `ReadDirectoryChangesW` auf beide Ordner (nicht rekursiv); jede Meldung startet einen 1-s-Entprell-Timer, danach `index.reconcile`. `ERROR_NOTIFY_ENUM_DIR` (Überlauf) löst ebenfalls einen Abgleich aus. Keine eigene Logik über Dateien.

##### 47. `crates/gameclipd/src/retention_exec.rs` (Create)

*   **Änderungen:** Läuft nach jedem neuen Clip oder Export, nach Verringern eines Limits und nach einem Ordnerwechsel. Ruft `retention::plan_deletions` einmal je Gruppe aus `retention_groups`. Vor jedem Löschen: `FileProbe::is_placeholder` (→ überspringen), Tag lesen und mit `tag_id` vergleichen; bei Abweichung nicht löschen, Eintrag `foreign`, Warnung. Sonst `DeleteFileW` für MP4 und `.m4a`, `ClipDeleted`, Logeintrag (Pfad, Größe, Grund). Warnungen als `StorageWarning`.

##### 48. `crates/gameclipd/src/frames.rs` (Create)

*   **Änderungen:** `GetFrameTimes` über `gameclip-media::demux::frame_times_us`; Ergebnis je `clip_id` im Speicher zwischengespeichert (die letzten 4 Clips).

##### 49. `crates/gameclipd/src/jobs.rs` (Create)

*   **Änderungen:** Warteschlange, höchstens ein laufender Auftrag. `Jobs::new(worker_path: PathBuf, …)`; im Betrieb ist `worker_path` die `gameclip-worker.exe` neben `gameclipd.exe`, in Tests `fake-worker`. Vergibt `job_id` und eine neue UUID als `output_tag_id`, bestimmt `target_part_path` (`naming::export_file_name` + `dedupe` + `.part`), legt den Index-Eintrag `pending` an. Ergibt `export_plan::plan` `TooSmall`, endet der Auftrag sofort mit `JobFailed{code: TooLarge}`, ohne Worker. Startet den Worker mit `BELOW_NORMAL_PRIORITY_CLASS` in einem Job-Objekt mit `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; Auftrag als JSON über stdin; stdout-Zeilen → `JobProgress`/`JobCompleted`/`JobFailed`. Zeitlimit `WORKER_SILENCE_TIMEOUT_S`. Erfolg: umbenennen, `ok`, Speicherlimit Export-Ordner. Fehler oder `CancelJob` (Job-Objekt beenden): `.part` und `pending`-Eintrag löschen.

##### 50. `crates/gameclipd/src/sound.rs`, `assets/sounds/success.wav`, `assets/sounds/failure.wav` (Create)

*   **Änderungen:** Wiedergabe über WASAPI mit `sound.volume`; Beginn und Dauer an den Mixer, wenn die Stummschaltung des eigenen Tons aktiv ist. Die WAV-Dateien sind selbst erstellt und stehen unter CC0-1.0.

##### 51. `crates/gameclipd/src/tray.rs` (Create)

*   **Änderungen:** Sechs Symbole und Vorrang wie in „Tray, Töne und Benachrichtigungen“; Tooltip mit allen zutreffenden Punkten. Menü: Clip speichern, Aufnahme pausieren/fortsetzen, Mikrofon pausieren, Galerie öffnen, Einstellungen, Neu starten (nur bei Fehler), Beenden. In M0 zusätzlich „Letzten Clip kopieren“ (CF_HDROP) für G1a. „Galerie öffnen“ startet die Oberfläche über die AUMID (`IApplicationActivationManager`).

##### 52. `crates/gameclipd/src/toast.rs` (Create)

*   **Änderungen:** `ToastNotificationManager::CreateToastNotifierWithId(AUMID)`; Erfolg „Clip gespeichert (Dauer, Größe)“ mit Knopf „Im Ordner anzeigen“ (`activationType="protocol"`, `arguments="gameclip:reveal?id=<clip_id>"`); Fehler mit Grund. Ob Toasts mit `AppListEntry="none"` für den Dienst erscheinen, wird in S1 geprüft; sonst verwendet `toast.rs` die AUMID der Oberfläche.

##### 53. `crates/gameclipd/src/ipc_server.rs` (Create)

*   **Änderungen:** `CreateNamedPipeW` mit SDDL `D:P(A;;GA;;;<SID>)` (nur der aktuelle Benutzer), `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE` für die erste Instanz, höchstens 4 Verbindungen; ein fünfter Client bekommt `ERROR_PIPE_BUSY` und versucht es erneut. Je Verbindung ein Thread: Handshake, Anfragen, Ereignisse. `SuspendHotkey`, `WatchFolders` und `SubscribeMicLevel` werden je Verbindung gezählt; die Wirkung gilt, solange mindestens eine Verbindung sie anfordert. Beim Trennen werden die Anforderungen dieser Verbindung zurückgenommen. Ordnerprüfung in `SetSettings`: existiert, beschreibbar, nicht `%WINDIR%` oder `%ProgramFiles%` bzw. darunter, nicht der Laufwerksstamm selbst (z. B. `D:\`).
    *   **Umgesetzt 2026-11 (M0-Umfang):** Handshake, `GetStatus`, `SaveClip`, Ereignisse `SaveCompleted`/`SaveFailed` (auch sofort bei abgelehntem Tastendruck mit `NotReady`); übrige Befehle `UnknownCommand`. Ein synchrones Pipe-Handle serialisiert Lesen und Schreiben, ein blockierendes `ReadFile` hielte also Ereignisse auf. Der Verbindungs-Thread prüft deshalb mit `PeekNamedPipe` (Takt 10 ms) und sendet dazwischen Ereignisse. Overlapped I/O ist die spätere Verbesserung.

##### 54. `crates/gameclipd/src/diagnostics.rs`, `src/logging.rs` (Create)

*   **Änderungen:** `logging.rs`: `tracing-appender` täglich rotierend, `max_log_files = 7`. `diagnostics.rs`: Zähler und Ereignisse aus „Diagnose“; `CopyDiagnostics` mit Windows-Build, GPU und Treiberversion, Encoder-Prüfung, Audiogeräte, letzte 200 Logzeilen; `%USERPROFILE%` → `<USER>`.

##### 55. `crates/gameclipd/tests/{retention_exec,index,save,jobs,ipc}.rs` (Create)

*   **Änderungen:** Mit `tempfile`-Ordnern, einer `PacketSource`-Attrappe statt der echten Pipeline (Datei 38), einer `FileProbe`-Attrappe für Platzhalter (Datei 45) und Clips aus `gameclip-testsupport::synth`; Feature `test-support`:
    *   `retention_exec`: löscht nur eigene Dateien, überspringt Platzhalter, Tag-Abweichung → `foreign`, gemeinsamer Ordner mit einer Gruppe.
    *   `index`: `reconcile` mit umbenannten, kopierten (zwei Zeilen mit gleicher `tag_id`, eine `ok`, eine `foreign`), fehlenden und unbekannten Dateien; Clips aus einem früheren Ordner bleiben; `pending` bleibt; `adopt` schreibt einen neuen Tag; beschädigte Datenbank.
    *   `save`: Platte voll (Ordner auf schreibgeschütztem Pfad), Timeout, `.m4a`-Fehler.
    *   `jobs`: `fake-worker` (Datei 40) für Erfolg, Fehler, Stille → Zeitlimit, Abbruch; `TooSmall` ohne Worker-Start.
    *   `ipc`: fremde SID wird abgewiesen (soweit im CI-Konto möglich), Pfadprüfung, `VersionMismatch`, `UnknownCommand`.

#### `crates/gameclip-worker` (Windows)

##### 56. `crates/gameclip-worker/Cargo.toml`, `src/main.rs`, `src/trim.rs`, `src/size_target.rs` (Create)

*   **Änderungen:**
    *   `main.rs`: `windows_subsystem = "windows"`; liest `WorkerJob` von stdin, schreibt `WorkerMessage` als JSON-Zeilen nach stdout, Exit 0/1. Setzt `IDXGIDevice::SetGPUThreadPriority(-7)` (niedrigster Wert).
    *   `trim.rs`: Ablauf Schritte 1–5 aus „Kürzen und Export“ mit `gameclip-media::decode`, `gameclip-gpu::Converter` und `encoder` (Profil `Export`); x264-Weg mit Kopie über Staging-Textur. In M0 ohne `TargetSize`.
    *   `size_target.rs` (ab M3): `export_plan::plan` (bei `TooSmall` sofort `Error{TooLarge}`), kodieren, Größe prüfen, höchstens ein Versuch mit `export_plan::correct`, dann `TooLarge`.
    *   Sicherung der Ausgabe wie in Datei 29 (`OpenOptions::write(true)` + `sync_all`).

##### 57. `crates/gameclip-worker/tests/trim.rs` (Create)

*   **Änderungen:** Mit `gameclip-testsupport::synth` (Datei 30): Schnitt bei Bild N ergibt laut eingebetteter Bildnummer Bild N als erstes und `out` als ausschließliche Grenze; die Audiolänge entspricht der Videolänge auf ±1 ms; `SeparateTracks` hat Reihenfolge und Standard-Markierung; `TargetSize` für 20 Dauern von 5 bis 300 s bleibt bei höchstens der Zielgröße oder endet mit `TooLarge`, wenn `export_plan` das vorhersagt.

#### `crates/gameclip-ui-platform` (Windows)

##### 58. `crates/gameclip-ui-platform/Cargo.toml`, `src/lib.rs`, `src/pipe_client.rs`, `src/service.rs`, `src/clipboard.rs`, `src/drag.rs`, `src/reveal.rs`, `src/thumbnail.rs`, `src/media.rs` (Create)

*   **Änderungen:** Freie Funktionen in `cfg(windows)`-Modulen, ohne Trait:
    *   `pipe_client.rs`: Verbindung zu `PIPE_NAME`, Prüfung des Servers über `GetNamedPipeServerProcessId` und Vergleich des Programmpfads mit `Package::Current().InstalledLocation()\gameclipd.exe`; Rahmung über `gameclip-protocol::framing`.
    *   `service.rs`: Start des Dienstes über `IApplicationActivationManager::ActivateApplication("<PFN>!GameClipService")`; Autostart über `StartupTask::GetAsync(STARTUP_TASK_ID)` mit `RequestEnableAsync`/`Disable`.
    *   `clipboard.rs` (`clipboard-win`, CF_HDROP), `drag.rs` (Crate `drag`), `reveal.rs` (`SHOpenFolderAndSelectItems`), `thumbnail.rs` (`IShellItemImageFactory::GetImage`, PNG-Kodierung über WIC), `media.rs`: einzige Stelle der Prüfung auf das Media Feature Pack, über `LoadLibraryExW("mfplat.dll", LOAD_LIBRARY_SEARCH_SYSTEM32)` (danach `FreeLibrary`). Media Foundation wird nie zur Ladezeit gelinkt, damit die Oberfläche auf N-Editionen startet und den Hinweis zeigen kann. Der Dienst meldet das nicht; die Oberfläche fragt es selbst ab.
*   **Begründung:** Alle Windows-Aufrufe der Oberfläche an einer Stelle; `gameclip-ui` selbst nennt `windows` nicht.

#### `apps/gameclip-ui` (Tauri + React)

##### 59. `apps/gameclip-ui/src-tauri/Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/icon.ico`, `icons/*.png` (Create)

*   **Änderungen:**
    *   `build.rs`: `tauri_build::build()`.
    *   `tauri.conf.json`: Tauri 2, ohne Updater; ein Fenster 1100×720, Mindestgröße 800×560 *(vorläufig)*; `bundle.active = false` (MSIX baut Datei 69); `frontendDist = "../dist"`; CSP: `default-src 'self'; media-src 'self' http://clip.localhost; img-src 'self' http://clip.localhost data:; connect-src ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'`.
    *   `capabilities/default.json`: `core:default`, `core:event:default`, `core:window:default`. Die eigenen Befehle sind App-Befehle und brauchen keine Plugin-Freigabe.

##### 60. `apps/gameclip-ui/src-tauri/src/{main,ipc,clip_protocol,commands}.rs` (Create)

*   **Änderungen:**
    *   `main.rs`: Beim Start über das Protokoll bekommt die App den URI selbst als Argument (`gameclip:reveal?id=<clip_id>`, Manifest ohne `Parameters`). `main.rs` erkennt dieses Argument, verbindet sich mit dem Dienst (Handshake, `GetClip`), zeigt die Datei per `reveal` im Explorer und beendet sich ohne Fenster. Ist der Dienst nicht erreichbar oder die ID unbekannt, öffnet es den Clip-Ordner aus `settings.toml` (bzw. den Standardordner).
    *   `ipc.rs`: nutzt `gameclip-ui-platform::pipe_client`. Ist keine Pipe erreichbar: `service::start()`, dann 10 s lang alle 500 ms erneut; danach Ereignis `service_unreachable` ans Frontend. Bei `VersionMismatch` Ereignis `version_mismatch`. Dienst-Ereignisse werden als Tauri-Events weitergegeben. Beim ersten Start `service::enable_autostart()`.
    *   `clip_protocol.rs`: `register_asynchronous_uri_scheme_protocol("clip", …)`. Routen `/<clip_id>` (Datei) und `/thumb/<clip_id>` (Vorschaubild). Pfad nur über `GetClip` vom Dienst. Range-Formen `bytes=a-b`, `bytes=a-` und `bytes=-n` werden unterstützt; die Antwort ist 206 mit höchstens 1 MiB Daten, korrektem `Content-Range: bytes a-e/size` und `Accept-Ranges: bytes` (der Player fordert den Rest selbst nach). Ohne Range → 200 mit der ganzen Datei als Stream. Unbekannte ID → 404; 416 nur, wenn `a ≥ size`.
    *   `commands.rs`: je Protokollbefehl ein Tauri-Befehl, dazu `copy_file`, `start_drag`, `reveal`, `set_autostart`.

##### 61. `apps/gameclip-ui/package.json`, `vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `index.html`, `src/test/setup.ts` (Create)

*   **Änderungen:** Abhängigkeiten `react@18`, `react-dom@18`, `@tauri-apps/api@2`; Entwicklung `@tauri-apps/cli@2`, `typescript@5`, `vite@6`, `@vitejs/plugin-react`, `vitest@3`, `jsdom`, `@testing-library/react`, `license-checker`. Skripte: `dev`, `dev:mock` (Vite mit `VITE_MOCK=1`), `build`, `test`, `typecheck`, `gen:types` (kopiert nichts; ts-rs schreibt direkt nach `src/generated/`, das Skript prüft nur das Vorhandensein).

##### 62. `apps/gameclip-ui/src/{ipc.ts,ipc.mock.ts,store.ts}`, `src/generated/*.ts` (Create)

*   **Änderungen:** `ipc.ts`: je Befehl eine typisierte Funktion über `invoke`, Ereignisse über `listen`. `ipc.mock.ts`: dieselbe Schnittstelle mit Beispieldaten und simuliertem Fortschritt; wird bei `VITE_MOCK=1` und in Tests verwendet. `store.ts`: externer Store mit `status`, `clips`, `exports`, `jobs`, `settings` über `useSyncExternalStore`. `generated/`: von ts-rs erzeugt, eingecheckt, nicht von Hand bearbeiten.

##### 63. `apps/gameclip-ui/src/{main.tsx,App.tsx}` (Create)

*   **Änderungen:** Ansichten `FirstRun`, `Gallery`, `Editor`, `Settings`; `FirstRun` solange `first_run_done == false`; Statusleiste; Vollbild-Meldungen für `version_mismatch` und `service_unreachable`. Beim Öffnen der Galerie `WatchFolders{on: true}`, beim Verlassen `{on: false}`.

##### 64. `apps/gameclip-ui/src/views/{FirstRun,Gallery,Editor,Settings}.tsx` (Create)

*   **Änderungen:**
    *   `FirstRun`: Mikrofon bestätigen oder wechseln, Hinweis „Das Mikrofon wird dauerhaft im Hintergrund aufgenommen, solange GameClip läuft“, Pegel über `SubscribeMicLevel`; Hotkey mit Testdruck und Warnung; Clip-Ordner; dann „Test-Clip erstellen“. Fehlt das Media Feature Pack (`GetCapabilities`), steht der Hinweis vorne.
    *   `Gallery`: Reiter „Clips“ und „Exporte“, Kacheln (Vorschaubild über `http://clip.localhost/thumb/<id>`), Sortierung nach Datum, Platzanzeige „7,2 / 10 GB“ aus `used_bytes`/`limit_bytes` (nur eigene Clips im aktuellen Ordner), Stern, Markierung „Fremd“ mit „Als eigenen Clip übernehmen“, Clips aus früheren Ordnern grau, Kontextmenü (Kopieren, Im Ordner anzeigen, Löschen mit Rückfrage; endgültig). Videos ohne `gameclip_id` werden nicht angezeigt.
    *   `Editor`: Player, `TrimBar`, Bildschritt mit Pfeiltasten, `ExportDialog`.
    *   `Settings`: Aufnahme (Länge, Auflösung, Stufe, Monitor, Mauszeiger, `ForecastLabel`), Audio (Geräte, Discord, Lautstärken), Hotkey, Speicher (Ordner, Limits), Töne, Autostart (`set_autostart`), Diagnose.

##### 65. `apps/gameclip-ui/src/components/{TrimBar,ExportDialog,HotkeyPicker,VolumeSlider,ForecastLabel,ClipCard,StatusBar}.tsx`, `src/timeline.ts` (Create)

*   **Änderungen:**
    *   `timeline.ts`: `toFrameIndex(currentTime_s, frames_us)` per Binärsuche und `toCurrentTime(us)`; Start rastet auf ein Bild, Ende auf die Zeit des Bildes **nach** dem letzten gewünschten Bild (ausschließlich). Ist das letzte gewünschte Bild das letzte des Clips, ist das Ende `duration_us`.
    *   `TrimBar`: Griffe für Start und Ende, Position über `ref` und `requestVideoFrameCallback`.
    *   `ExportDialog`: Mischung (Standard) / Getrennte Spuren / Zielgröße (MB-Feld); vorab `GetExportEstimate` mit Warnung; Fortschritt, Encoder („Export über CPU“), Abbrechen; eingereihte Aufträge aus `ListJobs` als „wartet“.
    *   `HotkeyPicker`: `SuspendHotkey{on: true}`, liest `KeyboardEvent.code` plus Modifier, Auswahlliste für F13–F24, `TestRegister`, Warnung; bei Bestätigen `SetSettings{hotkey}`, dann `SuspendHotkey{on: false}`.
    *   `VolumeSlider`: −30 bis +12 dB, Doppelklick → 0 dB. `ForecastLabel`: „ca. N MB RAM“ aus `total_bytes` und Warnstufe.

##### 66. `apps/gameclip-ui/src/styles/tokens.css`, `src/**/*.module.css` (Create)

*   **Änderungen:** CSS-Variablen für Farben (hell/dunkel über `prefers-color-scheme`), Abstände, Schrift (Segoe UI Variable), Radien; ein Modul je Komponente.

##### 67. `apps/gameclip-ui/src/**/*.test.tsx`, `src/timeline.test.ts` (Create)

*   **Änderungen:** Gegen `ipc.mock.ts`: `timeline` bildet `currentTime` auf den richtigen Bildindex ab (inklusive Rand am Ende); `TrimBar` rastet ein; `ExportDialog` schickt das richtige Profil und zeigt die Vorab-Warnung; `HotkeyPicker` lehnt F12 ab und hebt `SuspendHotkey` beim Schließen auf; `Gallery` markiert Fremd-Clips und rechnet sie nicht in die Platzanzeige; `FirstRun` durchläuft alle Schritte.

#### Paket

##### 68. `packaging/AppxManifest.template.xml` (Create)

*   **Änderungen:**
    *   Namespaces `uap`, `uap5`, `rescap`.
    *   `Identity Name="io.github.poldion.GameClip"`, `Publisher` = Platzhalter (Zertifikatssubjekt, Q2), `Version`.
    *   `TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.26100.0" MaxVersionTested="10.0.26100.0"`.
    *   Applications, beide mit `EntryPoint="Windows.FullTrustApplication"`:
        *   `GameClip` (`Executable="gameclip-ui.exe"`, Startmenü) mit `<uap:Extension Category="windows.protocol"><uap:Protocol Name="gameclip"/></uap:Extension>` (ohne `Parameters`, der URI kommt als Argument).
        *   `GameClipService` (`Executable="gameclipd.exe"`, `uap:VisualElements AppListEntry="none"`) mit `<uap5:Extension Category="windows.startupTask" Executable="gameclipd.exe" EntryPoint="Windows.FullTrustApplication"><uap5:StartupTask TaskId="GameClipServiceStartup" Enabled="false" DisplayName="GameClip"/></uap5:Extension>`.
    *   Das M0-Paket enthält nur `GameClipService` und den Worker.
    *   Capabilities: `<rescap:Capability Name="runFullTrust"/>`, `<DeviceCapability Name="microphone"/>`, `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` im laut Q15 richtigen Namespace.

##### 69. `packaging/build-msix.ps1` (Create)

*   **Änderungen:** Setzt `packaging/out/layout/` aus den Release-Builds, FFmpeg-DLLs, Tönen, Lizenzen und Icons zusammen, ersetzt Platzhalter, ruft `MakeAppx pack` auf. Schalter:
    *   `-TestCert`: erzeugt bei Bedarf mit `New-SelfSignedCertificate` ein Zertifikat mit Subjekt = Publisher, importiert es nach `Cert:\LocalMachine\TrustedPeople` (Administratorrechte), signiert mit `SignTool sign /fd SHA256`.
    *   `-NoSign`: nur packen (für die SignPath-Action in CI).

##### 70. `packaging/ffmpeg/fetch-ffmpeg.ps1`, `packaging/ffmpeg/ffmpeg.lock`, `packaging/licenses.ps1` (Create)

*   **Änderungen:** `ffmpeg.lock` enthält URL des gespiegelten Archivs (Release-Datei im eigenen Repository, ursprünglich BtbN, datierter Autobuild `autobuild-2026-11-04-12-51`, Datei `ffmpeg-n8.1.3-34-ga35157abb9-win64-gpl-shared-8.1.zip`; BtbN bietet keinen 7.1-Build mehr an, und `latest` wird täglich neu gebaut), SHA-256 und URL des Quellarchivs. Das Skript lädt, prüft die Prüfsumme, entpackt nach `third_party/ffmpeg/` (per `.gitignore` ausgeschlossen), erzeugt mit `tools/ffmpeg-bindgen` (eigener Workspace, bindgen 0.73) die Datei `third_party/ffmpeg/binding.rs` für `FFMPEG_BINDING_PATH` und ergänzt in CI `PATH` um `third_party/ffmpeg/bin`. `licenses.ps1` fügt npm- und FFmpeg-Lizenzen an `THIRD_PARTY_LICENSES.md` an.

##### 71. `packaging/assets/*.png` (Create)

*   **Änderungen:** App-Icons in den MSIX-Größen (44, 50, 150, 310 px) und die sechs Tray-Symbole (16, 20, 24, 32 px) *(Größen vorläufig)*.

### Verification Plan

*   **Compilation/Build:** `pwsh packaging/ffmpeg/fetch-ffmpeg.ps1`; `npm ci && npm run build` in `apps/gameclip-ui`; `cargo build --workspace --release`; `pwsh packaging/build-msix.ps1 -TestCert`.
*   **Unit Tests:** `cargo test --workspace`; `npm test` in `apps/gameclip-ui`.
*   **Static Analysis & Linting:** `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo deny check licenses`; Grenz-Prüfung per `cargo tree` (Datei 7); `npm run typecheck`.
*   **Test Coverage Requirement:** Zeilenabdeckung mit `cargo llvm-cov` (stabile Toolchain): Workspace ≥ 85 %, `gameclip-core` ≥ 90 %, `retention.rs` und `export_plan.rs` 100 %.
*   **E2E/Integration:** `cargo test -p gameclip-media -p gameclip-worker -p gameclipd` (synthetische Quelle, Prüfung per ffprobe, Temp-Ordner); Linux-CI-Job für Core, Protocol und Frontend.
*   **Manuelle Prototypen und Abnahme** (Protokolle in `docs/test-protocols.md`):

    | Test | Meilenstein | Bestanden, wenn |
    | --- | --- | --- |
    | S1 | M0 | Vanguard 48 h ohne Befund und 14 Tage ohne Sperre; kein schwarzes Bild randlos und im exklusiven Vollbild (sonst DDA, Q4); Speicherzeit p95 ≤ 3 s; CPU ≤ 3 % *(vorläufig)*; Dienst ≤ 80 MB Private Bytes plus Puffer *(vorläufig)*; im MSIX: Autostart, Pipe (über `gameclipd.exe --save-clip`), Loopback und randlose Aufnahme funktionieren; Toast des Dienstes erscheint |
    | S2 | M0 | NVENC, AMF und QSV kodieren ohne CPU-Kopie aus dem Texture-Array-Pool; IDR auf Anforderung bei allen drei; Laptop mit zwei GPUs; HDR wird erkannt; `dumpbin /dependents` der FFmpeg-DLLs ohne `mfplat.dll`; dokumentiert, ob es Hardware gibt, auf der nur `h264_mf` funktioniert (Q5) |
    | S4 | M0 / M2 | M0: A/V-Versatz < 1 Bild nach 30 min, eigener Ton nicht im nächsten Clip, Mischung und `.m4a` auf ±1 Sample deckungsgleich. M2: Discord-Spur getrennt und nicht doppelt in der Mischung |
    | S5 | M3 | Spulen in einem 5-min-Clip über das `clip`-Protokoll (`http://clip.localhost`) ohne Fehler, auch bei offenen Bereichen (`bytes=0-`); N-Edition wird erkannt |
    | S6 | M0 | Datei kommt per Zwischenablage (Tray-Aktion) in Discord und Explorer an; ab M3 auch per Drag-and-drop |
    | S7 | M4 | signiertes MSIX auf frischer VM ohne SmartScreen- und Defender-Warnung |
    | S8 | M0 | F8 und per SendInput gesendetes F13 lösen unter Vanguard aus; Verhalten in einem Raw-Input- und einem DirectInput-Spiel dokumentiert (Q6) |
    | S9 | M0 | Kürzen per Worker-Grundgerüst bei laufendem Spiel: keine verworfenen Bilder im Puffer, Bildrate des Spiels sinkt höchstens 10 % *(vorläufig)*, Schnitt bei Bild N ergibt Bild N; 60-s-Clip in ≤ 15 s *(vorläufig, Q7)* |
    | S11 | M3 | 20 Größen-Exporte, jeder höchstens Zielgröße oder `TooLarge` wie von `export_plan` vorhergesagt |
    | G1a | Ende M0 | Ausgangswerte Game Bar/OBS für alle vier Größen; GameClip besser (≥ 10 % *(vorläufig)*) bei Größe 2 oder 4 |
    | G1b | nach M3 | GameClip besser (≥ 10 % *(vorläufig)*) als Game Bar bei mindestens 2 von 4 Größen |

--------------------------------------------------------------------------------

<!-- Provenance: Indicates initial structural drafting via EGM. -->

*Drafted with EGM*
