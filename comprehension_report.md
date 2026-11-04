**What this proposes**

GameClip v1 ist eine kostenlose Offline-App für Windows 11, mit der man Clips aus Spielen erstellt. Sie nimmt ständig Bildschirm und Ton auf: System, Mikrofon und optional Discord. Die letzten 15 bis 300 Sekunden (Standard 60 s) hält sie als bereits kodierte H.264/AAC-Pakete in einem Ringpuffer im RAM.

Drückt der Nutzer den Hotkey (Standard F8), schreibt die App innerhalb von 3 Sekunden eine MP4 in den Clip-Ordner. Diese Datei enthält nur die gemischte Audiospur. Die Einzelspuren landen in einer internen `.m4a`. Mit dem Clip verbunden ist sie über einen UUID-Metadaten-Tag und einen SQLite-Index.

Für die Aufnahme nutzt GameClip nur Windows-Schnittstellen, die keinen Code in Spiele einschleusen: WGC, Prozess-Loopback und RegisterHotKey. So bleibt die App mit Riot Vanguard verträglich, dem Anti-Cheat von Valorant.

Es gibt drei Programme:

* **`gameclipd`** ist ein Hintergrunddienst. Er übernimmt Aufnahme, Puffer, Hotkey, Tray-Symbol, Index, das Löschen nach Speicherlimit und eine Auftragswarteschlange.
* **`gameclip-ui`** ist die Oberfläche, gebaut mit Tauri und React. Sie bietet Galerie, bildgenaues Kürzen, Export (nur Mischung, getrennte Spuren oder Zielgröße wie 10 MB für Discord) und Teilen per Zwischenablage oder Drag-and-drop.
* **`gameclip-worker`** ist ein kurzlebiger Prozess, der für Kürzen und Export neu kodiert.

Dienst und Oberfläche sprechen über eine Named Pipe je Benutzer, mit JSON-Nachrichten und Längenpräfix. Ausgeliefert wird ein signiertes MSIX-Paket. Kernlogik und Protokoll enthalten keinen Windows-Code. So kann eine spätere Linux-Version sie auf Basis von gpu-screen-recorder wiederverwenden.

Prototypen in Phase 0 vergleichen GameClip mit Game Bar und OBS (Ziel G1), bevor die Oberfläche gebaut wird.

**How the existing system works, per this document**

Es gibt noch kein GameClip-System. Das Repository ist leer, und alle Dateien werden neu angelegt (Zeile 705).

Laut Zeilen 25–38 nutzen Spieler heute Medal, die Replay-Funktionen von NVIDIA oder AMD, Xbox Game Bar oder den OBS Replay Buffer. Jede dieser Lösungen hat einen Haken: Konto und Cloud, Bindung an einen Hersteller, schwaches Kürzen und Teilen oder eine umständliche Einrichtung. Der übliche Ablauf (speichern, kürzen, unter 10 MB bringen, in Discord einfügen) braucht zwei oder drei Programme.

Vanguard läuft im Kernel und blockiert oder meldet Code-Injektion. Aufnahme über Hooks oder Injektion scheidet damit aus.

---

# Verständnisbericht – Stufe 6 (Verständnistest)

*   **Dokument:** [docs/designs/gameclip.md](docs/designs/gameclip.md)
*   **FeatureRange:** Zeilen 1–653 · **DetailedRange:** Zeilen 654–Ende
*   **Datum:** 2026-11-04
*   **Ergebnis:** 0 Blocker, 5 Major, 11 Nits. Jeder Abschnitt ließ sich in eigenen Worten wiedergeben. Die Major-Befunde betreffen Stellen, an denen die Teile nicht zusammenpassen, keine undefinierten Begriffe.

## Major

### 1. Die Prototypen werden im Feature-Teil vorausgesetzt, aber nie erklärt

*   **Stelle:** Zeilen 40, 44, 46, 58, 84–88, 450 gegen die Tabelle in Zeilen 1247–1258 und Zeilen 693–696
*   **Problem:**
    *   Der Feature-Teil stützt sich auf die Prototypen S1, S2, S4–S9, S11 und auf eine „Phase 0“. Was sie sind, sagt er nie.
    *   Offene Fragen werden „aus S1/S2/S8/S9 beantwortet“, Ziele „in Phase 0“ oder „im Prototyp S1“ gemessen.
    *   Beschrieben werden die S-Tests nur in der Tabelle mit den Bestehenskriterien ganz am Ende, und auch dort ohne Zweck je Test.
    *   „Phase 0“ taucht später nicht mehr auf. Dass sie M0 meint, muss der Leser erraten.
    *   S3 und S10 fehlen in der Nummerierung ohne Erklärung.

### 2. Die Tests, die Q4, Q5 und Q7 beantworten sollen, prüfen diese Fragen nicht

*   **Stelle:** Q4, Q5, Q7 (Zeilen 85, 86, 88) und Zeile 303 gegen die Kriterien von S1, S2, S9 (Zeilen 1249, 1250, 1256)
*   **Problem:**
    *   **Q4** (exklusives Vollbild, wird DDA gebraucht?) soll S1 beantworten. Die Kriterien von S1 decken Vanguard, Speicherzeit, CPU und RAM ab, aber kein exklusives Vollbild.
    *   **Q5** (Nutzen von Media Foundation) soll S2 beantworten. S2 erwähnt weder Media Foundation noch einen versagenden Hersteller-Encoder.
    *   **Q7** (Smart-Cut oder vollständiges Neukodieren) soll S9 beantworten. S9 prüft verworfene Bilder und Bildgenauigkeit, nicht ob Smart-Cut sich lohnt.

### 3. Drei verschiedene Lesarten für gleichzeitiges Speichern

*   **Stelle:** Zeilen 232, 355–358, 814, 815, 1037, 1077
*   **Problem:**
    *   Zeile 232: Ein Speichervorgang läuft, höchstens einer wird eingereiht.
    *   Zeile 814: Ein Zähler `saves_in_flight` (0, 1, 2) „mit Warteschlange“.
    *   Zeile 1037: Jeder Vorgang hat einen eigenen Thread, „höchstens zwei gleichzeitig“.
    *   Die Zustandstabelle führt „Speichert“ als Zustand. Zeile 814 sagt, Speichern ist kein Zustand, Zeile 815 sagt aber, es setzt „genau die Übergangstabelle“ um. Zeile 1077 führt „Speichert“ zusätzlich als Tray-Zustand.

### 4. Die Zustandsmaschine hat keinen Ausgang aus „Fehler“ und „Eingeschränkt“

*   **Stelle:** Zeilen 349–367, CUJ 6 (Zeile 129)
*   **Problem:** Es fehlen Übergänge aus „Fehler“ (nach 5 Fehlversuchen) und aus „Eingeschränkt“, z. B. wenn der Nutzer laut CUJ 6 eine freie Taste wählt oder der Encoder wieder verfügbar ist. Unklar ist, ob ein Neustart nötig ist, ob eine neue Taste zurück nach „Puffert“ führt und was „Fehler“ über Zeile 367 hinaus für die Export-Warteschlange heißt. CUJ 6 setzt eine Erholung ohne Neustart voraus, aber kein Übergang erlaubt sie.

### 5. Unklar, wie der Dienst startet, wenn Autostart aus ist

*   **Stelle:** Zeilen 19, 406, 444, 446, 1144
*   **Problem:** `gameclip-ui` startet `gameclipd` nicht (Zeile 1144). Im Startmenü steht nur `gameclip-ui`, und Autostart lässt sich abschalten. Das Dokument sagt nicht, wie der Dienst startet, wenn Autostart aus ist oder der Nutzer ihn über „Beenden“ im Tray geschlossen hat. Es sagt auch nicht, was die Oberfläche in diesem Fall anzeigt.

## Nits

| Nr. | Stelle | Problem |
| --- | --- | --- |
| 6 | Z. 44 gegen Z. 658 | Zeile 44 sagt, die Zahlen werden *vor* S1 festgelegt; Zeile 658 sagt, sie werden *nach* S1 bestätigt. |
| 7 | Z. 645, 736, 795, 900, 976, 1003, 1022, 1028 | Verweise auf D1, D2, D13, D16, D23, D25 und „Anhang A des Briefs“ sind oft die einzige Begründung, ohne den Inhalt kurz zu nennen (z. B. „D23 im Brief“ in Zeile 1028). |
| 8 | Z. 82–91 | Q8–Q10 fehlen in der Nummerierung ohne Hinweis, ob sie gelöst oder gestrichen sind. |
| 9 | Z. 85, 150, 177, 194–195 | WGC und DDA werden nie ausgeschrieben (Windows Graphics Capture, Desktop Duplication API). Warum DDA ein Ausweichweg wäre, wird nicht erklärt. |
| 10 | Z. 55 | „höchstens 60 bis 80 MB“ ist als Grenze unklar; an anderer Stelle steht ≤ 80 MB. |
| 11 | Z. 224, 228, 839 | Drei verschiedene Reserven (15 % Pool, ×1,05, +1 s). Unklar, welche Zahl die Oberfläche zeigt und welche reserviert wird. G3 („±5 %“) hängt davon ab. Zeile 839 zählt immer drei 128-kbit/s-Spuren, auch wenn Discord aus ist. |
| 12 | Z. 668 | „60 % bzw. 60 %“ für 720p und 30 fps; unklar, wie 720p30 (beides zusammen) berechnet wird. |
| 13 | Z. 303, 305, 528, 646 | Unklar, ob Smart-Cut in v1 gehört: Zeile 303 nennt einen zweiten Worker-Modus, Zeilen 528 und 646 eine spätere Verbesserung. |
| 14 | Z. 322, 1073, 1193 | F12 ist ohne Begründung gesperrt (vermutlich, weil Windows die Taste für den Debugger reserviert). |
| 15 | Z. 204, 225, 234, 362 | „Abschnitt“ im Ringpuffer ist nicht definiert. Die sichtbare Folge fehlt: Nach einem Geräteverlust lässt sich nichts von davor mehr speichern. |
| 16 | Z. 1255 | S8 prüft „per SendInput gesendetes F13“, ohne zu erklären, warum synthetische Eingaben wichtig sind (vermutlich Makro-Tasten oder Stream Deck). |
