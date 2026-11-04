# GameClip – Open-Source-Clip-Rekorder für Spiele (Windows)

* **Author:** Pold
* **Date:** 2026-11-04
* **Status:** Review-Runde 2 / Revision 10

| Revision | Inhalt |
| --- | --- |
| 1 | Erste Fassung nach Goldfish-Review-Runde 1 (12 Blocker, 27 Major, 14 Nits) |
| 2 | Windows und Ubuntu in v1, gpu-screen-recorder für Linux, gemischte Hauptspur |
| 3 | v1 nur Windows, kein Abmischen im Editor |
| 4 | Nur Windows 11, Hotkey F8 frei belegbar, Galerie und Trimmen in v1, bildgenauer Schnitt |
| 5 | Ergebnisse der Goldfish-Review-Runde 2 eingearbeitet (5 Blocker, 22 Major, 6 Nits), Dokument neu gegliedert, Linux in Anhang A |
| 6 | Spurmodell: Einzelspuren immer aufnehmen, Export standardmäßig nur Mischung (D7 geändert, P1 ersetzt) |
| 7 | Clip- und Export-Ordner nur mit Mischung, Einzelspuren intern; GB-Limit je Ordner; Mikrofon standardmäßig an mit Geräteauswahl; optionale Discord-Spur; HDR nur Warnung (D14–D17) |
| 8 | Lautstärke je Quelle (Standard 0 dB), Arbeitsprozess, Zielgröße, Stern-Schutz und 10 GB Standard, Spur-Export mit Mischung (D18–D22) |
| 9 | Kommandozeile intern, endgültiges Löschen, Spurnamen (D23–D25). Alle Vorschläge aus Runde 2 entschieden; offen sind nur technische Fragen für Prototypen und Design-Dokument |
| 10 | P2 bestätigt (D26), automatische Pause bei gesperrtem PC (D27); Details im Design-Dokument docs/designs/gameclip.md |

## Overview

GameClip ist eine Open-Source-Desktop-App für Windows 11. Sie nimmt im Hintergrund ständig das Spielgeschehen auf und hält die letzten 1 bis 5 Minuten als kodierte Videodaten im RAM. Drückt der Spieler einen Hotkey, schreibt GameClip diesen Puffer als MP4-Datei auf die Festplatte. In einer Galerie lassen sich Clips ansehen, bildgenau kürzen, für Discord verkleinern und per Zwischenablage oder Drag-and-drop in einen Messenger geben.

Zielgruppe sind Spieler, die eine kostenlose, offene Alternative zu Medal suchen: ohne Konto, ohne Cloud, ohne Werbung, mit wenig Ressourcenverbrauch und einer übersichtlichen Oberfläche. Die App muss mit Spielen funktionieren, die einen Kernel-Anti-Cheat nutzen, insbesondere Valorant (Riot Vanguard).

**Wettbewerb unter Windows:** Xbox Game Bar (vorinstalliert, kostenlos, ohne Konto, jede GPU), OBS Replay Buffer, NVIDIA App Instant Replay, AMD ReLive und Medal. Die Aufnahme allein ist kein Alleinstellungsmerkmal. Der wahrscheinliche Vorteil von GameClip ist der Ablauf **Hotkey → kürzen → Discord-Export → Zwischenablage** bei geringem Verbrauch. Ob das stimmt, wird in Phase 0 gemessen (siehe Erfolgskriterien).

Linux (Ubuntu) folgt in einer späteren Version mit gpu-screen-recorder als Aufnahmekern (Anhang A).

## Analysis

### Erfolgskriterien und Messgrößen

**Vergleich mit Wettbewerbern (Go/No-Go für das Projekt).** In Phase 0 werden auf demselben Referenzrechner gegen Xbox Game Bar und OBS Replay Buffer gemessen:

| Messgröße | Bedeutung |
| --- | --- |
| Minuten bis zum ersten Clip | ab Start des Installers |
| RAM und CPU im Leerlauf | Puffer läuft, Spiel läuft, Fenster geschlossen |
| Anzahl nötiger Einstellungen | bis zu einem brauchbaren Standard-Clip |
| Zeit bis zur teilbaren Datei < 10 MB | vom Hotkey bis zur Datei in der Zwischenablage |

Regel: GameClip muss Game Bar bei mindestens 2 von 4 Werten schlagen. Sonst wird der Umfang überdacht.

**Speicherzeit.** Vom Tastendruck bis zur geschlossenen MP4-Datei (moov geschrieben, fsync, Umbenennen von `.part`), in 95 % der Fälle höchstens 3 s. Gilt mit Standardeinstellungen, auf NVMe/SSD und mit Hardware-Encoder. Speichern ist nur Umpacken, kein Neukodieren. Der Erfolgston kommt erst, wenn die Datei fertig ist.

**RAM, in vier Teilen:**

| Teil | Ziel (Vorschlag) |
| --- | --- |
| (a) Dienst ohne Puffer | höchstens 60 bis 80 MB |
| (b) Puffer | nach Formel, in den Einstellungen angezeigt |
| (c) Zusatz bei offenem Fenster | separat gemessen |
| (d) Spitze beim Trimmen/Export | separat gemessen |

Formel für (b): `(V_max + Spuren × A) × (T + GOP) / 8 × 1,05 + Block-Aufrundung + Grundbedarf`. Dabei ist `V_max` die maximale Videobitrate, `A` die Audiobitrate je Spur, `T` die Pufferlänge und `GOP` die Länge einer Bildgruppe. Die Formel wird mit einer Tabelle als Unit-Test abgesichert.

Beispiel: 5 min 1440p bei ~30 Mbit/s ergeben etwa 1,1 GB Puffer.

Anzeige: Qualitätsstufen statt Mbit/s, z. B. „1080p60 Mittel (12–15 Mbit/s) – ca. 160 MB RAM (+3 MB mit Einzelspuren)“.

Warnregel (Vorschlag): gelb mit Rückfrage ab etwa 10 % des eingebauten RAM, Ablehnung ab 25 %.

**CPU.** Höchstens ~3 % für den Dienst mit Hardware-Encoder. Gemessen wird der Mittelwert über 10 Minuten auf einem benannten Referenzrechner mit einem benannten Spiel in 1080p60, mit PresentMon oder WPR. Zusätzlich wird eine Grenze für Bildverluste im Spiel festgelegt. Die Zahlen werden vor S1 festgelegt.

**Lokale Diagnose (ohne Netzwerk-Telemetrie).** Ohne diese Werte lassen sich die Ziele auf fremden Rechnern nicht prüfen:

- **Je Speichervorgang:** Dauer, Bytes, Anzahl der Bildgruppen, Encoder und Ergebnis.
- **Je Minute:** aufgenommene, getaktete, verworfene und doppelte Bilder, Länge der Encoder-Warteschlange, Puffergröße im Vergleich zur Prognose, Audio-Abweichung und aufgefüllte Stille je Quelle.
- **Ereignisse:** Wechsel des Aufnahmewegs und das Ergebnis der Hotkey-Registrierung mit Fehlercode.

Alles landet in rotierenden lokalen Logs und im Knopf „Diagnose kopieren“, wobei Pfade unkenntlich gemacht werden.

### Aufnahme und Encoding

- **Puffer aus kodierten Paketen.** Der Puffer speichert H.264-High- und AAC-Pakete mit einem Keyframe alle 1 bis 2 Sekunden. Alte Daten werden immer als ganze Bildgruppe (GOP) verworfen. Eine feste Maximalbitrate hält die RAM-Prognose genau. Der Speicher wird vorab in Blöcken von 1 bis 4 MB reserviert. Beim Speichern werden Blöcke per Referenzzählung festgehalten, nicht kopiert.
- **Aufnahmeweg.** Windows.Graphics.Capture (WGC) nimmt einen Monitor randlos auf. DXGI Desktop Duplication (DDA) bleibt nur, wenn S1 einen Fall findet, in dem WGC versagt und DDA funktioniert.
- **Konstante Bildrate.** WGC liefert ein neues Bild nur, wenn sich etwas ändert, und bis zu 144 bzw. 240 Mal pro Sekunde. Ein Taktgeber auf der GPU verdoppelt oder verwirft Bilder auf einen 60-fps-Takt, abgeleitet von der Audio-Hauptuhr bzw. QPC. Wo die Windows-Version es erlaubt, begrenzt `MinUpdateInterval` die Rate schon bei der Aufnahme. Ohne Taktgeber entstehen eine variable Bildrate, A/V-Versatz, eine falsche RAM-Prognose und ein fälschliches „Puffer nicht voll“.
- **Bild bleibt auf der GPU.** Regel: kein Zurücklesen in den Hauptspeicher, höchstens eine Kopie von GPU zu GPU pro Bild (WGC verwendet seine Texturen wieder). BGRA→NV12 wird auf der GPU umgewandelt. Es gibt ein gemeinsames `ID3D11Device` mit Video-Unterstützung und Multithread-Schutz, das an FFmpegs Hardware-Gerät übergeben wird. QSV-Frames werden aus D3D11 abgeleitet. FFmpeg ab Version 7.1 oder 8.x (prüfen, ab wann h264_mf D3D11-Frames annimmt).
- **Laptops mit zwei GPUs.** Kodiert wird auf der GPU, an der der Monitor hängt. Die Statuszeile zeigt diese GPU an. S2 testet QSV auf der iGPU bei angeschlossener dGPU.
- **Encoder-Reihenfolge.** NVENC / AMF / QSV, dann Media Foundation, dann x264 mit Warnung. Media Foundation bleibt nur, wenn S2 Hardware findet, auf der die Hersteller-Encoder versagen und Media Foundation funktioniert. Für jede Stufe wird festgelegt, was die Statuszeile anzeigt.
- **Formatwechsel.** GameClip skaliert immer auf eine feste Ausgabeauflösung. HDR ist in v1 nicht unterstützt: Die App erkennt HDR und warnt. Bei einem unvermeidbaren Neustart des Encoders (Monitorwechsel, Treiber-Reset, Standby) beginnt ein neuer Abschnitt. Gespeichert wird der neueste Abschnitt; ein Clip über mehrere Abschnitte ergibt mehrere Dateien mit einem Hinweis.
- **Exklusives Vollbild.** Das Verhalten (schwarzes Bild, Wechsel zu DDA, Warnung) wird nach dem Ergebnis von S1 festgelegt.
- **Mindestversion.** Windows 11 ab einem festen Build, z. B. 24H2 (26100). Das muss geprüft werden für `IsBorderRequired` mit `RequestAccessAsync(Borderless)`, für `MinUpdateInterval` und für das Support-Ende älterer Versionen. Unterhalb dieses Builds zeigt die App „Eingeschränkt“.
- **Windows 11 N/KN.** Ohne das Media Feature Pack fehlen H.264/AAC-Wiedergabe und Media Foundation. Die App erkennt das beim ersten Start und erklärt die Abhilfe.

### Audio

- **Spurmodell und Ordner (vom Autor entschieden, Revision 7).**
  - **Aufnahme:** Der Puffer hält immer die Mischung **und** die Einzelspuren (System, Mikrofon, optional Discord). Pro Spur und Minute kostet das bei 128 kbit/s etwa 1 MB RAM.
  - **Clip-Ordner (vom Nutzer gewählt):** Ein Hotkey-Speichern legt dort eine MP4 mit **nur der Mischung** ab. Diese Datei kann man direkt verschicken.
  - **Export-Ordner (vom Nutzer gewählt):** Getrimmte Clips und Größen-Exporte landen dort, ebenfalls **nur mit der Mischung**.
  - **Einzelspuren (interner Speicher, Vorschlag):** Die Einzelspuren werden zusätzlich als kleine Audiodatei (`.m4a`, ~1 MB pro Minute und Spur) in einem App-Ordner abgelegt, z. B. `%LOCALAPPDATA%\GameClip\tracks\`. Das Video wird dort nicht ein zweites Mal gespeichert. Die Datei gehört zu genau einem Clip und wird mit ihm gelöscht.
  - **„Mit getrennten Spuren exportieren“:** Eine manuelle Option im Export-Dialog, Standard ist aus. Sie setzt Video, Mischung und die gespeicherten Einzelspuren zu einer Datei zusammen. Spur 1 bleibt die Mischung, damit auch diese Datei überall vollständig klingt. Dafür müssen zwei Punkte bestätigt werden (Q9).
  - Das MP4-Spurlayout (Reihenfolge, Titel, Standard-Markierung) ist ein fester Dateivertrag, der auch für Linux gilt.
- **Speicherlimit je Ordner (vom Autor entschieden).** Für den Clip-Ordner und den Export-Ordner lässt sich je ein Limit in GB einstellen (z. B. 10 GB). Überschreitet ein neuer Clip das Limit, löscht GameClip die ältesten Clips des Ordners, bis er wieder passt. Gelöscht werden nur Dateien, die GameClip selbst angelegt hat (erkennbar am Metadaten-Tag), nie fremde Dateien im Ordner. Die zugehörigen Einzelspuren werden mitgelöscht. Offene Punkte dazu: Q10.
- **Mikrofon (vom Autor entschieden).** Standardmäßig an. Das Gerät ist wählbar (Standard: Windows-Standardmikrofon). Das Tray zeigt an, dass das Mikrofon aufgenommen wird, und eine Pause-Funktion ist vorhanden.
- **Discord als eigene Spur (vom Autor entschieden, optional).** Per Prozess-Loopback auf den Discord-Prozess (Discord.exe und Unterprozesse). Das betrifft nicht das Spiel und damit nicht Vanguard. Damit Discord in der Mischung nicht doppelt vorkommt, schließt die Spur „System“ Discord aus.
  - Folge: Windows erlaubt pro Loopback-Strom nur einen ausgeschlossenen Prozessbaum (prüfen). Ist Discord ausgeschlossen, kann GameClip nicht zugleich seinen eigenen Bestätigungston ausschließen. Der Ton liegt allerdings zeitlich immer *nach* dem Speichern. Er landet höchstens im nächsten Clip, wenn man innerhalb der Pufferlänge erneut speichert. Lösung (Vorschlag): Der Dienst kennt den Zeitpunkt seines eigenen Tons und schaltet die Spur „System“ für dessen Dauer stumm.
- **Quelle (Vorschlag, gilt wenn die Discord-Spur aus ist).** System-Ton ohne GameClip selbst, per `PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE` mit der PID von gameclipd. So landen der eigene Bestätigungston und andere Geräusche von GameClip nicht im Clip, und eine Spielerkennung ist nicht nötig. Discord-Gespräche und Musik sind weiterhin enthalten.
  - Nur den Ton des Spiels aufzunehmen, kommt später. Dafür wäre ein eigener Vanguard-Test nötig.
  - Prozess-Loopback liefert kein `GetMixFormat` und bei Stille keine Pakete (prüfen). Das Format muss vorgegeben und umgerechnet werden.
  - Kandidat: die Rust-Bibliothek `wasapi` (cpal unterstützt Application-Loopback nicht, prüfen).
- **Die Mischung braucht einen Mixer.** Spur 1 wird live gemischt. Dazu gehören Umrechnung der Abtastrate, Ausgleich von Abweichungen, Summierung mit Limiter und eine Lautstärkeregel. Vorschlag: eine Mikrofon-Lautstärke in den Einstellungen, optional auch für den System-Ton. Ein zu lautes oder zu leises Mikrofon lässt sich sonst nachträglich nicht mehr korrigieren. Das Nicht-Ziel heißt deshalb „keine Mischregler im Editor“.
- **Mikrofon und Datenschutz.** Das Mikrofon ist standardmäßig an (siehe oben). Der erste Start sagt das deutlich und bietet das Abschalten an. Das Tray zeigt den Mikrofonstatus, eine Pause-Funktion ist in v1 enthalten.
- **Synchronität.** Alle Spuren laufen auf einer gemeinsamen Uhr. Stille wird aufgefüllt, und wechselnde Geräte werden neu verbunden. Abnahme: Bild und Ton liegen nach 30 Minuten weniger als ein Bild auseinander.

### Trimmen und Export

- **Wer kodiert neu? (Blocker aus Runde 2, Vorschlag)** gameclipd verwaltet eine Auftragswarteschlange mit fester Höchstzahl gleichzeitiger Aufträge. Jeder Auftrag läuft in einem kurzlebigen Arbeitsprozess mit niedrigerer Priorität, z. B. der mitgelieferten `ffmpeg.exe`.
  - Ein Absturz trifft so weder den Puffer noch die Oberfläche.
  - Die Oberfläche braucht kein FFmpeg.
  - Der laufende Puffer hat Vorrang. Lässt sich keine zweite Hardware-Sitzung öffnen (NVENC-Sitzungslimit bei Consumer-Karten, prüfen), weicht der Auftrag auf x264 mit schnellem Preset aus und warnt.
  - Protokoll: `StartTrimJob`, `StartExportJob`, `JobProgress`, `CancelJob`.
  - Ausgabe als `<Name>_trim.mp4`. Das Original bleibt unverändert.
- **Bildgenauer Schnitt in WebView2.** `<video>` kann nicht Bild für Bild springen, und `currentTime` trifft ein Bild nur ungefähr. Der Dienst berechnet daher die Schnittpunkte als PTS bzw. Bildnummer. Die Oberfläche zeigt per `requestVideoFrameCallback` an und rastet auf die Bildzeitpunkte des Dienstes ein.
  - Damit das Spulen in großen Dateien klappt, muss das Tauri-Protokoll HTTP-Range-Anfragen unterstützen (prüfen).
  - Abnahme: Ein Schnitt bei Bild N ergibt laut ffprobe Bild N als erstes Bild.
- **Ton beim Schnitt.** Der Ton wird beim Trimmen immer neu kodiert, mit FFmpegs eigenem `aac`-Encoder (fdk-aac bleibt verboten). Grund: AAC arbeitet in Blöcken von 1024 Samples und hat eine Anlaufverzögerung.
- **Smart-Cut (Option).** Nur vom Schnittpunkt bis zum nächsten Keyframe neu kodieren (am Ende ebenso) und den Rest unverändert kopieren. Das bleibt bildgenau, ist deutlich schneller und vermeidet Qualitätsverlust. Es ist nicht der verworfene reine Keyframe-Schnitt.
- **Größen-Export (Discord).** Videobitrate = (Zielgröße × 0,92 − Audio − Container) / Dauer, mit begrenzter Maximalrate.
  - Auflösung und Bildrate richten sich nach der Dauer.
  - Nur die Mischspur, mit 96–128 kbit/s.
  - Die Größe wird danach geprüft. Ist die Datei zu groß, folgt ein zweiter Versuch mit niedrigerer Rate. Zwei Durchgänge mit x264 gibt es nur auf dem CPU-Weg.
  - Die Zielgröße ist einstellbar (Standard 10 MB, aktuelles Discord-Limit prüfen) und nicht fest auf „Discord“ verdrahtet.
- **Dateiname.** `<Programmname>_JJJJ-MM-TT_HH-MM-SS.mp4`. Der Programmname kommt vom Prozess des Vordergrundfensters (`GetForegroundWindow` → `QueryFullProcessImageName`), ersatzweise „Clip“. Unerlaubte Zeichen werden ersetzt, bei Namensgleichheit wird `_2` angehängt.

### Hotkey

- **Mechanismus.** RegisterHotKey, immer mit `MOD_NOREPEAT`, damit langes Drücken nicht mehrfach auslöst. F12 und Kombinationen mit der Windows-Taste sind gesperrt.
- **Konflikterkennung, ehrlich formuliert.** RegisterHotKey erkennt nur Apps, die ebenfalls RegisterHotKey nutzen. Discord, Overlays, Makro-Tools und Belegungen im Spiel sieht es nicht. Der Dialog sagt das.
- **Ob das Spiel die Taste noch sieht,** hängt davon ab, wie es Eingaben liest (Raw Input, DirectInput, GetAsyncKeyState). Das wird in S8 getestet, und der Warnungstext richtet sich nach dem Ergebnis.
- **Auswahldialog.**
  - Während der Auswahl gibt gameclipd die Taste frei (`SuspendHotkey`) und prüft die neue Taste (`TestRegister`).
  - Erkannt wird über `KeyboardEvent.code`.
  - Für Tastaturen ohne F13 bis F24 gibt es eine Auswahlliste.
- **Start und erster Start.**
  - Schlägt die Registrierung beim Login fehl, zeigt das Tray „Hotkey nicht aktiv“, und es kommen ein Fehlerton und eine Benachrichtigung.
  - Beim ersten Start zeigt GameClip die aktive Taste mit dem Warnungstext und verlangt einen Testdruck.
  - F8 bleibt Standard.
- **Kandidat:** Rust-Bibliothek `global-hotkey` (prüfen: F13–F24 und `MOD_NOREPEAT`).

### Prozessmodell, Schnittstelle und Plattformgrenze

- **Zwei Programme.**
  - `gameclipd`: reines Rust, ohne WebView. Aufnahme, Puffer, Hotkey, Tray und Auftragswarteschlange.
  - `gameclip-ui`: Tauri mit WebView2, startet nur bei Bedarf.
- **Dienst-Schnittstelle (Blocker aus Runde 2).** Ein eigener Protokoll-Abschnitt im Design-Dokument beschreibt:
  - **Befehle:** Puffer starten/stoppen, Clip speichern, Status, Einstellungen lesen/schreiben, Geräteliste, Encoder-Fähigkeiten mit Grund für „Eingeschränkt“, `SuspendHotkey`, `TestRegister`, Trim-/Export-Aufträge, Diagnose, Mikrofon pausieren.
  - **Ereignisse:** `SaveCompleted`, `SaveFailed`, `StateChanged`, `FormatChanged`, `NewClip`.
  - **Fehlercodes.**
  - **Versionsabgleich:** Dienst und Oberfläche werden zusammen ausgeliefert; die Oberfläche verweigert eine abweichende Hauptversion.
  - **Transport:** Named Pipe, deren Sicherheitsbeschreibung nur die SID des aktuellen Benutzers zulässt.
  - **Eine Instanz:** benannter Mutex.
  - **Fähigkeitsangaben:** z. B. Einzelspuren möglich, maximale Pufferlänge, Prognose verfügbar. So kann der spätere Linux-Adapter melden, was er nicht kann. Statusfelder sind je Adapter optional.
- **Was wirklich gemeinsam ist (korrigiert in Runde 2).** Unter Linux verwaltet gpu-screen-recorder Puffer, Mischung, Spuren und Dateinamen selbst. Gespeichert wird per Signal, und den Pfad meldet er als Textausgabe. Gemeinsam sind deshalb nur Oberfläche, Einstellungsschema, Galerie-Index, Protokoll und das Zustandsmodell, so wie es über das Protokoll sichtbar ist.
  - Puffer, Muxer und Prognose sind Teil des Windows-Kerns. Sie werden als normales Rust geschrieben, ohne Adapter-Abstraktion.
  - Plattformdienste der Oberfläche (Zwischenablage CF_HDROP, OLE-Drag-and-drop, „Im Explorer anzeigen“, Benachrichtigungen mit AUMID) liegen hinter einer kleinen Schnittstelle mit Windows-Umsetzung.
  - Auch Trimmen und Export sind plattformspezifisch.
- **Grenze erzwingen.**
  - Workspace: `gameclip-core`, `gameclip-protocol`, `gameclip-capture-win`, `gameclipd`, `gameclip-ui`, `gameclip-ui-platform`.
  - Ein CI-Job auf ubuntu-latest baut und testet core, protocol und das Frontend gegen ein Attrappen-Backend.
  - Eine Prüfung mit `cargo tree` bzw. cargo-deny bricht ab, wenn `windows` oder `windows-sys` in core oder protocol auftaucht.
- **Kommandozeile.** `gameclip --save-clip` ist in v1 ein internes Entwickler-Werkzeug ohne Stabilitätszusage. Öffentlich wird es erst mit der Linux-Version.
- **Gespeicherte Formate.**
  - Einstellungen mit Schema-Version und Migrationen nur nach vorn; die vorherige Fassung wird gesichert. Bei neuerem Schema werden die Einstellungen nur gelesen, unbekannte Felder bleiben erhalten.
  - Dateivertrag für Clips (Spurlayout und Metadaten-Tag).
  - Die App-ID bestimmt AUMID, Einstellungspfad und Pipe-Namen und lässt sich später nicht ändern. Sie wird vor M1 festgelegt.
- **Zustandsmodell.** Eine Übergangstabelle (Zustand × Ereignis → neuer Zustand, Seiteneffekt, Ton/Benachrichtigung) mit den Zuständen Leerlauf, Startet, Puffert, Speichert, Konfiguriert neu, Eingeschränkt, Erholt sich, Fehler. Neue Fälle:
  - ein laufender Auftrag
  - Hotkey nicht registriert
  - Einzelspuren umgeschaltet
  - Mikrofon pausiert
  - Speichern während „Startet“, „Konfiguriert neu“ oder „Erholt sich“
  - doppelter Tastendruck beim Speichern

  Aus der Tabelle werden die Tests mit Attrappen geschrieben.

### Bedienung und Rückmeldung

- **Ton ist das Hauptsignal.** Ein Ton für Erfolg, ein anderer für Fehler (Puffer leer, Platte voll, Encoder-Fehler, keine Quelle). Die Benachrichtigung ist zweitrangig, weil Fokus-Assistent sie im Vollbild unterdrückt. Für die Benachrichtigung braucht die App eine registrierte AUMID.
- **Langsames Speichern** (z. B. auf HDD) zeigt das Tray als Zustand „Speichert …“ an.
- **Erster Start.** Eine kurze Checkliste:
  1. Mikrofon ja oder nein
  2. Hotkey mit Testdruck
  3. Speicherordner

  Danach eine Statuszeile wie „Puffert 1:00 in 1080p60 über NVENC“ und das Angebot „Jetzt Test-Clip erstellen“.
- **Sonderfälle beim Speichern:**
  - Ist der Puffer noch nicht voll, wird gespeichert, was da ist, mit der tatsächlichen Länge.
  - Bei voller Platte wird die `.part`-Datei gelöscht und der Fehlerton gespielt.
  - Ein doppelter Tastendruck reiht höchstens einen weiteren Speichervorgang ein.
- **Galerie.** Sie zeigt den Inhalt des Speicherordners. Vorschaubilder kommen über die Windows-Shell (`IShellItemImageFactory`), es gibt keinen eigenen Cache. Die Galerie zeigt den belegten Platz an.
- **Teilen.** Datei in die Zwischenablage (CF_HDROP), Drag-and-drop aus der Galerie und „Im Ordner anzeigen“. Getestet wird mit Discord und Explorer.

### Anti-Cheat (Vanguard)

**Status: ausstehend.** Der Nachweis kommt aus S1.

Feste Regeln: keine DLL-Injektion, keine Grafik-Hooks, kein Kernel-Treiber, keine Tastatur-Hooks, kein Lesen von Spielspeicher, signierte Programmdateien.

Testaufbau:
- separates Riot-Konto
- randlos und exklusives Vollbild
- Spiel mit Administratorrechten
- Laptop mit zwei GPUs

Bestanden ist der Test, wenn keine Vanguard-Fehler auftreten, das Bild nicht schwarz ist und das Konto 14 Tage lang nicht gesperrt wird. M1 darf nach 48 Stunden ohne Befund beginnen, die 14-Tage-Prüfung läuft parallel weiter.

Der Test wird vor jedem Release wiederholt (Release-Gate M5). Öffentlich heißt es: „injiziert nichts, getestet mit X am Datum Y“.

### Verteilung und Lizenz

- **Lizenz:** GPL-3.0-or-later.
- **FFmpeg:** fest versionierter GPL-Build (BtbN oder gyan) mit Quellcode-Angebot, ohne `--enable-nonfree` und ohne fdk-aac.
- **Rust-Anbindung:** `rsmpeg` statt `ffmpeg-next`, wegen der Hardware-Frames.
- **Paketformat (vor S1 entscheiden).** Davon hängen randlose Aufnahme, Benachrichtigungen und Autostart ab:
  - MSIX braucht ggf. die Capabilities `graphicsCaptureWithoutBorder` und `graphicsCaptureProgrammatic` (prüfen).
  - Tauri baut nur MSI und NSIS.
  - Eine App ohne Paket braucht eine Startmenü-Verknüpfung mit AUMID, sonst bleiben Benachrichtigungen stumm.
  - Autostart läuft je nach Format über den Run-Schlüssel (MSI) oder StartupTask (MSIX).

  Optionen:
  - (a) MSI/NSIS mit AUMID-Verknüpfung
  - (b) MSI plus Sparse Package für eine App-Identität
  - (c) volles MSIX mit eigenem Build-Schritt

  S1 testet im endgültigen Paketformat.
- **Signatur:** über die SignPath Foundation (kostenlos für Open Source), dazu eine Meldung bei Microsoft Defender gegen Fehlalarme.
- **Updates:** über winget oder eine freiwillige Prüfung, die standardmäßig aus ist. Kein Tauri-Updater, der von sich aus ins Internet geht.

### Wiederverwendbare Bibliotheken (Lizenz und Pflege prüfen)

- **Aufnahme und Windows-APIs:** `windows-capture`, `windows` (windows-rs)
- **Tray, Hotkey, Audio:** `tray-icon`, `global-hotkey`, `wasapi`
- **Zwischenablage und Drag-and-drop:** `clipboard-win` (CF_HDROP), `tauri-plugin-drag`
- **FFmpeg-Anbindung:** `rsmpeg`

### Begriffe

| Begriff | Bedeutung |
| --- | --- |
| WGC | Windows.Graphics.Capture, die Aufnahme-Schnittstelle von Windows für Monitore und Fenster |
| DDA | DXGI Desktop Duplication, ältere Aufnahme des ganzen Bildschirms |
| AUMID | Application User Model ID; Windows ordnet damit Benachrichtigungen einer App zu |
| Prozess-Loopback | Aufnahme des Tons bestimmter Prozesse bzw. aller Prozesse außer bestimmten |
| Randlos | Aufnahme ohne den gelben Rahmen, den Windows sonst um aufgenommene Bereiche zeichnet |
| Eingeschränkt | Zustand, in dem GameClip läuft, aber einen schwächeren Weg nutzt (z. B. CPU-Encoder) |
| Dienst-Schnittstelle | Protokoll zwischen gameclipd und Oberfläche bzw. Kommandozeile |
| GOP | Bildgruppe von einem Keyframe bis zum nächsten |

## Goals vs. Non-Goals

* **In-Scope Goals (Ergebnisse für den Nutzer):**
  * Per Hotkey die letzten 1 bis 5 Minuten als MP4 speichern, innerhalb von 3 s (p95, Definition oben). Siehe D2, D10.
  * Clips, die auf Handy, WhatsApp und Discord vollständig zu hören sind. Siehe D7.
  * Clips in der Galerie ansehen, bildgenau kürzen und auf eine Zielgröße exportieren. Siehe D11, D12.
  * Clips per Zwischenablage, Drag-and-drop und „Im Ordner anzeigen“ teilen.
  * Getrennter Clip- und Export-Ordner, jeweils mit einstellbarem GB-Limit (D14).
  * Mikrofon (Gerät wählbar) und optional Discord als eigene Spuren (D15, D16).
  * Funktioniert mit Valorant/Vanguard. Siehe D3.
  * Verbrauch im Rahmen der Erfolgskriterien (RAM in vier Teilen, CPU ~3 %).
* **Rahmenbedingungen:**
  * Windows 11 ab festgelegtem Build, kein Windows 10 (D9).
  * Offline, ohne Konto, GPL-3.0-or-later (D1).
  * Keine Injektion, keine Hooks, kein Kernel-Treiber (D3).
  * Gemeinsame Teile (Oberfläche, Einstellungen, Protokoll) ohne Windows-Code, per CI geprüft (D6).
* **Out-of-Scope Non-Goals:**
  * Linux in Version 1 (später mit gpu-screen-recorder, Anhang A)
  * Mischregler im Editor (eine Mikrofon-Lautstärke in den Einstellungen ist erlaubt)
  * Nur den Ton des Spiels aufnehmen (später)
  * HDR-Aufnahme (v1 warnt nur)
  * Öffentliche, stabile Kommandozeile (kommt mit Linux)
  * Steam OS, macOS
  * Cloud-Upload, Konten, Social-Funktionen
  * Automatische Highlight-Erkennung, Webcam, Livestreaming
  * Overlay im Spiel
  * Bearbeitung über das Trimmen hinaus

## Alternatives Weighed

| Alternative | Entscheidung | Grund |
| --- | --- | --- |
| RAM-Puffer aus Rohbildern | verworfen (Review) | ~186 MB/s bei 1080p60 |
| Ringpuffer als Datei auf der SSD | zurückgestellt vom Autor | Länge wird über RAM-Prognose begrenzt |
| Code-Injektion („Game Capture“) | verworfen vom Autor | Vanguard blockiert oder meldet sie (D3) |
| Overlay im Spiel | verworfen vom Autor | bräuchte Injektion |
| Electron | verworfen vom Autor | ~100–200 MB mehr RAM (D5) |
| Steam OS in v1 | zurückgestellt vom Autor | Spielmodus kontrolliert Eingaben, eigene Steam-Aufnahme |
| Cloud-Upload | verworfen vom Autor | App soll offline bleiben |
| Fehler statt CPU-Fallback | verworfen vom Autor | x264 mit Warnung (D4) |
| Windows und Ubuntu gleichzeitig in v1 | erst gewählt, dann verworfen vom Autor | zwei getrennte Aufnahmekerne, daher betrifft eine spätere Portierung nur die gemeinsame Schicht; zwei Plattformen vor dem Vanguard-Test wären das größere Risiko (D6) |
| Eigener Linux-Aufnahmekern | verworfen vom Autor | gpu-screen-recorder deckt das ab (Anhang A) |
| Abmischen der Spuren im Editor | verworfen vom Autor | geht über das Ziel hinaus, bräuchte eigene Vorschau-Mischung |
| Nur eine Spur ohne Option auf Einzelspuren | verworfen vom Autor | Einzelspuren bleiben für externe Bearbeitung (D7) |
| Einzelspuren nur per Aufnahme-Einstellung (Review-Vorschlag P1) | verworfen vom Autor | Einzelspuren sollen immer verfügbar sein; die Wahl fällt beim Export (D7) |
| Windows 10 unterstützen | verworfen vom Autor | Support-Ende Oktober 2025 (D9) |
| v1 nur mit Tray, ohne Galerie und Trimmen | verworfen vom Autor | Galerie und Trimmen gehören zum Kern (D11) |
| Hotkey-Standard mit Modifier (Alt+F8) | verworfen vom Autor | Hotkey ist frei belegbar, Warnung im Dialog genügt (D10) |
| Verlustfreier Schnitt nur an Keyframes | verworfen vom Autor | nur auf 1–2 s genau (D12) |
| Windows-Tastatur-Hooks (WH_KEYBOARD_LL) | verworfen (Review) | wirken auf Anti-Cheat wie ein Keylogger (D10) |
| Puffer und Prognose als plattformneutrale Adapter-Abstraktion | verworfen (Review Runde 2) | gpu-screen-recorder verwaltet seinen Puffer selbst, Wiederverwendung findet nie statt |

## Key Decisions & Staging for Next Steps

### Entscheidungsregister

| ID | Entscheidung | Runde |
| --- | --- | --- |
| D1 | Offline, ohne Konto, Open Source unter GPL-3.0-or-later | 1 |
| D2 | Puffer im RAM aus kodierten Paketen, Standard 1 min 1080p60, bis 5 min, RAM-Prognose und Warnung | 1 |
| D3 | Keine Injektion, keine Hooks, kein Kernel-Treiber. Vanguard-Verträglichkeit ist Pflicht und wird in S1 nachgewiesen | 1 |
| D4 | Encoder-Fallback bis x264 mit Warnung | 1 |
| D5 | Stack: Rust-Dienst (gameclipd) und Tauri-Oberfläche (WebView2), FFmpeg mit Hardware-Encodern | 1 |
| D6 | v1 nur Windows. Gemeinsam sind nur Oberfläche, Einstellungen, Galerie-Index, Protokoll und Zustandsmodell; Puffer, Muxer und Prognose gehören zum Windows-Kern. Linux später mit gpu-screen-recorder | 3, präzisiert in Runde 2 |
| D7 | Einzelspuren werden immer aufgenommen. Clip-Ordner und Export-Ordner erhalten immer Dateien nur mit der Mischung. Einzelspuren liegen als Audiodatei im internen App-Ordner. „Mit getrennten Spuren exportieren“ ist eine manuelle Option | 3, geändert Rev. 6 und 7 |
| D8 | Zwei Programme (gameclipd, gameclip-ui) mit abgesicherter, versionierter Named-Pipe-Schnittstelle | 1, bestätigt Runde 2 |
| D9 | Nur Windows 11, kein Windows 10 | 4 |
| D10 | Hotkey per RegisterHotKey mit MOD_NOREPEAT, Standard F8, frei belegbar inklusive F13–F24 und Modifier | 4 |
| D11 | Galerie, Trimmen und „Im Ordner anzeigen“ in v1 | 4 |
| D12 | Trimmen ist bildgenau und kodiert neu, das Original bleibt unverändert | 4 |
| D13 | Bild bleibt auf der GPU: kein Zurücklesen, höchstens eine GPU-zu-GPU-Kopie pro Bild | 1, präzisiert Runde 2 |
| D14 | Clip-Ordner und Export-Ordner getrennt wählbar, jeweils mit GB-Limit; ältester Clip wird gelöscht, nur GameClip-eigene Dateien | Rev. 7 |
| D15 | Mikrofon standardmäßig an, Gerät wählbar | Rev. 7 (ersetzt Teil von P3) |
| D16 | Optionale Discord-Spur per Prozess-Loopback; System-Spur schließt Discord dann aus | Rev. 7 |
| D17 | HDR wird in v1 nicht aufgenommen: GameClip nimmt in SDR auf und warnt, wenn Windows-HDR aktiv ist | Rev. 7 (war P5) |
| D18 | Lautstärkeregler je Quelle (Mikrofon, System, Discord). Standard 100 % = 0 dB, also die unveränderte Originallautstärke. Die Regler wirken nur auf die Mischung, Einzelspuren bleiben unverändert | Rev. 8 (war P3) |
| D19 | Trimmen und Export laufen in einem kurzlebigen Arbeitsprozess, den gameclipd über eine Warteschlange steuert | Rev. 8 (war P4) |
| D20 | Zielgröße des Größen-Exports einstellbar, Standard 10 MB | Rev. 8 (war P7) |
| D21 | Speicherlimit Standard 10 GB je Ordner. Clips lassen sich mit einem Stern schützen und werden dann nie automatisch gelöscht | Rev. 8 (Teil von Q10) |
| D22 | Export „mit getrennten Spuren“: Spur 1 Mischung (Standard), danach die Einzelspuren | Rev. 8 (war Q9) |
| D23 | Kommandozeile `gameclip --save-clip` ist in v1 nur intern, ohne öffentliche Dokumentation und Stabilitätszusage | Rev. 9 (war P6) |
| D24 | Automatisches Löschen ist endgültig, nicht in den Papierkorb. Ist ein Clip größer als das Limit oder belegen geschützte Clips das Limit, wird nichts gelöscht und GameClip warnt | Rev. 9 (war P8) |
| D25 | Spuren werden eindeutig benannt: „Mischung“, „System“, „Mikrofon“, „Discord“ | Rev. 9 (war P9) |
| D26 | GameClips eigener Bestätigungston landet nicht im Clip (Prozess-Loopback mit Ausschluss bzw. Stummschaltung) | Rev. 10 (war P2) |
| D27 | Aufnahme pausiert automatisch, solange der PC gesperrt ist; keine automatische Pause im Akkubetrieb oder außerhalb von Vollbild-Apps | Rev. 10 |

### Vorschläge aus Runde 2, die der Autor bestätigen muss

| ID | Vorschlag |
| --- | --- |
| P1 | ~~Spurmodell mit Aufnahme-Einstellung~~ – ersetzt durch D7 (Revision 6) |
| P2 | ~~System-Ton ohne GameClip selbst~~ – übernommen als D26 |
| P3 | ~~Lautstärke je Quelle~~ – übernommen als D18 |
| P4 | ~~Arbeitsprozess für Trimmen/Export~~ – übernommen als D19 |
| P5 | ~~HDR nur Warnung~~ – übernommen als D17 |
| P6 | ~~Kommandozeile nur intern~~ – übernommen als D23 |
| P7 | ~~Zielgröße einstellbar~~ – übernommen als D20 |
| P8 | ~~Endgültiges Löschen~~ – übernommen als D24 |
| P9 | ~~Spurnamen~~ – übernommen als D25 |

### Offene Fragen

| ID | Frage | Beantwortet durch | Benötigt bis |
| --- | --- | --- | --- |
| Q1 | Konkrete Zahlen für RAM (a–d), CPU, Bildverluste und Qualitätsstufen; Referenzrechner | Autor + S1 | vor S1 |
| Q2 | Paketformat (MSI/NSIS, MSI + Sparse Package, MSIX) und App-ID | Recherche + S1 | vor S1 |
| Q3 | Mindest-Build von Windows 11 | Recherche | vor S1 |
| Q4 | Verhalten bei exklusivem Vollbild; wird DDA gebraucht? | S1 | M1 |
| Q5 | Media Foundation als Zwischenstufe nötig? | S2 | M1 |
| Q6 | Sieht das Spiel die Hotkey-Taste noch (Warnungstext)? | S8 | M2 |
| Q7 | Smart-Cut oder vollständiges Neukodieren beim Trimmen | S9 | M3 |
| Q8 | ~~Teilen des Originals~~ – entfällt, die Dateien im Clip-Ordner enthalten nur die Mischung (D7) | – | – |
| Q9 | ~~Mischung im Spur-Export~~ – entschieden als D22 | – | – |
| Q10 | ~~Schutz und Standard-Limit~~ – entschieden als D21, Rest als P8 | – | – |
| Q11 | Lässt Windows mehrere ausgeschlossene Prozessbäume pro Loopback-Strom zu? Sonst gilt die Stummschaltung des eigenen Tons | Recherche + S4 | M1 |
| Q12 | Zuordnung Clip ↔ interne Einzelspuren: über eine eindeutige ID im MP4-Metadaten-Tag statt über den Pfad, damit Umbenennen oder Verschieben im Explorer die Verbindung nicht zerstört. Was passiert mit verwaisten Einzelspuren, wenn der Nutzer einen Clip im Explorer löscht? Zählen die Einzelspuren zum 10-GB-Limit? | Design-Dokument | M2 |
| Q13 | Sicherheit beim automatischen Löschen: Was gilt als „GameClip-eigene Datei“ (Tag allein oder Tag plus interner Index)? Was passiert, wenn der Nutzer einen Ordner mit fremden Videos als Clip-Ordner wählt, oder Clip- und Export-Ordner auf denselben Ordner legt? | Design-Dokument | M2 |

### Phase-0-Prototypen

| ID | Inhalt | Bestanden wenn |
| --- | --- | --- |
| S1 | WGC (und DDA) → GPU → NVENC → Paketpuffer → MP4 per RegisterHotKey, im endgültigen Paketformat, nach Vanguard-Protokoll | Vanguard-Kriterien erfüllt, Speicherzeit p95 ≤ 3 s, CPU und RAM innerhalb Q1. **No-Go:** Projekt stoppen oder auf DDA wechseln |
| S2 | AMF, QSV, Media Foundation; Laptop mit zwei GPUs (QSV auf der iGPU bei dGPU-Anzeige); HDR-Erkennung | jede Hersteller-GPU kodiert ohne CPU-Kopie. **No-Go:** Hersteller als „Eingeschränkt“ führen |
| S4 | Exclude-Self-Loopback und Mikrofon, Mischung, 30 min Abweichungstest | Versatz < 1 Bild nach 30 min |
| S5 | WebView2: Spulen, Range-Streaming, N-Editionen | Spulen in einem 5-min-Clip ohne Fehler, N-Edition erkannt |
| S6 | Zwischenablage und Drag-and-drop in Discord und Explorer | Datei kommt in beiden an |
| S7 | Signierter Installer auf frischer VM, Virenscanner | keine Warnung von SmartScreen und Defender |
| S8 | Hotkeys: F8 und per SendInput gesendetes F13 unter Vanguard; ein Raw-Input- und ein DirectInput-Spiel | WM_HOTKEY kommt an, Vanguard unauffällig |
| S9 | Trimmen mit Hardware-Encoder bei laufendem Spiel und Puffer | keine Bildverluste im Puffer, Schnitt bei Bild N ergibt Bild N |
| S10 | Wird mit S4 zusammengelegt | – |
| S11 | Genauigkeit des Größen-Exports | 20 Testclips, keiner über der Zielgröße |

S3 (Linux) ist gestrichen und steht in Anhang A.

### Meilensteine

| Meilenstein | Inhalt | Freigabe |
| --- | --- | --- |
| M0 | Prototypen S1, S2, S4, S8, S9 | S1 bestanden (48 h), Wettbewerbsvergleich mindestens 2 von 4 |
| M1 | Dienst ohne Oberfläche, Speichern per Hotkey | Speicherzeit- und RAM-Ziele erfüllt |
| M2 | Tray, Einstellungen, Prognose, Installer mit AUMID, erster Start | Erster Clip in unter N Minuten |
| M3 | Galerie, Trimmen, Größen-Export, Zwischenablage, Drag-and-drop | S5, S6, S11 bestanden |
| M4 | Signatur, Hardware-Testmatrix (S2 erweitert, S7) | alle Hersteller getestet |
| M5 | Öffentliche Beta | Vanguard-Test erneut bestanden |

---

## Anhang A: Später – Linux

Mit gpu-screen-recorder (GPL-3) als Aufnahmekern. Er wird als Unterprozess im Replay-Modus gestartet, und das Speichern wird per SIGUSR1 ausgelöst.

**Gilt weiterhin:**
- Unix-Socket in `$XDG_RUNTIME_DIR`, Modus 0600, Prüfung per SO_PEERCRED.
- Bedienung ohne Tray: Vanilla-GNOME hat keinen Tray, Ubuntu bringt die AppIndicator-Erweiterung mit.
- `gameclip --save-clip` als öffentliche Kommandozeile mit festen Exit-Codes, für manuelle GNOME-Tastenkürzel. Das GlobalShortcuts-Portal gibt es erst ab GNOME 48; prüfen, ob Ubuntu 24.04 mit GNOME 46 es hat.
- Paketformate .deb und Flatpak.
- Dauerhafte Bildschirmfreigabe per Portal-Restore-Token mit Erklärbildschirm.
- Version von gpu-screen-recorder festlegen und Kommandozeilen-Optionen bei Updates testen; klären, ob er mitgeliefert oder vorausgesetzt wird.
- Trimmen und Export unter Linux: ffmpeg-Kommandozeile oder Software-Encoding.

**Überholt durch gpu-screen-recorder:** eigene PipeWire-, DMA-BUF-, VA-API- und NVENC-Anbindung, X11-Aufnahme, WebKitGTK-Vorschau-Erzeugung, Prototyp S3.

## Anhang B: Befunde aus Review-Runde 1 (Status)

| Befund | Status |
| --- | --- |
| Nutzen nicht messbar | offen, siehe Erfolgskriterien und M0 |
| Plattformmatrix zu groß | gelöst (D6, D9) |
| gpu-screen-recorder für Linux | entschieden für später (Anhang A) |
| Galerie und Trimmen verdoppeln Umfang | vom Autor verworfen (D11) |
| RAM-Ziel widerspricht Puffer | gelöst im Prinzip, Zahlen offen (Q1) |
| CPU nicht messbar | Messmethode festgelegt, Zahlen offen (Q1) |
| Speicherzeit undefiniert | gelöst (Definition übernommen) |
| Puffer aus kodierten Paketen | gelöst (D2) |
| Bild bleibt auf der GPU | gelöst (D13) |
| NVIDIA unter Linux | zurückgestellt mit Linux |
| Formatwechsel | Lösung beschrieben, HDR als P5 |
| Getrennte Spuren machen Teilen kaputt | gelöst (D7), Modell als P1 |
| Spiel-Ton vs. System-Ton | Vorschlag P2 |
| Mikrofon-Datenschutz | Vorschlag P3 |
| Prozesse trennen | gelöst (D8) |
| Abgesicherte Schnittstelle | Umfang beschrieben, Detail im Design-Dokument |
| Wayland-Freigabe, Hotkey-Portal, Tray unter Linux | zurückgestellt (Anhang A) |
| Rückmeldung im Spiel unsichtbar | gelöst (Ton als Hauptsignal) |
| Vanguard-Testplan | beschrieben, Ergebnis ausstehend (S1) |
| Paketformat, Signatur, Updates | Signatur und Updates beschrieben, Paketformat offen (Q2) |
| Teststrategie und Diagnose | beschrieben |
