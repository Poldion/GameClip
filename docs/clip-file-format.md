# Dateiformat der Clips

Fester Vertrag für Clips, Begleitdateien und Exporte (D25). Andere Programme dürfen sich darauf
verlassen; Änderungen brauchen eine neue `gameclip_version`-Regel.

## Clip (`<Clip-Ordner>\<Spiel>\<Programm>_JJJJ-MM-TT_HH-MM-SS.mp4`)

* **Video:** H.264 High, geschlossene Bildgruppen, ein Keyframe je Sekunde, keine B-Frames.
  Spurtitel (`handler_name`) `Video`.
* **Ton:** eine Spur AAC-LC, 48 kHz Stereo, 160 kbit/s, Titel `Mischung`, Standardspur.
* **Nullpunkt:** t0 = erster Keyframe. Die AAC-Anlaufzeit (1024 Samples) blendet eine Edit-List
  aus; Bild und Ton beginnen bei 0.
* **Tags** (geschrieben mit `movflags +use_metadata_tags`):
  * `gameclip_id` – UUID; Kopien behalten sie, deshalb nicht eindeutig
  * `gameclip_created` – RFC 3339 in UTC
  * `gameclip_version` – Version des Dienstes
* **Name:** `<Programm>` ist der Name der Programmdatei im Vordergrund ohne `.exe` (bereinigt,
  höchstens 64 Zeichen); ohne Vordergrundprogramm entfällt er. Bei Namensgleichheit `_2`, `_3`, …
* Geschrieben als `.part`, gesichert (`FlushFileBuffers`), dann umbenannt
  (`MOVEFILE_WRITE_THROUGH`).

## Einzelspuren (`%LOCALAPPDATA%\GameClip\tracks\<gameclip_id>.m4a`)

* Spuren AAC-LC 48 kHz Stereo, 128 kbit/s, ohne Lautstärkeänderung, in dieser Reihenfolge:
  `System` (Standardspur), `Mikrofon`, `Discord` – jeweils nur, wenn die Quelle aktiv war.
* Tag `gameclip_id` wie der Clip; gleicher Nullpunkt, also auf ±1 Sample deckungsgleich mit der
  Mischung.
* Im MSIX-Paket leitet Windows den Ordner nach
  `%LOCALAPPDATA%\Packages\<Paketfamilie>\LocalCache\Local\GameClip\tracks` um.

## Exporte (`<Export-Ordner>\<Originalname>_<Suffix>.mp4`)

| Profil | Suffix | Ton |
| --- | --- | --- |
| Mischung | `_trim` | `Mischung` 160 kbit/s |
| Getrennte Spuren | `_tracks` | `Mischung` (Standard), `System`, `Mikrofon`, `Discord` |
| Zielgröße | `_<N>MB` | `Mischung` 96 kbit/s; Auflösung und Bildrate ggf. reduziert |

Exporte tragen eine **neue** `gameclip_id`, `gameclip_created` und `gameclip_version`, sind mit
`+faststart` geschrieben und beginnen bei Bild `in` (Ende ausschließlich).
