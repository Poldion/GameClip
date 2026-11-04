# GameClip v1 – Clip-Rekorder für Windows 11

*   **Author:** Pold
*   **Status:** Draft
*   **Date:** 2026-11-04
*   **Grundlage:** [concepts_brief.md](concepts_brief.md), Revision 9 (Entscheidungen D1–D25)

--------------------------------------------------------------------------------

## Overview

GameClip ist eine Open-Source-Desktop-App (GPL-3.0-or-later) für Windows 11. Sie nimmt im Hintergrund ständig den Bildschirm und den Ton auf. Die letzten 1 bis 5 Minuten hält sie als bereits kodierte Video- und Audiodaten im Arbeitsspeicher. Drückt der Spieler einen Hotkey (Standard F8), schreibt GameClip diesen Puffer als MP4-Datei in einen gewählten Ordner.

In einer Galerie kann der Spieler seine Clips ansehen, bildgenau kürzen, auf eine Zielgröße verkleinern (z. B. 10 MB für Discord) und per Zwischenablage oder Drag-and-drop in einen Messenger geben. Die App funktioniert ohne Konto, ohne Cloud und ohne Internetverbindung.

GameClip besteht aus zwei Programmen:

*   `gameclipd` läuft dauerhaft im Hintergrund. Er nimmt auf, hält den Puffer, reagiert auf den Hotkey und zeigt ein Tray-Symbol.
*   `gameclip-ui` ist das Fenster mit Galerie und Einstellungen. Es startet nur, wenn der Nutzer es öffnet.

Linux (Ubuntu) folgt in einer späteren Version. Dort übernimmt das bestehende Projekt gpu-screen-recorder die Aufnahme. Das Fenster, die Einstellungen und das Protokoll zwischen den beiden Programmen werden deshalb von Anfang an ohne Windows-Code geschrieben.

## The Problem

Spieler wollen besondere Momente festhalten, ohne vorher zu wissen, wann sie passieren. Dafür gibt es „Instant Replay“: Die Software nimmt ständig auf und speichert erst auf Tastendruck die letzten Minuten.

Die vorhandenen Lösungen haben jeweils einen Haken:

| Lösung | Haken |
| --- | --- |
| Medal | Konto, Cloud-Funktionen und Werbung; nicht Open Source |
| NVIDIA App Instant Replay / AMD ReLive | nur für die GPU des jeweiligen Herstellers |
| Xbox Game Bar | vorinstalliert und ohne Konto, aber kaum Funktionen zum Kürzen und Teilen; Clips sind oft zu groß für Discord |
| OBS Replay Buffer | viele Einstellungen, für reines Clippen umständlich einzurichten; kein Editor |

Der Ablauf, den GameClip verkürzen soll: **Hotkey drücken → Clip kürzen → auf unter 10 MB bringen → in Discord oder WhatsApp einfügen.** Heute braucht man dafür meist zwei oder drei Programme.

Dazu kommt eine harte Bedingung: Viele Spieler spielen Valorant. Dessen Anti-Cheat Riot Vanguard läuft im Kernel und blockiert oder meldet Software, die Code in das Spiel einschleust. GameClip darf deshalb nur Windows-Schnittstellen nutzen, die von außen auf den Bildschirm zugreifen, ohne den Spielprozess anzufassen.

**Risiko für das ganze Projekt:** Die Aufnahme allein ist kein Alleinstellungsmerkmal. Game Bar ist auf jedem Windows-Rechner schon vorhanden. Ob GameClip spürbar besser ist, wird in Phase 0 gemessen (Ziel G1). Fällt der Vergleich schlecht aus, wird der Umfang überdacht, bevor die Oberfläche gebaut wird.

## Goals

Die Zahlenwerte sind vorläufig. Sie werden vor dem Prototyp S1 auf einem benannten Referenzrechner festgelegt (Open Question Q1).

*   **G1 – Spürbar schneller als Game Bar.** In Phase 0 werden GameClip, Xbox Game Bar und OBS Replay Buffer auf demselben Rechner verglichen:
    1.  Minuten von der Installation bis zum ersten Clip
    2.  RAM und CPU im Leerlauf, mit laufendem Puffer und Spiel
    3.  Anzahl der Einstellungen bis zu einem brauchbaren Clip
    4.  Zeit vom Hotkey bis zu einer Datei unter 10 MB in der Zwischenablage

    GameClip muss Game Bar bei mindestens 2 der 4 Werte schlagen.
*   **G2 – Speichern in höchstens 3 Sekunden.** Gemessen vom Tastendruck bis zur fertig geschriebenen MP4-Datei. Der Wert gilt in 95 % der Fälle, mit Standardeinstellungen, auf einer SSD und mit Hardware-Encoder.
*   **G3 – Wenig Verbrauch im Hintergrund.**
    *   `gameclipd` ohne Puffer: höchstens 60 bis 80 MB RAM.
    *   Puffer: so groß wie in den Einstellungen angezeigt, höchstens 5 % Abweichung.
    *   CPU: höchstens ~3 %, gemittelt über 10 Minuten mit Hardware-Encoder.
*   **G4 – Funktioniert mit Valorant.** Im Prototyp S1 laufen 48 Stunden ohne Vanguard-Fehler und ohne schwarzes Bild, danach 14 Tage ohne Kontosperre. Der Test wird vor jedem Release wiederholt.
*   **G5 – Clips sind überall vollständig zu hören.** Jede Datei im Clip-Ordner und im Export-Ordner enthält eine Mischung aus System-Ton, Mikrofon und (falls aktiviert) Discord. Sie klingt auf Handy, in WhatsApp und Discord vollständig.
*   **G6 – Einzelspuren für den Schnitt.** System, Mikrofon und Discord werden zusätzlich getrennt aufgenommen. Sie lassen sich auf Wunsch mit exportieren.
*   **G7 – Bildgenaues Kürzen.** Ein Schnitt bei Bild N ergibt eine Datei, deren erstes Bild laut ffprobe Bild N ist.
*   **G8 – Export auf Zielgröße.** Der Größen-Export (Standard 10 MB) überschreitet die Zielgröße nie. Prüfung: 20 Testclips, keiner zu groß.
*   **G9 – Begrenzter Speicherplatz.** Clip-Ordner und Export-Ordner haben je ein Limit (Standard 10 GB). Bei Überschreitung löscht GameClip die ältesten eigenen Clips. Mit Stern markierte Clips und fremde Dateien bleiben unangetastet.
*   **G10 – Übersichtliche Oberfläche.** Ein neuer Nutzer kommt mit höchstens drei Einrichtungsschritten zum ersten Clip: Mikrofon prüfen, Hotkey testen, Ordner wählen.

## Non-Goals

*   **Linux in Version 1.** Kommt später mit gpu-screen-recorder als Aufnahmekern.
*   **Windows 10.** Der Support endete im Oktober 2025.
*   **Steam OS, macOS.**
*   **Cloud-Upload, Konten, Social-Funktionen, Updates, die von selbst ins Internet gehen.**
*   **Overlay im Spiel.** Bräuchte Code-Injektion und verträgt sich nicht mit Vanguard.
*   **HDR-Aufnahme.** v1 nimmt in SDR auf und warnt, wenn Windows-HDR aktiv ist.
*   **Nur den Ton des Spiels aufnehmen.** v1 nimmt den System-Ton auf, Discord optional getrennt.
*   **Mischregler im Editor.** Lautstärkeregler gibt es nur in den Einstellungen und nur für die Mischung.
*   **Bearbeitung über das Kürzen hinaus.** Kein Text, keine Übergänge, kein Zusammenfügen.
*   **Automatische Highlight-Erkennung, Webcam, Livestreaming.**
*   **Öffentliche Kommandozeile.** `gameclipd.exe --save-clip` existiert nur intern, ohne Dokumentation, Alias und Stabilitätszusage.

## Open Questions

*   **Q1 – Zahlen und Referenzrechner.** Konkrete Grenzwerte für RAM (Dienst, Puffer, offenes Fenster, Spitze beim Export), CPU und Bildverluste im Spiel. Bitraten der Qualitätsstufen. Welcher Rechner und welches Spiel dienen als Referenz?
*   **Q2 – MSIX-Publisher.** Paketformat (MSIX) und App-ID (`io.github.poldion.GameClip`) sind entschieden. Der `Publisher` im Manifest muss genau dem Subjekt des Signaturzertifikats entsprechen (z. B. von SignPath). Er steht erst fest, wenn das Zertifikat da ist, und darf sich danach nicht mehr ändern, sonst gilt jedes Update als neue App.
*   **Q3 – Mindest-Build von Windows 11.** Zum Beispiel 24H2 (26100). Hängt davon ab, ab wann `MinUpdateInterval` und die randlose Aufnahme verfügbar sind.
*   **Q4 – Exklusives Vollbild.** Liefert WGC dort ein Bild? Wird DDA als Ausweichweg gebraucht? Antwort aus S1.
*   **Q5 – Media Foundation als Zwischenstufe.** Gibt es Hardware, auf der NVENC/AMF/QSV versagen, Media Foundation aber funktioniert? Antwort aus S2.
*   **Q6 – Sieht das Spiel die Hotkey-Taste noch?** Bestimmt den Warnungstext im Auswahldialog. Antwort aus S8.
*   **Q7 – Smart-Cut oder volles Neukodieren beim Kürzen.** Antwort aus S9.
*   **Q11 – Mehrere ausgeschlossene Prozesse pro Loopback-Strom.** Wenn Windows das nicht kann, schaltet GameClip die Spur „System“ während des eigenen Bestätigungstons stumm.
*   **Q12 – Clip und Einzelspuren verbinden.** Über eine ID im MP4-Metadaten-Tag. Was passiert mit verwaisten Einzelspuren? Zählen sie zum Limit?
*   **Q13 – Sicheres Löschen.** Was gilt genau als GameClip-eigene Datei? Wie verhält sich die App, wenn Clip- und Export-Ordner derselbe Ordner sind oder fremde Videos enthalten?

## Critical User Journeys (CUJs)

### CUJ 1: Erster Start bis zum ersten Clip

*   **Given**: GameClip ist frisch installiert. Ein Mikrofon ist angeschlossen, eine NVIDIA-, AMD- oder Intel-GPU ist vorhanden.
*   **When**: Der Nutzer startet GameClip zum ersten Mal und folgt der Checkliste. Erst bestätigt oder wechselt er das Mikrofon (standardmäßig an). Dann drückt er den Hotkey einmal zum Test (Standard F8). Zuletzt wählt er den Clip-Ordner.
*   **Then**: Die Statuszeile zeigt z. B. „Puffert 1:00 in 1080p60 über NVENC“. Ein Druck auf „Jetzt Test-Clip erstellen“ legt eine MP4 im Clip-Ordner ab, und der Erfolgston ertönt.

### CUJ 2: Clip im Spiel speichern

*   **Given**: `gameclipd` puffert. Der Nutzer spielt Valorant im Vollbild.
*   **When**: Er drückt den Hotkey.
*   **Then**: Innerhalb von 3 Sekunden liegt eine MP4 nur mit der Mischspur im Clip-Ordner. Die Einzelspuren liegen intern. Der Erfolgston ertönt, Vanguard meldet nichts. Ist der Clip-Ordner über seinem Limit, wurden die ältesten nicht geschützten GameClip-Clips gelöscht.

### CUJ 3: Clip kürzen und in Discord teilen

*   **Given**: Ein Clip liegt im Clip-Ordner.
*   **When**: Der Nutzer öffnet die Galerie, setzt Start- und Endpunkt bildgenau und wählt „Für Discord exportieren (10 MB)“.
*   **Then**: Während das Spiel weiterläuft, erzeugt ein Arbeitsprozess eine Datei unter 10 MB im Export-Ordner. Der Puffer verliert dabei keine Bilder. Die Datei liegt per Knopfdruck in der Zwischenablage, Strg+V in Discord fügt sie ein.

### CUJ 4: Clip mit getrennten Spuren für den Schnitt exportieren

*   **Given**: Ein Clip liegt im Clip-Ordner, seine Einzelspuren liegen intern.
*   **When**: Der Nutzer wählt im Export-Dialog „Mit getrennten Spuren exportieren“.
*   **Then**: Im Export-Ordner entsteht eine MP4 mit den Spuren „Mischung“ (Standard), „System“, „Mikrofon“ und gegebenenfalls „Discord“. Ein Player spielt nur die Mischung ab. Ein Schnittprogramm zeigt alle Spuren getrennt.

### CUJ 5: Speicherlimit erreicht

*   **Given**: Der Clip-Ordner enthält 10 GB GameClip-Clips, einige davon mit Stern.
*   **When**: Der Nutzer speichert einen neuen Clip.
*   **Then**: GameClip löscht die ältesten Clips ohne Stern samt ihren Einzelspuren endgültig, bis der neue Clip ins Limit passt. Fremde Dateien bleiben unangetastet. Reicht der Platz trotzdem nicht, weil alles geschützt ist, wird nichts gelöscht, der Clip trotzdem gespeichert und eine Warnung angezeigt.

### CUJ 6: Hotkey ist schon belegt

*   **Given**: Eine andere App hat F8 bereits per RegisterHotKey belegt.
*   **When**: `gameclipd` startet beim Login.
*   **Then**: Das Tray zeigt „Hotkey nicht aktiv“, es kommen ein Fehlerton und eine Benachrichtigung. Im Auswahldialog lässt sich eine andere Taste wählen und sofort testen, auch F13 bis F24.

## Technical Plan (Architecture)

### Architekturentscheidungen

Diese Punkte hatten mehr als eine vertretbare Lösung und wurden vom Autor entschieden.

| ID | Frage | Optionen |
| --- | --- | --- |
| T1 | Paketformat und Installer | **Entschieden: volles MSIX** (siehe „Paket und Installation“) |
| T2 | Nachrichtenformat auf der Named Pipe | **Entschieden: Längenpräfix + JSON** (siehe „Dienst-Schnittstelle“) |
| T3 | Wo der Clip-Index liegt (Stern, Eigentum, Zuordnung zu Einzelspuren) | **Entschieden: SQLite im App-Ordner** (siehe „Clip-Index“) |
| T4 | Arbeitsprozess für Kürzen und Export | **Entschieden: eigenes Programm `gameclip-worker`** (siehe „Kürzen und Export“) |
| T5 | Frontend-Framework in Tauri | **Entschieden: React** (siehe „Oberfläche“) |

### Überblick

```text
┌──────────────────────────── gameclipd (immer aktiv, kein WebView) ───────────────────────────┐
│                                                                                              │
│  WGC-Monitor ─► GPU-Kopie ─► Taktgeber 60 fps ─► Skalieren + BGRA→NV12 ─► HW-Encoder (H.264) ─┐│
│                                                                                              ││
│  WASAPI: System-Loopback ─┐                                                                  ││
│          Mikrofon ────────┼─► Resampler 48 kHz ─┬─► AAC je Quelle ──────────────────────────┤│
│          Discord-Loopback ┘                    └─► Mixer (Gain, Limiter) ─► AAC Mischung ───┤│
│                                                                                              ▼│
│                                                     Ringpuffer (Blöcke + Paketindex) ◄─────────┘
│                                                              │
│  Hotkey (RegisterHotKey) ──► Zustandsmaschine ──► Speichern ─┴─► MP4 (Mischung) → Clip-Ordner
│                                     │                         └─► .m4a (Einzelspuren) → tracks\
│  Tray · Töne · Benachrichtigungen ◄─┤
│                                     ├──► Auftragswarteschlange ──► gameclip-worker ──► Export-Ordner
│  Clip-Index (SQLite) · Einstellungen┤
│                                     └──► Named Pipe (nur aktueller Benutzer, Längenpräfix + JSON)
└──────────────────────────────────────────────┬───────────────────────────────────────────────┘
                                               │
                         ┌─────────────────────┴─────────────────────┐
                         │ gameclip-ui (Tauri + WebView2, bei Bedarf)│
                         │ Galerie · Trim · Export · Einstellungen   │
                         │ Zwischenablage · Drag-out · Explorer      │
                         └───────────────────────────────────────────┘
```

### Komponenten

| Komponente | Aufgabe | Läuft in |
| --- | --- | --- |
| Aufnahme Video | WGC-Monitoraufnahme, GPU-Kopie, Taktgeber, Skalierung, NV12-Umwandlung | gameclipd, eigener Thread |
| Encoder | FFmpeg mit Hardware-Frames, Encoder-Auswahl und Ausweichkette | gameclipd, eigener Thread |
| Aufnahme Audio | WASAPI-Quellen, Resampler, Mixer, AAC-Encoder | gameclipd, ein Thread je Quelle plus Mixer |
| Ringpuffer | Pakete speichern, alte Bildgruppen verwerfen, Schnappschüsse beim Speichern | gameclipd |
| Muxer | MP4 und .m4a aus einem Schnappschuss schreiben | gameclipd, eigener Thread je Speichervorgang |
| Zustandsmaschine | Zustände und Übergänge, ohne Betriebssystem-Code | gameclip-core |
| Hotkey | RegisterHotKey auf einem eigenen Nachrichten-Thread | gameclipd |
| Tray, Töne, Benachrichtigungen | Status, Menü, Erfolgs-/Fehlerton, Toasts | gameclipd |
| Clip-Index und Speicherlimit | Eigentum, Stern, Zuordnung, Löschen der ältesten Clips | gameclipd (SQLite) |
| Auftragswarteschlange | Kürzen und Export, höchstens ein Auftrag gleichzeitig | gameclipd, startet `gameclip-worker` |
| Arbeitsprozess | Dekodieren, Schneiden, Kodieren, Muxen, Größen-Prüfung | `gameclip-worker`, ein Prozess je Auftrag |
| IPC-Server | Named Pipe, Befehle und Ereignisse | gameclipd [T2] |
| Oberfläche | Galerie, Player, Trim, Export-Dialog, Einstellungen, erster Start | gameclip-ui (React) |
| UI-Plattformdienste | Zwischenablage, Drag-out, Explorer, Vorschaubilder | gameclip-ui, Rust-Seite von Tauri |

### Video-Pipeline

1.  **Aufnahme.** `GraphicsCaptureItem` für den gewählten Monitor (Standard: Hauptmonitor). `IsBorderRequired = false` nach `RequestAccessAsync(Borderless)`, `IsCursorCaptureEnabled` laut Einstellung. Der Frame-Pool liefert BGRA-Texturen auf einem gemeinsamen `ID3D11Device` mit Video-Unterstützung und Multithread-Schutz. Dieses Gerät wird auch an FFmpeg übergeben.
2.  **Kopie.** WGC verwendet seine Texturen wieder, deshalb wird jedes Bild einmal auf der GPU in eine eigene Textur kopiert. Ein Zurücklesen in den Hauptspeicher gibt es nicht. Die einzige Ausnahme ist der x264-Weg (siehe 5).
3.  **Taktgeber.** Ein Thread läuft im festen 60-fps-Takt nach QPC. Bei jedem Takt nimmt er das neueste Bild. Ist keines neu, wiederholt er das letzte. Kommen mehrere Bilder an, verwirft er alle außer dem neuesten. Wo es der Windows-Build erlaubt, begrenzt `MinUpdateInterval` die Aufnahmerate schon vorher.
4.  **Skalieren und Umwandeln.** Der D3D11-VideoProcessor skaliert auf die feste Ausgabeauflösung (Standard 1080p) und wandelt BGRA in NV12 um (BT.709, begrenzter Bereich). Wechselt die Quelle die Auflösung, ändert sich nur die Eingabe des VideoProcessors. Der Encoder läuft weiter.
5.  **Kodieren.** Die NV12-Textur geht als `AV_PIX_FMT_D3D11`-Frame an den Encoder. Die Ausweichkette prüft beim Start mit einer echten Testsitzung:
    1.  `h264_nvenc`, `h264_amf`, `h264_qsv` (QSV über abgeleitete Hardware-Frames) – je nach GPU am Monitor
    2.  `h264_mf` (Media Foundation), nur wenn S2 einen Nutzen zeigt (Q5)
    3.  `libx264` mit Preset `superfast` und Warnung „Eingeschränkt: CPU-Encoder“. Nur hier wird das Bild in den Hauptspeicher kopiert.

    Einstellungen für alle Encoder: H.264 High, geschlossene Bildgruppen, ein Keyframe pro Sekunde (60 Bilder), keine B-Frames (dadurch fast keine Verzögerung beim Speichern), begrenzte Maximalbitrate (CBR oder VBR mit `maxrate`).
6.  **Neustart.** Bei Geräteverlust (TDR, Standby, Monitorwechsel) baut die Pipeline Gerät und Encoder neu auf. Im Puffer beginnt ein neuer Abschnitt. Die Zustandsmaschine wechselt nach „Erholt sich“.
7.  **HDR.** Ist Windows-HDR auf dem gewählten Monitor aktiv, zeigt die App „Eingeschränkt: HDR wird nicht unterstützt, Farben können blass wirken“.

### Audio-Pipeline

| Quelle | Mechanismus | Wann |
| --- | --- | --- |
| System | Prozess-Loopback mit `EXCLUDE_TARGET_PROCESS_TREE`. Ausgeschlossen ist gameclipd, oder Discord, wenn die Discord-Spur an ist (Q11) | immer |
| Mikrofon | WASAPI-Aufnahme im Shared Mode, Gerät wählbar (Standard: Windows-Standardmikrofon) | standardmäßig an |
| Discord | Prozess-Loopback mit `INCLUDE_TARGET_PROCESS_TREE` auf `Discord.exe`; der Dienst sucht den Prozess und bindet neu, wenn Discord startet oder beendet wird | optional |

*   Jede Quelle wird auf 48 kHz Stereo Float umgerechnet. Lücken (WASAPI liefert bei Stille keine Pakete) werden mit Stille aufgefüllt.
*   Zeitstempel kommen von derselben QPC-Uhr wie beim Video. Drift zwischen Geräten gleicht der Resampler aus.
*   **Einzelspuren:** jede Quelle wird ohne Lautstärkeänderung als AAC-LC mit 128 kbit/s kodiert (FFmpeg-Encoder `aac`).
*   **Mischung:** Summe der Quellen mit den Lautstärkereglern (Standard 0 dB), danach ein Limiter gegen Übersteuern, kodiert als AAC-LC mit 160 kbit/s.
*   **Eigener Ton:** Ist gameclipd nicht ausgeschlossen (weil Discord ausgeschlossen ist und Windows nur einen ausgeschlossenen Prozessbaum erlaubt), merkt sich der Dienst Beginn und Dauer seines Erfolgs- bzw. Fehlertons. Die Spur „System“ wird in diesem Zeitfenster stummgeschaltet.
*   Wird das Mikrofon pausiert, liefert seine Quelle Stille. Die Spur bleibt also erhalten.

### Ringpuffer

*   **Speicher:** Ein Pool aus Blöcken zu 2 MB, beim Start reserviert. Die Größe ergibt sich aus der Prognose plus 15 % Reserve. Ändert der Nutzer Länge oder Qualität, wird der Puffer neu angelegt („Puffer wird zurückgesetzt“).
*   **Paketindex:** Je Paket ein Eintrag `{spur, pts, dts, keyframe, block, offset, länge, abschnitt}`. Spuren: Video, Mischung, System, Mikrofon, Discord.
*   **Verwerfen:** Gilt eine Zeitgrenze (Pufferlänge) und eine Bytegrenze (Poolgröße). Verworfen wird immer die älteste ganze Video-Bildgruppe. Audiopakete vor dem neuen ältesten Keyframe fallen mit weg.
*   **Schnappschuss:** Beim Speichern werden die betroffenen Blöcke per Referenzzählung festgehalten. Der Encoder schreibt in freie Blöcke weiter. Ist die Reserve erschöpft, verwirft der Encoder die *neuesten* Pakete und meldet das. Ein laufender Speichervorgang wird nie beschädigt.
*   **Prognose (eine Funktion für Dienst und Oberfläche):** `(V_max + Σ Audiobitraten) × (T + 1 s) / 8 × 1,05`, aufgerundet auf ganze Blöcke, plus Grundbedarf. Die Oberfläche zeigt z. B. „ca. 160 MB RAM“. Ab 10 % des eingebauten RAM gibt es eine gelbe Warnung mit Rückfrage, ab 25 % wird die Einstellung abgelehnt.

### Speichern per Hotkey

1.  WM_HOTKEY kommt an. Die Zustandsmaschine prüft, ob gerade gepuffert wird. Läuft schon ein Speichervorgang, wird höchstens ein weiterer eingereiht.
2.  Der Dienst nimmt den QPC-Zeitstempel des Tastendrucks und wartet, bis das Videopaket mit diesem Zeitpunkt im Puffer liegt (ohne B-Frames höchstens wenige Bilder).
3.  Schnappschuss: vom letzten Keyframe vor `Tastendruck − Pufferlänge` bis zum Tastendruck, nur aus dem neuesten Abschnitt. Ist der Puffer noch nicht voll, wird gespeichert, was da ist.
4.  Es werden zwei Dateien geschrieben, jeweils zuerst als `.part`, dann `fsync`, dann umbenannt:
    *   `<Clip-Ordner>\<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4` mit Video und der Spur „Mischung“. Das moov-Atom kommt ans Ende, damit die Datei in einem Durchgang geschrieben wird. Die Datei bekommt einen Metadaten-Tag `gameclip_id = <UUID>`.
    *   `%LOCALAPPDATA%\GameClip\tracks\<UUID>.m4a` mit den Einzelspuren „System“, „Mikrofon“ und gegebenenfalls „Discord“, mit Spurtiteln.
5.  Der Programmname kommt vom Prozess des Vordergrundfensters, ersatzweise „Clip“. Unerlaubte Zeichen werden ersetzt, bei Namensgleichheit wird `_2` angehängt.
6.  Der Clip wird in den Index eingetragen, danach greift das Speicherlimit (siehe unten).
7.  Erst dann ertönt der Erfolgston und das Ereignis `SaveCompleted` geht an verbundene Oberflächen. Bei einem Fehler (Platte voll, Ordner fehlt, Puffer leer, Encoder-Fehler) werden die `.part`-Dateien gelöscht, der Fehlerton ertönt und `SaveFailed{grund}` wird gesendet.

### Speicherlimit und Löschen

*   Je Ordner (Clip-Ordner, Export-Ordner) gilt ein Limit, Standard 10 GB.
*   **Eigene Datei:** Eine Datei gilt nur dann als GameClip-eigen, wenn sie im Index mit Status `ok` steht **und** ihr Metadaten-Tag `gameclip_id` mit dem Index übereinstimmt. Vor dem Löschen wird der Tag erneut gelesen. Fremde Dateien zählen weder zum Limit noch werden sie gelöscht (Q13).
*   **Nach jedem neuen Clip:** Liegt die Summe der eigenen, nicht geschützten Dateien plus der geschützten über dem Limit, werden die ältesten nicht geschützten Clips endgültig gelöscht (`DeleteFileW`, kein Papierkorb), bis die Summe passt. Der gerade geschriebene Clip wird nie gelöscht.
*   Zu jedem gelöschten Clip im Clip-Ordner wird auch seine `.m4a` gelöscht.
*   Reicht das nicht, weil alles geschützt ist oder ein Clip allein größer ist als das Limit, wird nichts gelöscht. Es erscheint eine Warnung im Tray und in der Galerie.
*   Sind Clip- und Export-Ordner derselbe Ordner, gilt ein gemeinsames Limit, und die Einstellungen weisen darauf hin.
*   **Verwaiste Einzelspuren:** Fehlt die zugehörige MP4 (z. B. im Explorer gelöscht), werden die `.m4a` beim nächsten Abgleich gelöscht (Q12). Einzelspuren zählen nicht zum Ordner-Limit, weil sie im App-Ordner liegen. Sie belegen etwa 1 MB pro Minute und Spur.

### Clip-Index (T3: SQLite)

*   **Datei und Zugriff:** `index.sqlite` mit `rusqlite` im WAL-Modus. Nur gameclipd schreibt, und zwar über einen einzigen Thread. Die Oberfläche fragt über IPC ab.
*   **Tabelle `clips`:**

    | Spalte | Bedeutung |
    | --- | --- |
    | `id` | UUID, gleich dem Metadaten-Tag `gameclip_id` |
    | `folder` | `clips` oder `exports` |
    | `path` | absoluter Pfad |
    | `size_bytes` | Dateigröße |
    | `created_at` | Zeitpunkt der Erstellung |
    | `duration_ms` | Dauer |
    | `starred` | Stern ja/nein |
    | `tracks_path` | Pfad der `.m4a`, nur bei Hotkey-Clips |
    | `source_id` | bei Exporten die ID des Original-Clips |
    | `status` | `ok`, `missing` oder `foreign` |

*   **Tabelle `meta`:** `schema_version` und Migrationen. Vor jeder Migration wird die Datei kopiert.
*   **Abgleich mit dem Dateisystem:**
    *   Beim Start und bei Änderungen, die `ReadDirectoryChangesW` für Clip- und Export-Ordner meldet.
    *   Fehlt eine Datei, wird sie `missing`. Ist sie nach 24 h nicht zurück, verschwindet der Eintrag, und die zugehörige `.m4a` wird gelöscht.
    *   Umbenennen oder Verschieben innerhalb eines überwachten Ordners: Der Dienst findet die Datei über ihren Tag und aktualisiert den Pfad.
*   **Unbekannte Clips:** Trägt eine MP4 einen `gameclip_id`-Tag, steht aber nicht im Index (z. B. nach einer Neuinstallation), wird sie `foreign`. Sie zählt nicht zum Limit und wird nie gelöscht. Die Galerie bietet „In GameClip übernehmen“ an.
*   **Doppelte IDs:** Wird eine Datei kopiert, gibt es zwei Dateien mit derselben ID. Die Datei, deren Pfad im Index steht, bleibt der Eintrag. Die Kopie wird `foreign`.
*   **Beschädigung:** Lässt sich die Datenbank nicht öffnen, wird sie umbenannt (`index.sqlite.broken`) und aus den Metadaten-Tags neu aufgebaut. Alle Clips gelten dann als `foreign`, bis der Nutzer sie übernimmt. Das automatische Löschen ist so lange ausgesetzt, und die Galerie zeigt einen Hinweis. Sterne gehen dabei verloren.

### Kürzen und Export

*   Die Oberfläche sendet `StartExportJob{clip_id, in_pts, out_pts, profil}` mit diesen Profilen:
    *   `Mischung` (Standard)
    *   `GetrennteSpuren`
    *   `Zielgröße{bytes}` (Standard 10 MB)
*   Der Dienst reiht den Auftrag ein, höchstens einer läuft gleichzeitig. Er startet `gameclip-worker.exe` mit Priorität `BELOW_NORMAL` und übergibt nur Dateipfade, die er selbst bestimmt.
*   **Vertrag mit dem Worker (T4):**
    *   **Eingabe:** Der Auftrag kommt als JSON über stdin, z. B. `{quelle, einzelspuren_pfad, ziel_part_pfad, in_pts, out_pts, profil, encoder_reihenfolge, adapter_luid}`.
    *   **Ausgabe:** Eine JSON-Zeile pro Meldung über stdout:
        *   `{"progress": 0.42}`
        *   `{"encoder": "h264_nvenc"}`
        *   `{"done": {"bytes": …, "dauer_ms": …}}`
        *   `{"error": {"code": "…", "message": "…"}}`
    *   **Exit-Codes:** 0 bei Erfolg, 1 bei Fehler (Details in der letzten Meldung). Jeder andere Code gilt als Absturz.
    *   **Bibliotheken:** Der Worker lädt dieselben FFmpeg-DLLs wie der Dienst. Die Encoder-Prüfung teilt er sich mit dem Dienst über das Crate `gameclip-media`.
    *   **Zeitlimit:** Der Dienst beendet den Worker, wenn 30 s lang keine Meldung kommt.
*   **Ablauf im Worker:**
    1.  Quelle mit Hardware-Decoder öffnen (`d3d11va`), sonst Software-Decoder.
    2.  Zum Keyframe vor `in_pts` springen und bis `in_pts` dekodieren, ohne Ausgabe.
    3.  Ab `in_pts` bis `out_pts` kodieren, bei `Zielgröße` vorher auf der GPU skalieren.
    4.  Audio aus der Mischspur bzw. der `.m4a` sample-genau zuschneiden und neu kodieren.
    5.  Muxen mit `+faststart` in `ziel_part_pfad`, dann `fsync`. Das Umbenennen übernimmt der Dienst.
    6.  Bei `Zielgröße`: Größe prüfen. Ist die Datei zu groß, einmal mit 10 % weniger Bitrate wiederholen.
*   **Smart-Cut (Q7)** wird als zweiter Modus im Worker umgesetzt, wenn S9 es rechtfertigt. Dabei wird nur von `in_pts` bis zum nächsten Keyframe neu kodiert (am Ende ebenso), der Rest wird kopiert.
*   **Video:**
    *   Der Clip wird mit Hardware-Decoder und Hardware-Encoder neu kodiert, bildgenau ab `in_pts` (Smart-Cut als spätere Option, Q7).
    *   Lässt sich keine zweite Hardware-Encoder-Sitzung öffnen, nimmt der Auftrag `libx264` mit `veryfast` und meldet „Export über CPU“.
*   **Audio:**
    *   `Mischung` und `Zielgröße` nehmen die Spur „Mischung“ der MP4.
    *   `GetrennteSpuren` schreibt zuerst „Mischung“ als Standardspur, danach „System“, „Mikrofon“ und „Discord“ aus der `.m4a`.
    *   Audio wird immer sample-genau neu kodiert.
*   **Zielgröße:**
    *   Videobitrate = `(Ziel × 8 × 0,92 − Audiobitrate × Dauer − Container-Reserve) / Dauer`, mit `maxrate`.
    *   Ist die Bitrate zu niedrig für die Auflösung, wird stufenweise verkleinert: 1080p → 720p → 480p, ab 60 s auch 30 fps.
    *   Ton: nur die Mischung mit 96 kbit/s.
    *   Danach wird die Dateigröße geprüft. Ist sie zu groß, folgt ein zweiter Versuch mit 10 % weniger Bitrate.
*   **Ausgabe:** `<Export-Ordner>\<Originalname>_trim.mp4` bzw. `_10MB.mp4`, mit `+faststart` (moov am Anfang, für Messenger). Neue `gameclip_id`, Eintrag im Index, dann greift das Speicherlimit des Export-Ordners. Das Original bleibt unverändert.
*   **Rückmeldung:** Ereignisse `JobProgress{prozent}`, `JobCompleted{clip_id}` und `JobFailed{grund}`. `CancelJob` beendet den Arbeitsprozess und löscht die `.part`-Datei.

### Hotkey

*   Ein eigener Thread mit einem Message-only-Fenster ruft `RegisterHotKey` mit `MOD_NOREPEAT` auf.
*   Erlaubt sind F1–F24 (außer F12) sowie Kombinationen mit Strg, Alt und Shift. Kombinationen mit der Windows-Taste sind gesperrt.
*   `SuspendHotkey` gibt die Taste frei, solange der Auswahldialog offen ist. `TestRegister{taste}` prüft, ob die neue Taste frei ist.
*   Schlägt die Registrierung fehl, wechselt der Dienst nach „Eingeschränkt: Hotkey nicht aktiv“ und meldet den Fehlercode.

### Dienst-Schnittstelle (IPC)

*   **Transport:** Named Pipe `\\.\pipe\<AppID>-<SID>`. Ihre Sicherheitsbeschreibung erlaubt nur die SID des aktuellen Benutzers. Ein benannter Mutex `Local\<AppID>` stellt sicher, dass nur eine Instanz läuft.
*   **Nachrichtenformat (T2):** Jede Nachricht beginnt mit 4 Bytes Länge (u32, Little Endian), danach folgt UTF-8-JSON.
    *   Höchstgröße einer Nachricht: 8 MB. Größere Nachrichten werden abgelehnt und die Verbindung wird getrennt.
    *   Hülle:
        *   Anfrage: `{"id": <u64>, "type": "<Befehl>", "body": {…}}`
        *   Antwort: `{"id": <u64>, "ok": true, "body": {…}}` oder `{"id": <u64>, "ok": false, "error": {"code": "<Code>", "message": "…"}}`
        *   Ereignis: `{"event": "<Name>", "body": {…}}`
    *   Felder, die der Empfänger nicht kennt, ignoriert er. Neue Felder erhöhen deshalb nur `protokoll_minor`.
    *   Fehlercodes: `NotReady`, `InvalidArgument`, `NotFound`, `Busy`, `HotkeyTaken`, `DiskFull`, `EncoderFailed`, `VersionMismatch`, `Internal`.
*   **Handshake:** `Hello{protokoll_major, protokoll_minor}`. Weicht die Hauptversion ab, zeigt die Oberfläche „Bitte GameClip neu starten“ und sendet nichts weiter.
*   **Befehle:**
    *   `GetStatus`, `GetSettings`, `SetSettings{teil}`, `GetForecast{einstellungen}`, `ListDevices`, `GetCapabilities`
    *   `SaveClip`, `PauseMic{an}`, `SuspendHotkey{an}`, `TestRegister{taste}`
    *   `ListClips{ordner}`, `SetStar{clip_id, an}`, `DeleteClip{clip_id}`, `GetFrameTimes{clip_id}`
    *   `StartExportJob{…}`, `CancelJob{job_id}`, `CopyDiagnostics`
*   **Ereignisse:** `StateChanged`, `SaveCompleted`, `SaveFailed`, `NewClip`, `ClipDeleted`, `FormatChanged`, `JobProgress`, `JobCompleted`, `JobFailed`, `StorageWarning`.
*   **Fähigkeiten:** `GetCapabilities` meldet `{einzelspuren, max_puffer_s, prognose, encoder, hdr}`. Für den späteren Linux-Adapter (gpu-screen-recorder) sind alle Statusfelder optional.
*   **Sicherheit:** Befehle enthalten nie freie Dateipfade, nur `clip_id`s. Speicherordner werden nur über `SetSettings` geändert, und der Dienst prüft sie (existiert, beschreibbar, kein Systemordner).

### Zustandsmaschine

| Zustand | Ereignis | Neuer Zustand | Wirkung |
| --- | --- | --- | --- |
| Leerlauf | Start | Startet | Geräte, Encoder und Hotkey anlegen |
| Startet | alles bereit | Puffert | Tray „Puffert“ |
| Startet | Hotkey fehlgeschlagen | Eingeschränkt | Tray-Warnung, Fehlerton |
| Startet | kein Encoder außer x264 | Eingeschränkt | Warnung „CPU-Encoder“ |
| Puffert / Eingeschränkt | Hotkey | Speichert (parallel zum Puffern) | siehe „Speichern per Hotkey“ |
| Speichert | Hotkey | Speichert | höchstens ein weiterer Vorgang wird eingereiht |
| Speichert | fertig | vorheriger Zustand | Erfolgston, `SaveCompleted` |
| Speichert | Fehler | vorheriger Zustand | Fehlerton, `SaveFailed` |
| Puffert | Einstellung geändert | Konfiguriert neu | Puffer neu anlegen |
| Konfiguriert neu | fertig | Puffert | Tray „Puffert“ |
| Puffert | Geräteverlust | Erholt sich | Neuaufbau mit Wartezeit 1, 2, 4, 8 s |
| Erholt sich | Erfolg | Puffert | neuer Abschnitt |
| Erholt sich | 5 Fehlversuche | Fehler | Tray-Fehler, Benachrichtigung |
| Startet / Konfiguriert neu / Erholt sich | Hotkey | unverändert | Fehlerton „Puffer nicht bereit“ |
| beliebig | Mikrofon pausiert | unverändert | Mikrofon-Quelle liefert Stille, Tray-Symbol |

Exportaufträge sind unabhängig von der Zustandsmaschine. Sie laufen in jedem Zustand außer „Fehler“.

### Speicherorte

| Was | Ort |
| --- | --- |
| Einstellungen | `%APPDATA%\GameClip\settings.toml` mit `schema_version` |
| Clip-Index | `%LOCALAPPDATA%\GameClip\index.sqlite` |
| Einzelspuren | `%LOCALAPPDATA%\GameClip\tracks\<UUID>.m4a` |
| Logs | `%LOCALAPPDATA%\GameClip\logs\` (rotierend, 5 × 10 MB) |
| Clips | vom Nutzer gewählter Clip-Ordner (Standard `%USERPROFILE%\Videos\GameClip`) |
| Exporte | vom Nutzer gewählter Export-Ordner (Standard `%USERPROFILE%\Videos\GameClip\Export`) |

*   **Einstellungen:**
    *   Migrationen laufen nur nach vorn, vor jeder Migration wird `settings.toml.bak` angelegt.
    *   Unbekannte Felder bleiben erhalten.
    *   Ist `schema_version` neuer als die App, liest der Dienst die Datei nur und überschreibt sie nicht.
*   **Standardordner:** Clip- und Export-Ordner sind standardmäßig zwei verschiedene Ordner, damit jeder sein eigenes Limit hat.

### Oberfläche

*   Tauri 2 mit WebView2. Die Oberfläche spricht nur über die Named Pipe mit dem Dienst und lädt keine FFmpeg-Bibliothek.
*   **Frontend (T5): React mit TypeScript, gebaut mit Vite.**
    *   **Kein Router.** Es gibt drei Ansichten (Galerie, Editor, Einstellungen) und den Ablauf für den ersten Start. Gewechselt wird über einen einfachen Zustand.
    *   **IPC-Anbindung.** Die Rust-Seite von Tauri hält die Pipe-Verbindung und leitet Ereignisse als Tauri-Events weiter. Im Frontend kapselt ein Modul `ipc.ts` die Befehle als typisierte Funktionen. Ein kleiner Store auf Basis von `useSyncExternalStore` hält Status und Clipliste, es gibt keine zusätzliche State-Bibliothek. Die TypeScript-Typen werden aus `gameclip-protocol` erzeugt (`ts-rs`), damit beide Seiten dieselben Felder kennen.
    *   **Trim-Leiste ohne Neu-Rendern.** Die Wiedergabeposition läuft über `requestVideoFrameCallback` und `ref`s direkt ins DOM. React rendert nur neu, wenn Start- oder Endpunkt geändert werden.
    *   **Gestaltung.** Eigene CSS-Variablen für Farben, Abstände und Schrift (helles und dunkles Thema nach Windows-Einstellung), CSS Modules je Komponente, keine UI-Bibliothek. Ziel ist eine ruhige, übersichtliche Oberfläche mit wenigen Bedienelementen pro Ansicht.
*   **Galerie:** Zwei Reiter, „Clips“ und „Exporte“. Vorschaubilder kommen von der Windows-Shell (`IShellItemImageFactory`). Angezeigt werden Stern, Dauer, Größe und belegter Platz im Verhältnis zum Limit.
*   **Player und Kürzen:** `<video>` lädt die Datei über ein eigenes Tauri-Protokoll `clip://<clip_id>`, das HTTP-Range-Anfragen unterstützt. Für bildgenaues Setzen holt die Oberfläche mit `GetFrameTimes` die Zeitstempel aller Bilder und rastet Start und Ende auf diese ein. `requestVideoFrameCallback` zeigt das aktuelle Bild an. Pfeiltasten springen ein Bild weiter oder zurück.
*   **Export-Dialog:** Wahl zwischen „Mischung“ (Standard), „Mit getrennten Spuren“ und „Zielgröße“ mit einstellbaren MB.
*   **Teilen:**
    *   Datei in die Zwischenablage als `CF_HDROP`, über `clipboard-win`.
    *   Drag-and-drop aus der Galerie über `tauri-plugin-drag`.
    *   „Im Ordner anzeigen“ über `SHOpenFolderAndSelectItems`.
*   **Erster Start:** Mikrofon (an, Gerät wählbar), dann Hotkey mit Testdruck und Warnung, dann Clip-Ordner. Danach „Jetzt Test-Clip erstellen“.
*   **Windows 11 N/KN:** Fehlt das Media Feature Pack, zeigt der erste Start, wie es installiert wird.

### Tray, Töne und Benachrichtigungen

*   **Tray:** Das Symbol zeigt Puffert, Speichert, Eingeschränkt, Fehler und Mikrofon pausiert. Menü: Clip speichern, Mikrofon pausieren, Galerie öffnen, Einstellungen, Beenden.
*   **Töne:** Zwei kurze WAV-Dateien, eine für Erfolg und eine für Fehler. Der Dienst spielt sie über WASAPI ab, die Lautstärke ist einstellbar und sie lassen sich abschalten.
*   **Benachrichtigungen:** Windows-Toasts über die AUMID des MSIX-Pakets, Text „Clip gespeichert (1:00, 142 MB)“ bzw. der Fehlergrund. Fokus-Assistent kann Toasts unterdrücken. Der Ton bleibt deshalb das Hauptsignal.

### Plattformgrenze und Crates

| Crate | Inhalt | Windows-Code erlaubt |
| --- | --- | --- |
| `gameclip-core` | Zustandsmaschine, Einstellungsschema, Prognoseformel, Speicherlimit-Logik, Dateinamen | nein |
| `gameclip-protocol` | Befehle, Ereignisse, Versionen, Rahmung (Längenpräfix + JSON) | nein |
| `gameclip-media` | FFmpeg-Anbindung über `rsmpeg`, Encoder-Prüfung und Ausweichkette, Muxer, Hilfen für Hardware-Frames; genutzt von Dienst und Worker | ja |
| `gameclip-capture-win` | WGC, D3D11, Taktgeber, WASAPI, Mixer, Ringpuffer | ja |
| `gameclipd` | Dienstprogramm: Hotkey, Tray, Töne, Index, Warteschlange, IPC-Server | ja |
| `gameclip-ui-platform` | Zwischenablage, Drag-out, Explorer, Vorschaubilder hinter einer kleinen Schnittstelle | ja (Windows-Umsetzung) |
| `gameclip-ui` | Tauri-App (Rust) und Frontend (React, TypeScript, Vite) | nein, außer über `gameclip-ui-platform` |
| `gameclip-worker` | Arbeitsprozess für Kürzen und Export | ja |

Durchgesetzt wird das so:
*   Ein CI-Job auf `ubuntu-latest` baut und testet `gameclip-core`, `gameclip-protocol` und das Frontend gegen eine Attrappe des Dienstes.
*   `cargo deny` bricht ab, wenn `windows` oder `windows-sys` in `gameclip-core` oder `gameclip-protocol` auftaucht.

### Diagnose

*   **Logs:** `tracing` mit rotierenden Dateien.
*   **Je Speichervorgang:** Dauer, Bytes, Anzahl der Bildgruppen, Encoder, Ergebnis.
*   **Je Minute:** aufgenommene, wiederholte und verworfene Bilder, Länge der Encoder-Warteschlange, Puffergröße im Vergleich zur Prognose, Audio-Drift und aufgefüllte Stille je Quelle.
*   **„Diagnose kopieren“:** Windows-Build, GPU und Treiber, Ergebnis der Encoder-Prüfung, Audiogeräte, die letzten 200 Logzeilen. Benutzerpfade werden unkenntlich gemacht. Nichts davon wird gesendet.

### Abhängigkeiten und Lizenz

*   **FFmpeg:** fest versionierter GPL-Shared-Build (BtbN, Version 7.1 oder neuer), mitgeliefert als DLLs, mit Quellcode-Angebot. Ohne `--enable-nonfree`, ohne fdk-aac. Anbindung über `rsmpeg`.
*   **Rust-Bibliotheken (Lizenz und Pflege vor M1 prüfen):** `windows`, `windows-capture` (als Vorlage oder direkt), `wasapi`, `tray-icon`, `clipboard-win`, `tauri-plugin-drag`, `tracing`, `serde`, `toml`, `uuid`.
*   **Lizenz des Projekts:** GPL-3.0-or-later.
*   **Signatur:** über die SignPath Foundation (MSIX muss signiert sein, prüfen ob SignPath MSIX unterstützt). Für Entwicklungs-Builds ein selbst erzeugtes Testzertifikat.
*   **Updates:** über winget. Ein Tauri-Updater ist nicht eingebaut.

### Paket und Installation (T1: volles MSIX)

*   **Inhalt des Pakets:** `gameclipd.exe`, `gameclip-ui.exe`, `gameclip-worker.exe`, FFmpeg-DLLs, Töne und Lizenztexte. Beide Programme sind als Anwendungen mit eigener ID im Manifest eingetragen. Nur `gameclip-ui` erscheint im Startmenü.
*   **Fähigkeiten im Manifest:** `runFullTrust` (Win32-Desktop-App), `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` (prüfen), `microphone`.
*   **Autostart:** `windows.startupTask` für `gameclipd`. Der Nutzer kann ihn in den Windows-Einstellungen und in GameClip abschalten.
*   **App-ID:** `io.github.poldion.GameClip`. Die AUMIDs lauten `<PackageFamilyName>!GameClip` (Oberfläche) und `<PackageFamilyName>!GameClipService` (Dienst). Pipe-Name und Mutex leiten sich aus der App-ID ab. Die App-ID wird nie geändert.
*   **Gebaut wird außerhalb von Tauri.** Tauri baut nur die Programme. Ein eigenes Skript `packaging/build-msix.ps1` erzeugt `AppxManifest.xml` aus einer Vorlage und packt mit `MakeAppx.exe`. Signiert wird mit `SignTool.exe`.
*   **Virtualisierung:** Schreibzugriffe auf `%APPDATA%` und `%LOCALAPPDATA%` leitet Windows bei MSIX-Apps in einen paketeigenen Ordner um. Einstellungen, Index, Einzelspuren und Logs landen dort, und die Deinstallation entfernt sie. Clip- und Export-Ordner unter `Videos` sind nicht betroffen, Clips bleiben also nach der Deinstallation erhalten. „Diagnose kopieren“ zeigt den tatsächlichen Pfad.
*   **Risiko:** In S1 muss geprüft werden, ob sich im Paket Autostart, Named Pipe, WASAPI-Prozess-Loopback und die randlose Aufnahme wie erwartet verhalten. S1 läuft deshalb von Anfang an als MSIX.

## Alternatives Considered

### Produkt und Umfang

#### Alternative A: Windows und Ubuntu gleichzeitig in Version 1

*   **Pros**: Keine spätere Portierung; die gemeinsame Schicht wird von Anfang an an zwei Systemen geprüft.
*   **Cons**: Zwei Aufnahmekerne (eigener Windows-Kern, gpu-screen-recorder unter Linux) gleichzeitig vor dem Vanguard-Test. Unter Linux kommen Wayland-Freigaben, Hotkey-Portale, fehlende Trays und NVIDIA ohne VA-API hinzu.
*   **Reason for Rejection**: Rejected by author: Mit gpu-screen-recorder unter Linux betrifft eine spätere Portierung nur Oberfläche, Einstellungen und Protokoll. Diese Teile bleiben per CI frei von Windows-Code. Zwei Plattformen vor dem Vanguard-Nachweis wären das größere Risiko.

#### Alternative B: Eigener Linux-Aufnahmekern

*   **Pros**: Gleiches Puffer- und Spurmodell auf beiden Systemen.
*   **Cons**: PipeWire, Portal, DMA-BUF, VA-API und NVENC unter Linux selbst bauen bedeutet Jahre an Treiber-Sonderfällen, die gpu-screen-recorder bereits gelöst hat.
*   **Reason for Rejection**: Rejected by author: gpu-screen-recorder (GPL-3) deckt Replay-Puffer, Wayland, X11, VA-API und NVENC ab und passt zur Projektlizenz.

#### Alternative C: Windows 10 unterstützen

*   **Pros**: Mehr mögliche Nutzer.
*   **Cons**: Der gelbe Aufnahmerahmen lässt sich nicht abschalten, für Prozess-Loopback sind zusätzliche Versionsprüfungen nötig, und es gibt eine zweite Testplattform.
*   **Reason for Rejection**: Rejected by author: Der Support für Windows 10 endete im Oktober 2025.

#### Alternative D: Erste Version nur mit Tray, ohne Galerie und Kürzen

*   **Pros**: Etwa halber Umfang, schnellere erste Version.
*   **Cons**: Der Ablauf Kürzen → Discord-Export → Zwischenablage ist der wahrscheinliche Vorteil gegenüber Game Bar (G1) und fehlte dann.
*   **Reason for Rejection**: Rejected by author: Galerie, Kürzen und „Im Ordner anzeigen“ gehören zum Kern des Produkts.

#### Alternative E: Overlay im Spiel

*   **Pros**: Sichtbare Bestätigung direkt im Spiel.
*   **Cons**: Braucht Code-Injektion oder Grafik-Hooks; Vanguard blockiert oder meldet das.
*   **Reason for Rejection**: Rejected by author: Verletzt die Regel „keine Injektion“ (G4). Ton und Toast ersetzen das Overlay.

#### Alternative F: Cloud-Upload zum Teilen

*   **Pros**: Teilen per Link, keine Größenlimits von Messengern.
*   **Cons**: Braucht Server, Konten und Internet.
*   **Reason for Rejection**: Rejected by author: GameClip bleibt offline und ohne Konto. Geteilt wird über Zwischenablage und Drag-and-drop.

#### Alternative G: Steam OS in Version 1

*   **Pros**: Steam-Deck-Nutzer als Zielgruppe.
*   **Cons**: Im Spielmodus kontrolliert Steam die Eingaben, globale Hotkeys funktionieren dort kaum, und Steam hat eine eigene Hintergrundaufnahme.
*   **Reason for Rejection**: Rejected by author: zurückgestellt, kein Teil von v1.

### Aufnahme und Encoding

#### Alternative H: Aufnahme per Code-Injektion („Game Capture“)

*   **Pros**: Funktioniert auch bei exklusivem Vollbild, etwas geringere Latenz.
*   **Cons**: Lädt Code in den Spielprozess, Anti-Cheat erkennt und blockiert das.
*   **Reason for Rejection**: Rejected by author: Vanguard-Verträglichkeit ist Pflicht.

#### Alternative I: Puffer aus Rohbildern

*   **Pros**: Kein Encoder im Hintergrund nötig.
*   **Cons**: 1080p60 in NV12 sind ~186 MB pro Sekunde, eine Minute also über 11 GB.
*   **Reason for Rejection**: Rejected by author nach Review: passt nicht in den RAM.

#### Alternative J: Ringpuffer als Datei auf der SSD

*   **Pros**: Beliebig lange Puffer ohne RAM-Kosten.
*   **Cons**: Ständige Schreiblast auf der SSD, mehr Fehlerfälle (Platte voll, langsame Datenträger).
*   **Reason for Rejection**: Rejected by author: zurückgestellt; die Länge wird über die RAM-Prognose und Warnungen begrenzt.

#### Alternative K: Fehler statt CPU-Encoder, wenn keine Hardware-Unterstützung da ist

*   **Pros**: Kein unerwarteter CPU-Verbrauch im Spiel.
*   **Cons**: Ohne passende GPU funktioniert die App gar nicht.
*   **Reason for Rejection**: Rejected by author: Ausweichen auf x264 mit deutlicher Warnung.

#### Alternative L: Verlustfreies Kürzen nur an Keyframes

*   **Pros**: Sehr schnell, kein Qualitätsverlust, kein Encoder nötig.
*   **Cons**: Schnitt nur auf 1 bis 2 Sekunden genau.
*   **Reason for Rejection**: Rejected by author: Kürzen soll bildgenau sein. Smart-Cut bleibt als spätere Beschleunigung (Q7).

### Audio

#### Alternative M: Nur eine gemischte Spur, keine Einzelspuren

*   **Pros**: Einfacherer Puffer und Muxer, weniger Speicher.
*   **Cons**: Mikrofon und System lassen sich später nicht getrennt bearbeiten.
*   **Reason for Rejection**: Rejected by author: Einzelspuren werden immer aufgenommen und können manuell exportiert werden.

#### Alternative N: Einzelspuren nur über eine Aufnahme-Einstellung

*   **Pros**: Etwas weniger RAM, wenn die Einstellung aus ist.
*   **Cons**: Wer die Einstellung vergisst, kann später nicht mehr getrennt exportieren.
*   **Reason for Rejection**: Rejected by author: Einzelspuren sind immer verfügbar; die Wahl fällt beim Export.

#### Alternative O: Einzelspuren im Original-Clip statt intern

*   **Pros**: Alles in einer Datei, keine interne Zuordnung nötig.
*   **Cons**: Dateien im Clip-Ordner hätten mehrere Spuren und wären größer.
*   **Reason for Rejection**: Rejected by author: Clip- und Export-Ordner enthalten immer nur die Mischung. Die Einzelspuren liegen als `.m4a` im App-Ordner.

#### Alternative P: Mischregler im Editor

*   **Pros**: Lautstärke pro Clip nachträglich anpassen.
*   **Cons**: Braucht eine eigene Vorschau-Mischung, weil WebView2 nur eine Spur abspielt.
*   **Reason for Rejection**: Rejected by author: geht über das Ziel hinaus. Lautstärkeregler gibt es nur in den Einstellungen und nur für die Mischung.

### Bedienung

#### Alternative Q: Standard-Hotkey mit Modifier (z. B. Alt+F8)

*   **Pros**: Geringere Gefahr, eine Spieltaste zu belegen.
*   **Cons**: Umständlicher im Spiel.
*   **Reason for Rejection**: Rejected by author: F8 bleibt Standard. Der Hotkey ist frei belegbar, und die Warnung im Auswahldialog deckt das Risiko ab.

#### Alternative R: Tastatur-Hooks (`WH_KEYBOARD_LL`)

*   **Pros**: Die Taste erreicht weiterhin das Spiel.
*   **Cons**: Wirkt auf Anti-Cheat wie ein Keylogger und empfängt nichts, wenn ein Spiel mit Administratorrechten den Fokus hat.
*   **Reason for Rejection**: Rejected by author nach Review: RegisterHotKey wird genutzt.

#### Alternative S: Automatisches Löschen in den Papierkorb

*   **Pros**: Versehentlich gelöschte Clips lassen sich wiederherstellen.
*   **Cons**: Der Papierkorb liegt auf derselben Platte, der Platz wird nicht frei, und das Limit wirkt nicht.
*   **Reason for Rejection**: Rejected by author: Endgültiges Löschen, dafür Schutz per Stern.

### Architektur (Entscheidungen T1–T5)

#### Alternative T: Electron statt Tauri

*   **Pros**: Mitgeliefertes Chromium, auf allen Systemen gleiches Verhalten.
*   **Cons**: Etwa 100 bis 200 MB mehr RAM und ein deutlich größeres Paket.
*   **Reason for Rejection**: Rejected by author: Tauri nutzt das vorhandene WebView2.

#### Alternative U: Ein einziger Prozess für Dienst und Oberfläche

*   **Pros**: Keine IPC, weniger Code.
*   **Cons**: Ein Hänger oder Absturz der WebView reißt Aufnahme und Puffer mit. WebView2-Prozesse bleiben im Speicher, auch wenn das Fenster zu ist.
*   **Reason for Rejection**: Rejected by author nach Review: zwei Programme, gameclipd und gameclip-ui.

#### Alternative V: NSIS/MSI über Tauri oder MSI plus Sparse Package (T1)

*   **Pros**: NSIS/MSI ist der Standardweg von Tauri mit dem geringsten Aufwand. Das Sparse Package gibt eine App-Identität bei klassischer Installation.
*   **Cons**: NSIS/MSI hat keine Paket-Identität, ob randlose Aufnahme ohne Rückfrage geht, ist ungeprüft. Benachrichtigungen brauchen eine Verknüpfung mit AUMID. Das Sparse Package ist wenig dokumentiert und schwer zu debuggen.
*   **Reason for Rejection**: Rejected by author: Volles MSIX gibt eine saubere Identität, deklarierbare Fähigkeiten und Benachrichtigungen ohne Verknüpfungstricks. Der Preis sind ein eigener Build-Schritt, Pflicht zur Signatur und ein virtualisierter App-Ordner.

#### Alternative W: JSON pro Zeile oder MessagePack auf der Pipe (T2)

*   **Pros**: JSON pro Zeile lässt sich von Hand testen. MessagePack ist kompakt und schnell.
*   **Cons**: JSON pro Zeile verlässt sich auf Zeilenumbrüche als Grenze. MessagePack ist ohne Hilfswerkzeug nicht lesbar.
*   **Reason for Rejection**: Rejected by author: Längenpräfix plus JSON ist lesbar, hat eindeutige Nachrichtengrenzen und eine prüfbare Höchstgröße.

#### Alternative X: Index nur in Metadaten-Tags oder als JSON-Datei (T3)

*   **Pros**: Tags überleben eine Neuinstallation. Eine JSON-Datei braucht keine Datenbank.
*   **Cons**: Um einen Stern per Tag zu setzen, muss die Videodatei umgeschrieben werden, und kopierte Tags machen fremde Dateien zu „eigenen“. Die JSON-Datei wird bei jeder Änderung komplett neu geschrieben.
*   **Reason for Rejection**: Rejected by author: SQLite mit atomaren Schreibvorgängen und schnellen Abfragen. Der Preis: Sterne gehen bei Neuinstallation oder Beschädigung verloren.

#### Alternative Y: Mitgelieferte `ffmpeg.exe` als Arbeitsprozess (T4)

*   **Pros**: Wenig eigener Code, FFmpeg kann alles Nötige bereits.
*   **Cons**: Weniger Kontrolle über Encoder-Auswahl und Fehlercodes, Smart-Cut ist schwer umsetzbar, Fehler kommen als Text.
*   **Reason for Rejection**: Rejected by author: Eigenes Programm `gameclip-worker` mit denselben Encoder-Regeln wie der Dienst. Der Preis ist deutlich mehr eigener Code in M3.

#### Alternative Z: Svelte oder SolidJS (T5)

*   **Pros**: Kleineres Bundle, weniger Neu-Rendern bei sich ständig ändernden Werten.
*   **Cons**: Kleineres Ökosystem, weniger fertige Bausteine, weniger mögliche Mitwirkende.
*   **Reason for Rejection**: Rejected by author: React wegen Ökosystem und Bekanntheit. Das Risiko des Neu-Renderns in der Trim-Leiste wird durch direkte DOM-Updates abgefangen.

--------------------------------------------------------------------------------

## Non-Functional Requirements

*   **Sicherheit:**
    *   Die Named Pipe ist nur für die SID des aktuellen Benutzers offen.
    *   Befehle enthalten keine freien Pfade. Speicherordner prüft der Dienst (existiert, beschreibbar, kein Windows-, Programm- oder Systemordner).
    *   Der Worker bekommt nur Pfade vom Dienst.
    *   Gelöscht wird nur nach doppelter Eigentumsprüfung (Index und Tag).
    *   Alle Programmdateien und DLLs sind signiert.
*   **Datenschutz:**
    *   Keine Netzwerkzugriffe. Das Manifest deklariert keine `internetClient`-Fähigkeit.
    *   Das Mikrofon ist standardmäßig an. Der erste Start sagt das deutlich, das Tray zeigt den Mikrofonstatus, und eine Pause ist jederzeit möglich.
    *   „Diagnose kopieren“ macht Benutzerpfade unkenntlich.
*   **Leistung:** Ziele G2 und G3. Gemessen wird mit PresentMon (Bildverluste im Spiel), `typeperf` (CPU und Working Set des Dienstes) und den Speicherzeiten aus dem eigenen Log.
*   **Einführung und Rückweg:**
    *   Verteilung über winget und GitHub Releases.
    *   Ein Rückweg ist das Installieren der vorherigen MSIX-Version. Einstellungen mit neuerem Schema werden dabei nur gelesen, nicht überschrieben.
    *   Die Datenbank wird vor jeder Migration gesichert.
    *   Clips bleiben bei Deinstallation und Downgrade erhalten.

--------------------------------------------------------------------------------

### Future Improvements

*   **Linux-Version** mit gpu-screen-recorder (Anhang A des Briefs).
*   **Smart-Cut** beim Kürzen (Q7).
*   **Nur den Ton des Spiels aufnehmen** (eigener Vanguard-Test nötig).
*   **HDR-Aufnahme** mit Tone-Mapping auf der GPU.
*   **Öffentliche Kommandozeile** für Stream Deck und Skripte.
*   **Ringpuffer auf der SSD** für Puffer über 5 Minuten.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

### Festgelegte Standardwerte

Diese Werte stehen als Konstanten in `gameclip-core/src/settings.rs`. Die mit *vorläufig* markierten werden nach S1 auf dem Referenzrechner bestätigt (Q1).

| Einstellung | Standard |
| --- | --- |
| Pufferlänge | 60 s (erlaubt 15–300 s) |
| Ausgabeauflösung | 1080p (wählbar: 720p, 1080p, 1440p, 2160p) |
| Bildrate | 60 fps (wählbar: 30, 60) |
| Qualitätsstufen 1080p60 | Niedrig 8 · **Mittel 14** · Hoch 24 Mbit/s *(vorläufig)* |
| Qualitätsstufen 1440p60 | Niedrig 12 · Mittel 20 · Hoch 32 Mbit/s *(vorläufig)* |
| Qualitätsstufen 2160p60 | Niedrig 25 · Mittel 40 · Hoch 60 Mbit/s *(vorläufig)* |
| 720p / 30 fps | 60 % bzw. 60 % der 1080p60-Werte *(vorläufig)* |
| Keyframe-Abstand | 1 s |
| Audio | Einzelspuren AAC-LC 128 kbit/s, Mischung 160 kbit/s, 48 kHz Stereo |
| Hotkey | F8 |
| Mikrofon | an, Windows-Standardgerät |
| Discord-Spur | aus |
| Lautstärken | Mikrofon, System, Discord je 0 dB (erlaubt −30 bis +12 dB) |
| Töne | an, Lautstärke 70 % |
| Mauszeiger | aufnehmen |
| Monitor | Hauptmonitor |
| Clip-Ordner | `%USERPROFILE%\Videos\GameClip` |
| Export-Ordner | `%USERPROFILE%\Videos\GameClip\Export` |
| Speicherlimit | je Ordner 10 GB (erlaubt 1–2000 GB) |
| Zielgröße | 10 MB |
| Autostart | an |
| RAM-Warnung | gelb ab 10 % des physischen RAM, abgelehnt ab 25 % |
| Dienst-RAM ohne Puffer | Ziel ≤ 80 MB *(vorläufig)* |
| Mindest-Build | Windows 11 24H2, Build 26100 *(vorläufig, Q3)* |
| App-ID / MSIX-Identity-Name | `io.github.poldion.GameClip` |
| Pipe-Name | `\\.\pipe\io.github.poldion.GameClip-<SID>` |
| Mutex | `Local\io.github.poldion.GameClip` |
| Repository | `https://github.com/Poldion/GameClip` |

### Reihenfolge der Umsetzung

1.  **M0 – Prototypen:**
    *   Workspace-Gerüst, `gameclip-core` und `gameclip-protocol` ohne Windows-Code.
    *   `gameclip-media` und `gameclip-capture-win` so weit, dass S1 läuft: WGC → NVENC → Ringpuffer → MP4 per F8, im MSIX-Testpaket.
    *   Danach S2, S4, S8, S9, der Vanguard-Test (48 h) und der Vergleich nach G1.
2.  **M1 – Dienst:** `gameclipd` mit Hotkey, Speichern, Index, Speicherlimit, Logs. Bedient wird nur über das Tray-Menü.
3.  **M2 – Oberfläche:** Einstellungen, Prognose, erster Start, Toasts, Paket mit App-ID.
4.  **M3 – Galerie und Export:** Galerie, Editor, `gameclip-worker`, Größen-Export, Zwischenablage, Drag-and-drop. Danach S5, S6 und S11.
5.  **M4 – Freigabe:** Signatur über SignPath, Hardware-Testmatrix, S7.
6.  **M5 – Öffentliche Beta:** Vanguard-Test wiederholen, winget-Manifest.

### Target Files to Create/Modify

Das Repository ist leer. Alle Dateien werden neu angelegt.

#### Repository-Wurzel

##### 1. `Cargo.toml` (Create)

*   **Type / Scope**: Cargo-Workspace.
*   **Proposed Changes**:
    *   Mitglieder: alle Crates unter `crates/` und `apps/gameclip-ui/src-tauri`.
    *   Gemeinsame Abhängigkeiten über `[workspace.dependencies]` mit festen Versionen: `windows`, `rsmpeg`, `rusqlite` (Feature `bundled`), `serde`, `serde_json`, `toml`, `uuid`, `tracing`, `tracing-appender`, `ts-rs`, `tray-icon`, `wasapi`, `clipboard-win`.
    *   `[profile.release]` mit `lto = "thin"` und `panic = "abort"` für `gameclipd`.
*   **Warum**: Eine Stelle für Versionen, damit Dienst, Worker und Oberfläche dieselben Bibliotheken nutzen.

##### 2. `rust-toolchain.toml` (Create)

*   **Type / Scope**: Werkzeugkette.
*   **Proposed Changes**: Feste stabile Rust-Version, Ziele `x86_64-pc-windows-msvc` und `x86_64-unknown-linux-gnu`, dazu `clippy` und `rustfmt`.
*   **Warum**: Reproduzierbare Builds. Das Linux-Ziel braucht die CI-Prüfung der Plattformgrenze.

##### 3. `deny.toml` (Create)

*   **Type / Scope**: `cargo-deny`-Konfiguration.
*   **Proposed Changes**:
    *   Erlaubte Lizenzen müssen mit GPL-3.0 verträglich sein: MIT, Apache-2.0, BSD-2/3, ISC, Zlib, MPL-2.0, Unicode-3.0, GPL-3.0.
    *   `bans`: `windows` und `windows-sys` sind als Abhängigkeit von `gameclip-core` und `gameclip-protocol` verboten.
*   **Warum**: Lizenzverträglichkeit sichern und die Plattformgrenze durchsetzen.

##### 4. `LICENSE` (Create)

*   **Type / Scope**: Lizenztext.
*   **Proposed Changes**: GPL-3.0-Volltext. Jede Quelldatei trägt den Kopf `SPDX-License-Identifier: GPL-3.0-or-later`.
*   **Warum**: Die Projektlizenz ist GPL-3.0-or-later (D1). Sie ist wegen x264 und FFmpeg-GPL nötig.

##### 5. `README.md` (Create)

*   **Type / Scope**: Projektbeschreibung.
*   **Proposed Changes**: Zweck, Systemvoraussetzungen (Windows 11 ab Build 26100, unterstützte GPUs), Installation über winget und MSIX, Bauanleitung, der Satz „GameClip injiziert keinen Code; getestet mit Vanguard am <Datum>“, Hinweis auf das Mikrofon und Link zu den Lizenzen.
*   **Warum**: Einstieg für Nutzer und Mitwirkende.

##### 6. `THIRD_PARTY_LICENSES.md` (Create)

*   **Type / Scope**: Lizenzhinweise.
*   **Proposed Changes**: Wird von `cargo about` erzeugt und um FFmpeg, x264 und die npm-Abhängigkeiten ergänzt. Enthält das Quellcode-Angebot für den FFmpeg-Build mit Link und Version.
*   **Warum**: Pflicht nach der GPL und den Lizenzen der Abhängigkeiten.

##### 7. `.github/workflows/ci.yml` (Create)

*   **Type / Scope**: CI.
*   **Proposed Changes**:
    *   Job `windows`: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy -- -D warnings`, `cargo deny check`, `npm ci && npm run build && npm test` in `apps/gameclip-ui`. FFmpeg holt `packaging/ffmpeg/fetch-ffmpeg.ps1`.
    *   Job `linux`: `cargo test -p gameclip-core -p gameclip-protocol -p gameclip-fake-daemon` und Frontend-Tests gegen die Attrappe.
*   **Warum**: Tests bei jedem Commit und Prüfung der Plattformgrenze.

##### 8. `.github/workflows/release.yml` (Create)

*   **Type / Scope**: Release-Pipeline.
*   **Proposed Changes**: Läuft bei einem Tag `v*`. Baut im Release-Modus, führt `packaging/build-msix.ps1` aus, signiert über SignPath und lädt `.msix` und Prüfsummen als GitHub-Release hoch.
*   **Warum**: Reproduzierbare, signierte Pakete.

#### Dokumentation

##### 9. `docs/designs/gameclip.md` (Create)

*   **Type / Scope**: Dieses Design-Dokument in der endgültigen Fassung.
*   **Warum**: Ablageort des Designs.

##### 10. `docs/protocol.md` (Create)

*   **Type / Scope**: Protokollbeschreibung.
*   **Proposed Changes**: Rahmung, Hülle, alle Befehle und Ereignisse mit Feldern, Fehlercodes und Versionsregeln. Abweichungen zum Code findet ein Test, der die Beispiele aus der Datei gegen `gameclip-protocol` parst.
*   **Warum**: Vertrag für die spätere Linux-Version und für Mitwirkende.

##### 11. `docs/test-protocols.md` (Create)

*   **Type / Scope**: Test- und Prototyp-Protokolle.
*   **Proposed Changes**:
    *   S1 bis S11 jeweils mit Aufbau, Schritten, Kriterien für bestanden/nicht bestanden und Ergebnisblock (Datum, Build, Ergebnis).
    *   Vanguard-Protokoll: separates Konto, Modi, 48 h und 14 Tage, Wiederholung vor jedem Release.
    *   G1-Vergleich mit Game Bar und OBS: Ablauf und Tabelle.
    *   Referenzrechner, sobald festgelegt.
*   **Warum**: Die Abbruchkriterien (G1, G4) müssen nachvollziehbar dokumentiert sein.

##### 12. `docs/clip-file-format.md` (Create)

*   **Type / Scope**: Dateivertrag.
*   **Proposed Changes**:
    *   MP4 im Clip-Ordner: Video H.264 High, eine Audiospur „Mischung“, Tag `gameclip_id`.
    *   `.m4a` in `tracks\`: Spuren „System“, „Mikrofon“, „Discord“ mit `handler_name` bzw. `title`.
    *   Exporte mit getrennten Spuren: Reihenfolge Mischung (Standard), System, Mikrofon, Discord.
    *   Namensschema der Dateien.
*   **Warum**: Fester Vertrag, auf den auch die Linux-Version aufbaut (D25).

#### `crates/gameclip-core` (kein Windows-Code)

##### 13. `crates/gameclip-core/Cargo.toml` (Create)

*   **Proposed Changes**: Abhängigkeiten nur `serde`, `toml`, `uuid`, `thiserror`.
*   **Warum**: Hält das Crate frei von Plattformcode.

##### 14. `crates/gameclip-core/src/lib.rs` (Create)

*   **Proposed Changes**: Exportiert die Module `state`, `settings`, `forecast`, `retention`, `naming` und `migrate`.

##### 15. `crates/gameclip-core/src/state.rs` (Create)

*   **Type / Scope**: Zustandsmaschine.
*   **Proposed Changes**:
    *   Typen `State` (Leerlauf, Startet, Puffert, Konfiguriert neu, Eingeschränkt{gründe}, Erholt sich{versuch}, Fehler) und `Event`, dazu eine reine Funktion `transition(state, event) -> (State, Vec<Effect>)`.
    *   `Effect` beschreibt Wirkungen wie Ton, Toast, Puffer neu anlegen oder Neuaufbau planen. Der Dienst führt sie aus.
    *   Ein Speichervorgang ist kein eigener Zustand, sondern ein Zähler `saves_in_flight` (0, 1 oder 2) mit Warteschlange.
    *   Umgesetzt wird genau die Übergangstabelle aus dem Technischen Plan.
*   **Warum**: Testbar ohne Hardware, und Linux kann es wiederverwenden.

##### 16. `crates/gameclip-core/src/settings.rs` (Create)

*   **Type / Scope**: Einstellungsschema.
*   **Proposed Changes**:
    *   Struktur `Settings` mit `schema_version` und allen Feldern aus „Festgelegte Standardwerte“, dazu `Default` und die Prüffunktion `validate() -> Result<(), Vec<Feldfehler>>`.
    *   Unbekannte Felder werden in `extra: toml::Table` aufbewahrt und beim Schreiben wieder ausgegeben.
*   **Warum**: Eine Quelle für Standardwerte und Prüfregeln.

##### 17. `crates/gameclip-core/src/migrate.rs` (Create)

*   **Proposed Changes**:
    *   Migrationen laufen nur nach vorn, je Version eine Funktion `vN_to_vN+1(&mut toml::Table)`.
    *   Ist `schema_version` neuer als die App, gilt `ReadOnly`.
    *   Version 1 ist die erste.
*   **Warum**: Updates und Downgrades sollen nie Einstellungen zerstören.

##### 18. `crates/gameclip-core/src/forecast.rs` (Create)

*   **Type / Scope**: RAM-Prognose.
*   **Proposed Changes**:
    *   `forecast(settings) -> Forecast{puffer_bytes, gesamt_bytes, stufe}`.
    *   Formel: `(V_max + 3 × 128k + 160k) × (T + 1 s) / 8 × 1,05`, aufgerundet auf 2-MB-Blöcke, plus Grundbedarf (Konstante, vorläufig 80 MB).
    *   `stufe` ist `Ok`, `Warnung` oder `Abgelehnt`, abhängig vom physischen RAM, der als Parameter übergeben wird.
*   **Warum**: Dienst und Oberfläche zeigen dieselbe Zahl.

##### 19. `crates/gameclip-core/src/retention.rs` (Create)

*   **Type / Scope**: Logik für das Speicherlimit, ohne Dateizugriffe.
*   **Proposed Changes**:
    *   `plan_deletions(clips: &[ClipInfo], limit_bytes, neu: ClipId) -> RetentionPlan{loeschen: Vec<ClipId>, warnung: Option<Grund>}`.
    *   Berücksichtigt werden nur `status == ok`. Sortiert wird nach `created_at`, geschützte Clips und der neue Clip werden übersprungen.
    *   Reicht Löschen nicht aus, liefert die Funktion eine leere Liste und eine Warnung.
*   **Warum**: Der riskanteste Teil (endgültiges Löschen) wird als reine Funktion vollständig getestet.

##### 20. `crates/gameclip-core/src/naming.rs` (Create)

*   **Proposed Changes**:
    *   `clip_file_name(programm: Option<&str>, zeit) -> String` im Schema `<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4`. Die Endung `.exe` wird entfernt, unerlaubte Zeichen werden zu `_`, der Name ist höchstens 64 Zeichen lang, ersatzweise heißt er `Clip`.
    *   `dedupe(name, existiert: impl Fn) -> String` hängt bei Gleichheit `_2`, `_3` usw. an.
    *   Dazu `export_file_name(original, profil)` mit `_trim` bzw. `_<N>MB`.

##### 21. `crates/gameclip-core/tests/*.rs` (Create)

*   **Type / Scope**: Unit-Tests.
*   **Proposed Changes**:
    *   `state_tests.rs`: jede Zeile der Übergangstabelle.
    *   `retention_tests.rs`: geschützt, fremd, Clip größer als Limit, gemeinsamer Ordner, neuer Clip wird nie gelöscht.
    *   `forecast_tests.rs`: Tabelle mit erwarteten MB je Auflösung, Stufe und Länge.
    *   `naming_tests.rs`: Sonderzeichen, Länge, Kollision.
    *   `migrate_tests.rs`: unbekannte Felder bleiben erhalten, neueres Schema ergibt nur Lesen.

#### `crates/gameclip-protocol` (kein Windows-Code)

##### 22. `crates/gameclip-protocol/src/lib.rs`, `messages.rs`, `framing.rs`, `error.rs`, `version.rs` (Create)

*   **Type / Scope**: IPC-Vertrag.
*   **Proposed Changes**:
    *   `messages.rs`: Enums `Request` und `Event` mit allen Befehlen und Ereignissen aus dem Technischen Plan, `serde` mit `tag = "type"`, `#[derive(TS)]` für die TypeScript-Ausgabe.
    *   `framing.rs`: Lesen und Schreiben mit u32-Längenpräfix, Höchstgröße 8 MB, über `std::io::Read`/`Write`, ohne Plattformcode.
    *   `error.rs`: Fehlercodes als Enum.
    *   `version.rs`: `PROTOCOL_MAJOR = 1`, `PROTOCOL_MINOR = 0` und die Regel für den Handshake.
*   **Warum**: Gemeinsamer Vertrag für Dienst, Oberfläche, Attrappe und die spätere Linux-Version.

##### 23. `crates/gameclip-protocol/tests/roundtrip.rs` (Create)

*   **Proposed Changes**: Jede Nachricht wird serialisiert und wieder eingelesen. Weitere Tests: unbekannte Felder werden ignoriert, Nachricht über 8 MB wird abgelehnt, die Beispiele aus `docs/protocol.md` werden geparst.

##### 24. `crates/gameclip-fake-daemon/src/main.rs` (Create)

*   **Type / Scope**: Attrappe des Dienstes für Tests.
*   **Proposed Changes**: Spricht das Protokoll über einen lokalen Socket (Unix-Socket unter Linux, Named Pipe unter Windows über `interprocess`), liefert feste Clips, Status und Fortschritt und lässt sich per Kommandozeile auf Fehlerfälle schalten.
*   **Warum**: Frontend und Protokoll lassen sich ohne Windows-Hardware testen, auch im Linux-CI-Job.

#### `crates/gameclip-media` (FFmpeg, Windows)

##### 25. `crates/gameclip-media/Cargo.toml` und `build.rs` (Create)

*   **Proposed Changes**: Abhängigkeit `rsmpeg` mit dynamischem Linken. `build.rs` liest `FFMPEG_DIR` aus `packaging/ffmpeg/ffmpeg.lock`, das Skript `fetch-ffmpeg.ps1` setzt den Pfad.

##### 26. `crates/gameclip-media/src/hwdevice.rs` (Create)

*   **Proposed Changes**: Erzeugt einen FFmpeg-`AVHWDeviceContext` vom Typ D3D11VA aus einem vorhandenen `ID3D11Device` und den passenden `AVHWFramesContext` (NV12, Poolgröße 8). Für QSV wird ein abgeleiteter Kontext erzeugt.
*   **Warum**: Gemeinsames GPU-Gerät für Aufnahme und Encoder, damit das Bild auf der GPU bleibt (D13).

##### 27. `crates/gameclip-media/src/encoder.rs` (Create)

*   **Type / Scope**: Encoder-Prüfung und Ausweichkette.
*   **Proposed Changes**:
    *   `probe(adapter, auflösung, fps, bitrate) -> Vec<EncoderKandidat>` öffnet je Kandidat eine echte Sitzung und kodiert 10 Testbilder.
    *   Reihenfolge: Hersteller-Encoder der Monitor-GPU (`h264_nvenc` bzw. `h264_amf` bzw. `h264_qsv`), dann `h264_mf` (nur wenn ein Feature-Flag nach S2 es freigibt), dann `libx264`.
    *   Gemeinsame Optionen: Profil High, `g=fps`, `bf=0`, geschlossene GOP, `maxrate = bitrate`, `bufsize = bitrate`.
    *   Je Encoder die passende Zuordnung: NVENC `preset p4`, `tune ll`, `rc cbr`; AMF `usage lowlatency`, `rc cbr`; QSV `preset medium`, `look_ahead 0`; x264 `preset superfast`, `tune zerolatency`.
    *   `EncoderKandidat` enthält den Grund des Fehlschlags für „Diagnose kopieren“.

##### 28. `crates/gameclip-media/src/audio_codec.rs` (Create)

*   **Proposed Changes**: AAC-LC-Encoder (FFmpeg `aac`) für 48 kHz Stereo. Eingabe sind Float-Blöcke, Ausgabe Pakete mit PTS in Samples. Wird für Einzelspuren und Mischung genutzt.

##### 29. `crates/gameclip-media/src/mux.rs` (Create)

*   **Type / Scope**: Muxer.
*   **Proposed Changes**:
    *   `write_mp4(pfad, video_pakete, audio_spuren: &[(titel, pakete, standard)], tags, faststart: bool)` und `write_m4a(pfad, audio_spuren, tags)`.
    *   Schreibt erst nach `<pfad>.part` und ruft dann `FlushFileBuffers` auf. Umbenannt wird beim Aufrufer.
    *   PTS werden auf den ersten Keyframe normiert.
*   **Warum**: Dienst (Hotkey-Speichern) und Worker (Export) schreiben Dateien auf dieselbe Weise.

##### 30. `crates/gameclip-media/src/decode.rs`, `scale.rs` (Create)

*   **Proposed Changes**:
    *   `decode.rs`: Hardware-Decoder `d3d11va` mit Ausweichen auf den Software-Decoder und Suche nach dem Keyframe vor einer PTS.
    *   `scale.rs`: Skalieren auf der GPU über den Filter `scale_d3d11`, falls vorhanden (prüfen), sonst über den D3D11-VideoProcessor aus `gameclip-capture-win`, der als Callback übergeben wird.
*   **Warum**: Nur der Worker nutzt diese Teile, sie gehören aber zur FFmpeg-Schicht.

##### 31. `crates/gameclip-media/tests/encode_mux.rs` (Create)

*   **Type / Scope**: Integrationstest mit synthetischer Quelle.
*   **Proposed Changes**:
    *   Erzeugt 10 s Testbild plus Sinuston, kodiert mit `libx264` und `aac` und schreibt MP4 und M4A.
    *   Prüft per `ffprobe -show_streams -show_frames`: Dauer, Spuranzahl, Spurtitel, Keyframe am Anfang, A/V-Versatz unter einem Bild, Tag `gameclip_id`.

#### `crates/gameclip-capture-win` (Windows)

##### 32. `src/d3d.rs` (Create)

*   **Proposed Changes**:
    *   Wählt den Adapter, an dem der gewählte Monitor hängt (`IDXGIOutput` → `IDXGIAdapter`, LUID).
    *   Erzeugt `ID3D11Device` mit `D3D11_CREATE_DEVICE_VIDEO_SUPPORT | BGRA_SUPPORT` und schaltet Multithread-Schutz an.
    *   Erkennt Geräteverlust über `GetDeviceRemovedReason`.

##### 33. `src/wgc.rs` (Create)

*   **Proposed Changes**:
    *   `GraphicsCaptureItem` aus `HMONITOR` über `IGraphicsCaptureItemInterop`.
    *   `RequestAccessAsync(Borderless)`, danach `IsBorderRequired = false`.
    *   `Direct3D11CaptureFramePool::CreateFreeThreaded` mit 2 Puffern, `IsCursorCaptureEnabled` laut Einstellung, `MinUpdateInterval = 1/fps`, wenn verfügbar.
    *   `FrameArrived` kopiert das Bild in einen eigenen Ring aus 3 Texturen und vermerkt den QPC-Zeitstempel (`SystemRelativeTime`).
*   **Warum**: Aufnahme ohne Injektion (G4).

##### 34. `src/pacer.rs` (Create)

*   **Proposed Changes**: Thread mit `CreateWaitableTimerExW(HIGH_RESOLUTION)` im Takt `1/fps`. Je Takt nimmt er die neueste Textur aus dem Ring oder wiederholt die letzte. Er zählt wiederholte und verworfene Bilder für die Diagnose.

##### 35. `src/convert.rs` (Create)

*   **Proposed Changes**: `ID3D11VideoProcessor` mit Eingabe BGRA in Quellgröße und Ausgabe NV12 in Zielgröße, Farbraum BT.709 mit begrenztem Bereich und Seitenverhältnis mit Rand. Ausgegeben wird in eine Textur aus dem FFmpeg-Frame-Pool.

##### 36. `src/video_pipeline.rs` (Create)

*   **Proposed Changes**: Verbindet `wgc`, `pacer`, `convert` und den Encoder aus `gameclip-media`. Encoder-Pakete gehen an den Ringpuffer. Bei Geräteverlust oder HDR-Wechsel meldet die Pipeline das als Ereignis an den Dienst. Für x264 gibt es eine Kopie über eine Staging-Textur in den Hauptspeicher.

##### 37. `src/audio/loopback.rs` (Create)

*   **Proposed Changes**:
    *   Prozess-Loopback über `ActivateAudioInterfaceAsync` mit `AUDIOCLIENT_ACTIVATION_PARAMS`.
    *   Modus `EXCLUDE` mit Ziel-PID (gameclipd oder Discord) bzw. `INCLUDE` für die Discord-Spur.
    *   Das Format ist fest vorgegeben (48 kHz, Float, Stereo), weil `GetMixFormat` hier nicht verfügbar ist.
    *   Ereignisgesteuert, Zeitstempel über `IAudioCaptureClient::GetBuffer` (QPC-Position).
*   **Warum**: System-Ton ohne GameClip bzw. ohne Discord, und Discord als eigene Spur (D16).

##### 38. `src/audio/mic.rs` (Create)

*   **Proposed Changes**:
    *   WASAPI-Aufnahme im Shared Mode vom gewählten Gerät (`IMMDeviceEnumerator`, ID aus den Einstellungen, ersatzweise das Standardgerät).
    *   Bei `IMMNotificationClient`-Meldungen (Gerät entfernt oder neu) wird neu gebunden.
    *   Bei Pause liefert die Quelle Stille.

##### 39. `src/audio/discord.rs` (Create)

*   **Proposed Changes**: Sucht alle 2 s nach `Discord.exe` (Toolhelp-Snapshot) und wählt den Prozess, dessen Elternprozess nicht `Discord.exe` heißt. Startet bzw. stoppt `loopback` mit `INCLUDE` auf diese PID. Wechselt die Discord-PID, wird auch die System-Quelle (`EXCLUDE`) neu gebunden.

##### 40. `src/audio/resample.rs`, `mixer.rs` (Create)

*   **Proposed Changes**:
    *   `resample.rs`: Umrechnung auf 48 kHz mit `rubato`. Die Drift wird über den Abstand zwischen QPC-Zeit und Sample-Zählung geschätzt und über das Verhältnis korrigiert. Lücken werden mit Stille aufgefüllt.
    *   `mixer.rs`: Blöcke von 10 ms aus allen Quellen, Gain in dB je Quelle, Summe, Limiter (Look-ahead 5 ms, Schwelle −1 dBFS). Fenster für die Stummschaltung des eigenen Tons auf der System-Quelle.

##### 41. `src/ring.rs` (Create)

*   **Type / Scope**: Ringpuffer.
*   **Proposed Changes**:
    *   Block-Pool aus `Arc<Block>` zu 2 MB, Paketindex als `VecDeque<PacketRef>`, Abschnittszähler.
    *   `push(spur, paket)` und `evict()` nach Zeit- und Bytegrenze, immer als ganze Video-Bildgruppe.
    *   `snapshot(von_qpc, bis_qpc) -> Snapshot` hält die Blöcke per `Arc` fest, nur aus dem neuesten Abschnitt.
    *   Ist der Pool voll und ein Schnappschuss hält Blöcke, werden die neuen Pakete verworfen und gezählt.
*   **Warum**: Kern des Pufferns (D2). Konstanter Speicher ohne Fragmentierung.

##### 42. `crates/gameclip-capture-win/tests/ring_tests.rs` (Create)

*   **Proposed Changes**: Mit synthetischen Paketen: Verwerfen nur ganzer GOPs, Audio bleibt passend zum ersten Keyframe, Schnappschuss während des Verwerfens, voller Pool mit gehaltenem Schnappschuss, Abschnittswechsel.

#### `crates/gameclipd` (Windows)

##### 43. `src/main.rs` (Create)

*   **Proposed Changes**:
    *   Ein Mutex `Local\<AppID>` stellt sicher, dass nur eine Instanz läuft.
    *   Danach starten in dieser Reihenfolge: Logging, Einstellungen, Index, Encoder-Prüfung, Pipeline, Hotkey, Tray, IPC-Server.
    *   Danach geht die Hauptschleife über einen `crossbeam`-Kanal mit Ereignissen an die Zustandsmaschine aus `gameclip-core`. Ihre Wirkungen führt der Dienst aus.
    *   Kein Konsolenfenster (`windows_subsystem = "windows"`).
    *   Wird das Programm mit `--save-clip` aufgerufen, startet `main.rs` keinen Dienst, sondern verzweigt nach `cli.rs`.

##### 43a. `src/cli.rs` (Create)

*   **Type / Scope**: Interne Kommandozeile (D23).
*   **Proposed Changes**:
    *   `gameclipd.exe --save-clip` verbindet sich mit der Pipe der laufenden Instanz, führt den Handshake aus und sendet `SaveClip`. Dann wartet es höchstens 10 s auf `SaveCompleted` oder `SaveFailed`.
    *   Exit-Codes: 0 bei Erfolg, 1 bei einem Fehler oder wenn kein Dienst läuft. Die Fehlermeldung landet in einer Zeile auf stderr (`AttachConsole(ATTACH_PARENT_PROCESS)`, falls von einer Konsole gestartet).
    *   Im MSIX-Paket gibt es keinen App-Execution-Alias. Aufrufbar ist die Kommandozeile nur über den vollen Installationspfad.
    *   Die Kommandozeile wird nicht öffentlich dokumentiert und hat keine Stabilitätszusage. Sie wird in `docs/protocol.md` nur als „intern“ erwähnt.
*   **Warum**: D23 im Brief; nützlich für Tests und Skripte während der Entwicklung.

##### 44. `src/save.rs` (Create)

*   **Proposed Changes**: Ablauf „Speichern per Hotkey“ Schritte 1–7 aus dem Technischen Plan:
    *   Schnappschuss nehmen.
    *   MP4 (Mischung, ohne faststart) und M4A (Einzelspuren) über `gameclip-media::mux` schreiben.
    *   Per `MoveFileExW(MOVEFILE_WRITE_THROUGH)` umbenennen.
    *   Index eintragen, Speicherlimit ausführen, Ton abspielen und Ereignis senden.
    *   Jeder Speichervorgang läuft in einem eigenen Thread, höchstens zwei gleichzeitig.

##### 45. `src/foreground.rs` (Create)

*   **Proposed Changes**: `GetForegroundWindow` → `GetWindowThreadProcessId` → `QueryFullProcessImageNameW` → Dateiname ohne Endung. Bei Zugriffsfehler (z. B. ein Spiel mit Administratorrechten) gibt die Funktion `None` zurück.

##### 46. `src/index.rs` und `migrations/001_init.sql` (Create)

*   **Proposed Changes**:
    *   Tabellen `clips` und `meta` wie im Technischen Plan, WAL-Modus, ein Schreib-Thread.
    *   Funktionen `insert`, `set_star`, `mark_missing`, `list(folder)`, `owned_ok(folder)`, `adopt(id)` und `rebuild_from_tags()`.
    *   Vor jeder Migration wird die Datei kopiert.

##### 47. `src/watcher.rs` (Create)

*   **Proposed Changes**:
    *   `ReadDirectoryChangesW` auf Clip- und Export-Ordner.
    *   Bei Umbenennen wird der Pfad aktualisiert. Bei Löschen gilt `missing`, und ein Zeitstempel wird vermerkt. Ist ein Clip seit mehr als 24 h `missing`, werden Eintrag und `.m4a` entfernt.
    *   Neue Dateien mit Tag, die nicht im Index stehen, werden `foreign`.
    *   Beim Start läuft ein vollständiger Abgleich.

##### 48. `src/retention_exec.rs` (Create)

*   **Proposed Changes**: Ruft `gameclip-core::retention::plan_deletions` auf. Vor jedem Löschen wird der Tag `gameclip_id` aus der Datei gelesen und mit dem Index verglichen. Dann folgen `DeleteFileW` für MP4 und `.m4a`, das Ereignis `ClipDeleted` und ein Logeintrag mit Pfad, Größe und Grund. Bei einer Abweichung wird nicht gelöscht, der Eintrag wird `foreign`, und es erscheint eine Warnung.

##### 49. `src/jobs.rs` (Create)

*   **Proposed Changes**:
    *   Warteschlange mit höchstens einem laufenden Auftrag.
    *   `gameclip-worker.exe` wird mit `BELOW_NORMAL_PRIORITY_CLASS` in einem Job-Objekt gestartet, damit er beim Ende des Dienstes mit beendet wird.
    *   Der Auftrag geht als JSON über stdin. Die JSON-Zeilen von stdout werden in Ereignisse übersetzt.
    *   Zeitlimit 30 s ohne Meldung.
    *   Bei Erfolg: Umbenennen, Index, Speicherlimit des Export-Ordners. Bei Fehler oder Abbruch wird `.part` gelöscht.

##### 50. `src/hotkey.rs` (Create)

*   **Proposed Changes**: Eigener Thread mit einem Message-only-Fenster. `RegisterHotKey(MOD_NOREPEAT | Modifier, VK)`, F12 und Win-Kombinationen werden abgelehnt. Nachrichten `Suspend`, `Resume` und `TestRegister` kommen über `PostThreadMessage`. `WM_HOTKEY` wird mit dem QPC-Zeitstempel an die Hauptschleife gemeldet.

##### 51. `src/tray.rs` (Create)

*   **Proposed Changes**: `tray-icon` mit fünf Symbolen (Puffert, Speichert, Eingeschränkt, Fehler, Mikrofon pausiert) und dem Menü aus dem Technischen Plan. „Galerie öffnen“ startet `gameclip-ui` über die AUMID (`IApplicationActivationManager`).

##### 52. `src/sound.rs` und `assets/sounds/success.wav`, `failure.wav` (Create)

*   **Proposed Changes**: WASAPI-Wiedergabe der zwei WAV-Dateien mit eingestellter Lautstärke. Beginn und Dauer werden an den Mixer gemeldet, falls die Stummschaltung des eigenen Tons aktiv ist. Die Töne sind selbst erstellt und stehen unter einer freien Lizenz.

##### 53. `src/toast.rs` (Create)

*   **Proposed Changes**: `ToastNotificationManager::CreateToastNotifier(AUMID)` mit einfachem Text-Template. Bei Erfolg „Clip gespeichert (Dauer, Größe)“ mit Knopf „Im Ordner anzeigen“, bei Fehler der Fehlergrund.

##### 54. `src/ipc_server.rs` (Create)

*   **Proposed Changes**:
    *   `CreateNamedPipeW` mit `SECURITY_ATTRIBUTES` aus einer SDDL nur für die Benutzer-SID, `PIPE_REJECT_REMOTE_CLIENTS`, bis zu 4 Verbindungen.
    *   Je Verbindung ein Thread: Handshake, Anfragen, Ereignisse senden. Rahmung über `gameclip-protocol::framing`.
    *   `SetSettings` prüft Ordnerpfade (existiert, beschreibbar, nicht unter `%WINDIR%`, `%ProgramFiles%` oder dem Laufwerksstamm).

##### 55. `src/diagnostics.rs`, `logging.rs` (Create)

*   **Proposed Changes**:
    *   `logging.rs`: `tracing-appender` mit täglich rotierender Datei, höchstens 5 Dateien zu je 10 MB.
    *   `diagnostics.rs`: Zähler je Minute und je Speichervorgang wie im Technischen Plan. `CopyDiagnostics` sammelt Windows-Build, GPU und Treiber (`IDXGIAdapter::GetDesc1` plus Treiberversion aus der Registry), das Ergebnis der Encoder-Prüfung, die Audiogeräte und die letzten 200 Logzeilen. `%USERPROFILE%` wird durch `<USER>` ersetzt.

#### `crates/gameclip-worker` (Windows)

##### 56. `src/main.rs`, `job.rs`, `trim.rs`, `size_target.rs` (Create)

*   **Proposed Changes**:
    *   `job.rs`: Liest den Auftrag von stdin, schreibt JSON-Zeilen nach stdout und setzt die Exit-Codes 0 bzw. 1.
    *   `trim.rs`: Ablauf Schritte 1–5 aus „Kürzen und Export“. Encoder-Auswahl über `gameclip-media::encoder` mit LUID des Adapters, Audio sample-genau mit `aac`.
    *   `size_target.rs`: Bitratenformel, Auflösungsstufen (Bits pro Pixel und Bild unter 0,05 ergibt die nächste Stufe), Prüfung danach und höchstens ein Wiederholversuch.
    *   Kein Konsolenfenster.

##### 57. `crates/gameclip-worker/tests/trim_tests.rs` (Create)

*   **Proposed Changes**: Mit der synthetischen Quelle aus Datei 31, deren Bilder durchnummeriert sind:
    *   Schnitt bei Bild N ergibt Bild N als erstes Bild.
    *   Audio-Länge stimmt auf ±1 ms.
    *   Export mit getrennten Spuren hat die richtige Reihenfolge und den Standard-Marker.
    *   Größen-Export für 20 Dauern zwischen 5 und 300 s bleibt unter 10 MB.

#### `crates/gameclip-ui-platform` (Windows-Umsetzung)

##### 58. `src/lib.rs`, `clipboard.rs`, `drag.rs`, `reveal.rs`, `thumbnail.rs` (Create)

*   **Proposed Changes**:
    *   Trait `UiPlatform` mit `copy_file`, `start_drag`, `reveal` und `thumbnail(pfad, größe) -> PNG`.
    *   `clipboard.rs`: `CF_HDROP` über `clipboard-win`.
    *   `drag.rs`: Anbindung an `tauri-plugin-drag`.
    *   `reveal.rs`: `SHOpenFolderAndSelectItems`.
    *   `thumbnail.rs`: `IShellItemImageFactory::GetImage`.
    *   Für Tests gibt es eine Attrappe.
*   **Warum**: Hält Windows-Code aus `gameclip-ui` heraus.

#### `apps/gameclip-ui` (Tauri + React)

##### 59. `apps/gameclip-ui/src-tauri/Cargo.toml`, `tauri.conf.json`, `capabilities/default.json` (Create)

*   **Proposed Changes**:
    *   Tauri 2 ohne Updater-Plugin, ein Fenster mit 1100×720 und Mindestgröße 800×560.
    *   CSP erlaubt nur `self` und `clip:`.
    *   Die Capabilities erlauben nur die eigenen Befehle und `tauri-plugin-drag`.
    *   `bundle.active = false`, weil das MSIX vom eigenen Skript gebaut wird.

##### 60. `apps/gameclip-ui/src-tauri/src/main.rs`, `ipc_client.rs`, `clip_protocol.rs`, `commands.rs` (Create)

*   **Proposed Changes**:
    *   `ipc_client.rs`: Verbindet sich mit der Pipe und führt den Handshake aus. Bei abweichender Hauptversion geht das Ereignis `version_mismatch` an das Frontend. Startet gameclipd nicht, solange die Verbindung fehlt, versucht es alle 2 s erneut.
    *   `clip_protocol.rs`: Eigenes Protokoll `clip://<id>`, das den Pfad über den Dienst auflöst. Es liefert die Datei mit `Range`-Unterstützung (206, `Content-Range`) aus, nur für Pfade, die der Dienst nennt.
    *   `commands.rs`: Tauri-Befehle spiegeln die Protokollbefehle. Dazu kommen `copy_file`, `start_drag`, `reveal` und `thumbnail` über `gameclip-ui-platform`.

##### 61. `apps/gameclip-ui/package.json`, `vite.config.ts`, `tsconfig.json`, `index.html` (Create)

*   **Proposed Changes**: React 18, TypeScript im Strict-Modus, Vite, `vitest` und `@testing-library/react`. Skripte `dev`, `build`, `test`, `typecheck`, `gen:types`. Letzteres kopiert die von `ts-rs` erzeugten Typen nach `src/generated/`.

##### 62. `apps/gameclip-ui/src/ipc.ts`, `store.ts`, `generated/protocol.ts` (Create)

*   **Proposed Changes**:
    *   `ipc.ts`: Je Befehl eine typisierte Funktion über `invoke`. Ereignisse kommen über `listen`.
    *   `store.ts`: Ein externer Store mit `status`, `clips`, `exports`, `jobs` und `settings`, angebunden über `useSyncExternalStore`. Er wird über Ereignisse aktualisiert.
    *   `generated/protocol.ts`: erzeugt, nicht von Hand bearbeiten.

##### 63. `apps/gameclip-ui/src/App.tsx`, `main.tsx` (Create)

*   **Proposed Changes**:
    *   Wechsel zwischen `FirstRun`, `Gallery`, `Editor` und `Settings`. `FirstRun` erscheint, solange `settings.first_run_done == false`.
    *   Eine Statusleiste ist immer sichtbar.
    *   Bei `version_mismatch` erscheint eine Vollbild-Meldung.

##### 64. `apps/gameclip-ui/src/views/FirstRun.tsx`, `Gallery.tsx`, `Editor.tsx`, `Settings.tsx` (Create)

*   **Proposed Changes**:
    *   `FirstRun`: drei Schritte (Mikrofon mit Pegelanzeige und Hinweis „wird standardmäßig aufgenommen“, Hotkey mit Testdruck und Warnung, Clip-Ordner), danach „Test-Clip erstellen“. Fehlt das Media Feature Pack, wird der Hinweis vorangestellt.
    *   `Gallery`: Reiter „Clips“ und „Exporte“, Kachelraster, Sortierung nach Datum, Platzanzeige „7,2 / 10 GB“, Stern, „Fremd“-Markierung mit „Übernehmen“, Kontextmenü (Kopieren, Im Ordner anzeigen, Löschen mit Rückfrage).
    *   `Editor`: Player, `TrimBar`, Bildschritt mit Pfeiltasten, `ExportDialog`.
    *   `Settings`: Gruppen Aufnahme (Länge, Auflösung, Stufe, Monitor, Mauszeiger, mit `ForecastLabel`), Audio (Geräte, Discord an/aus, Lautstärkeregler), Hotkey, Speicher (Ordner, Limits), Töne, Autostart, Diagnose.

##### 65. `apps/gameclip-ui/src/components/*.tsx` (Create)

*   **Proposed Changes**:
    *   `TrimBar`: Leiste mit Start- und Endgriff. Die Position läuft über `ref` und `requestVideoFrameCallback`. Die Griffe rasten auf `GetFrameTimes` ein.
    *   `ExportDialog`: Auswahl Mischung (Standard) / Getrennte Spuren / Zielgröße (MB-Feld), dazu Fortschritt und Abbrechen.
    *   `HotkeyPicker`: ruft `SuspendHotkey` auf, liest `KeyboardEvent.code`, bietet eine Auswahlliste für F13–F24, ruft `TestRegister` auf und zeigt die Warnung.
    *   `VolumeSlider`: −30 bis +12 dB, Doppelklick setzt auf 0 dB.
    *   `ForecastLabel`: zeigt „ca. N MB RAM“ und die Warnstufe.
    *   `ClipCard`, `StatusBar`.

##### 66. `apps/gameclip-ui/src/styles/tokens.css` und `*.module.css` (Create)

*   **Proposed Changes**: CSS-Variablen für Farben (helles und dunkles Thema über `prefers-color-scheme`), Abstände (4-px-Raster), Schriftgrößen (Segoe UI Variable) und Radien. Je Komponente ein CSS-Modul.

##### 67. `apps/gameclip-ui/src/**/*.test.tsx` (Create)

*   **Proposed Changes**: Tests gegen einen gemockten `ipc.ts`:
    *   `TrimBar` rastet auf Bildzeiten ein.
    *   `ExportDialog` schickt das richtige Profil.
    *   `HotkeyPicker` lehnt F12 ab.
    *   `Gallery` zeigt `foreign` ohne Löschoption für das Limit.
    *   `FirstRun` durchläuft alle Schritte.

#### Paket

##### 68. `packaging/AppxManifest.template.xml` (Create)

*   **Proposed Changes**:
    *   Identity mit `Name="io.github.poldion.GameClip"` und Platzhaltern für Publisher (Zertifikatssubjekt, Q2) und Version.
    *   Zwei Applications: `GameClip` mit `gameclip-ui.exe` im Startmenü und `GameClipService` mit `gameclipd.exe` und `AppListEntry="none"`.
    *   `windows.startupTask` für `gameclipd`.
    *   Capabilities: `runFullTrust`, `microphone`, `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` (prüfen).
    *   `TargetDeviceFamily` mit MinVersion 10.0.26100.0.

##### 69. `packaging/build-msix.ps1` (Create)

*   **Proposed Changes**: Setzt das Layout aus den Release-Builds, den FFmpeg-DLLs, den Tönen, den Lizenzen und den Icons zusammen. Ersetzt die Platzhalter im Manifest, ruft `MakeAppx pack` auf und signiert mit `SignTool` (lokal Testzertifikat, in CI SignPath).

##### 70. `packaging/ffmpeg/fetch-ffmpeg.ps1` und `ffmpeg.lock` (Create)

*   **Proposed Changes**: `ffmpeg.lock` enthält die BtbN-Release-URL des GPL-Shared-Builds (Version 7.1.x), die SHA-256 und die Quellcode-URL. Das Skript lädt, prüft die Prüfsumme und entpackt nach `third_party/ffmpeg/`, ohne in den Projektbaum zu committen.

##### 71. `packaging/assets/*.png` (Create)

*   **Proposed Changes**: App-Icons in den MSIX-Größen (44, 50, 150, 310) und Tray-Icons (16, 20, 24, 32) für die fünf Zustände.

##### 72. `.gitignore` (Create)

*   **Proposed Changes**: `target/`, `node_modules/`, `dist/`, `third_party/ffmpeg/`, `*.msix`, `*.pfx`.

### Verification Plan

*   **Compilation/Build:**
    *   `cargo build --workspace --release`
    *   `npm ci && npm run build` in `apps/gameclip-ui`
    *   `pwsh packaging/build-msix.ps1 -TestCert`
*   **Unit Tests:**
    *   `cargo test --workspace`
    *   `npm test` in `apps/gameclip-ui`
*   **Static Analysis & Linting:**
    *   `cargo clippy --workspace -- -D warnings`
    *   `cargo fmt --check`
    *   `cargo deny check` (Lizenzen und Plattformgrenze)
    *   `npm run typecheck`
*   **Test Coverage Requirement:**
    *   `gameclip-core` mindestens 90 % Zweigabdeckung, weil Zustandsmaschine, Speicherlimit und Prognose dort liegen.
    *   `gameclip-protocol` mindestens 85 %.
    *   Gemessen mit `cargo llvm-cov`.
*   **E2E/Integration:**
    *   `cargo test -p gameclip-media -p gameclip-worker` (synthetische Quelle, Prüfung per ffprobe)
    *   Linux-CI-Job gegen `gameclip-fake-daemon`
*   **Manuelle Prototypen und Abnahme** (Protokolle in `docs/test-protocols.md`):

    | Test | Bestanden, wenn |
    | --- | --- |
    | S1 | Vanguard 48 h ohne Befund und 14 Tage ohne Sperre, Speicherzeit p95 ≤ 3 s, CPU ≤ 3 %, Dienst-RAM ≤ 80 MB plus Puffer (vorläufig) |
    | S2 | NVENC, AMF und QSV kodieren ohne CPU-Kopie; Laptop mit zwei GPUs; HDR wird erkannt |
    | S4 | A/V-Versatz < 1 Bild nach 30 min, Discord-Spur getrennt, eigener Ton nicht im nächsten Clip |
    | S5 | Spulen in einem 5-min-Clip ohne Fehler, Range-Anfragen funktionieren, N-Edition wird erkannt |
    | S6 | Datei kommt per Zwischenablage und Drag-and-drop in Discord und Explorer an |
    | S7 | signiertes MSIX auf frischer VM ohne SmartScreen- und Defender-Warnung |
    | S8 | F8 und per SendInput gesendetes F13 lösen unter Vanguard aus, Verhalten in einem Raw-Input- und einem DirectInput-Spiel dokumentiert |
    | S9 | Export bei laufendem Spiel ohne verworfene Bilder im Puffer, Schnitt bei Bild N ergibt Bild N |
    | S11 | 20 Größen-Exporte, keiner über der Zielgröße |
    | G1 | Vergleich mit Game Bar und OBS dokumentiert; mindestens 2 von 4 Werten besser |

--------------------------------------------------------------------------------

*Drafted with EGM*
