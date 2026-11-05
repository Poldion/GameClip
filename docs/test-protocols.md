# Testprotokolle

Manuelle Prototypen und Abnahmen aus dem [Design](designs/gameclip.md#verification-plan). Jeder
Abschnitt hat Aufbau, Schritte, Kriterien und einen Ergebnisblock. Mit *vorläufig* markierte
Grenzwerte werden nach S1 auf dem Referenzrechner festgelegt (Q1).

## Referenzrechner (vorläufig)

| | |
| --- | --- |
| CPU | AMD Ryzen 5 3600X (6 Kerne) |
| RAM | 48 GB |
| GPU | AMD Radeon RX 6750 XT, Treiber 32.0.21045.16010 |
| Monitore | 2560 × 1440 (primär), 1080 × 1920 |
| Windows | 11 Pro 26H2, Build 26300 |
| Referenzspiel | Valorant (Riot Vanguard) |

Für S2 fehlen noch ein Rechner mit NVIDIA-GPU, einer mit Intel-GPU (QSV) und ein Laptop mit
zwei GPUs.

## S1 – Dienst, Vanguard, Leistung (M0)

**Aufbau:** Referenzrechner, MSIX mit Testzertifikat, Standardeinstellungen, Valorant.

**Schritte:**
1. Paket installieren, Autostart einschalten, neu anmelden.
2. Valorant 48 h Spielzeit verteilt über 14 Tage, dabei regelmäßig F8.
3. Im Spiel randlos und im exklusiven Vollbild je 10 Clips speichern und ansehen.
4. CPU und Private Bytes des Dienstes 10 min im Spiel messen (Leistungsmonitor).
5. `gameclipd.exe --save-clip` aus dem Paketordner mit `Start-Process -Wait` ausführen.

**Kriterien:** kein Vanguard-Befund in 48 h, keine Sperre in 14 Tagen; kein schwarzes Bild
(sonst DDA, Q4); Speicherzeit p95 ≤ 3 s; CPU ≤ 3 % *(vorläufig)*; ≤ 80 MB Private Bytes plus
Puffer *(vorläufig)*; Autostart, Pipe, Loopback, randlose Aufnahme und Toast funktionieren.

**Ergebnis:**

| Datum | Build | Ergebnis |
| --- | --- | --- |
| 2026-11-05 | 0.1.0.0/0.1.0.1 | Teilweise: Installation, Start, Tray, F8 in Valorant, Ton, Speichern (60-s-Clips in 170–280 ms). Toast wird zugestellt, Banner im Vollbildspiel unterdrückt (Windows „Nicht stören“). Offen: 48-h-Test, Vollbild exklusiv, CPU/RAM, Autostart nach Neuanmeldung, gelber Rahmen. |

## S2 – Encoder auf allen Herstellern (M0)

**Aufbau:** je ein Rechner mit NVIDIA, AMD und Intel; ein Laptop mit zwei GPUs; ein Monitor mit HDR.

**Schritte:** `cargo test -p gameclip-media --test encoder_gpu -- --nocapture` und
`cargo test -p gameclip-capture-win --test video_capture -- --nocapture`; HDR ein- und
ausschalten; `dumpbin /dependents` aller FFmpeg-DLLs im Paket.

**Kriterien:** NVENC, AMF und QSV kodieren ohne CPU-Kopie; IDR auf Anforderung bei allen drei;
Laptop wählt die GPU des Monitors; HDR wird erkannt; keine FFmpeg-DLL hängt von `mfplat.dll` ab;
dokumentiert, ob nur `h264_mf` irgendwo funktioniert (Q5).

| Datum | Hardware | Ergebnis |
| --- | --- | --- |
| 2026-11 | RX 6750 XT | AMF: D3D11-Frames, IDR auf Anforderung, Keyframe je Sekunde erst durch eigenes IDR (`usage lowlatency` ignoriert `gop_size`). Keine Abhängigkeit von `mfplat.dll`. NVIDIA, Intel, zwei GPUs, HDR: offen. |

## S4 – Ton (M0 / M2)

**Kriterien M0:** A/V-Versatz < 1 Bild nach 30 min; eigener Ton nicht im nächsten Clip;
Mischung und `.m4a` auf ±1 Sample deckungsgleich. **M2:** Discord-Spur getrennt, nicht doppelt.

**Schritte:** 30 min aufnehmen mit einem Taktsignal (Bild und Ton gleichzeitig), Clip speichern,
Versatz messen; `cargo test -p gameclip-capture-win --test audio_capture -- --ignored` auf einem
ruhigen Rechner mit Lautsprechern.

| Datum | Ergebnis |
| --- | --- |
| 2026-11 | Mischung und `.m4a` deckungsgleich (gleicher Start, gleiche Länge in Samples, Test `save`). Testton per Loopback bei laufendem Spiel nur in 1 von 3 Läufen erkannt – Ursache offen. 30-min-Versatz offen. |
| 2026-11-05 | M2, automatisch: Mit Discord-Spur liefert die Pipeline vier Spuren auf derselben Zeitachse (Test `records_discord_track_on_the_same_timeline`, Discord lief, INCLUDE auf Discord und EXCLUDE von Discord aus „System“ starten ohne Fehler). Hörprobe offen. |

**Schritte M2 (Discord):** In den Einstellungen „Discord als eigene Spur aufnehmen“ einschalten.
In einem Discord-Sprachkanal jemanden sprechen lassen, gleichzeitig ein Spiel mit Ton laufen
lassen, Clip speichern und mit „Mit getrennten Spuren exportieren“ exportieren. Prüfen: Die
Stimme ist in „Discord“ und in der Mischung, aber **nicht** in „System“. Die Spiel-Geräusche
stehen in „System“, aber nicht in „Discord“. Dann Discord beenden und neu starten (innerhalb von
2 s wird neu gebunden), noch einmal speichern. Der Bestätigungston darf dabei in keiner Spur zu
hören sein.

## S6 – Teilen (M0)

**Schritte:** Tray → „Letzten Clip kopieren“, in Discord und im Explorer einfügen; Tray →
„Letzten Clip unter 10 MB kopieren“, in Discord einfügen.

| Datum | Ergebnis |
| --- | --- |
| 2026-11-05 | „Unter 10 MB kopieren“ → Einfügen in Discord funktioniert. Explorer offen. |

## S8 – Hotkey (M0)

**Schritte:** F8 in Valorant (Raw Input) und einem DirectInput-Spiel; F13 per SendInput (z. B.
Makro-Software) im Spiel.

| Datum | Ergebnis |
| --- | --- |
| 2026-11-05 | F8 in Valorant löst aus. In der Entwicklungsumgebung lösen per SendInput eingespeiste Tasten kein `WM_HOTKEY` aus (auch nicht im minimalen Gegentest) – mit echter Makro-Software zu prüfen. |

## S9 – Kürzen und Zielgröße bei laufendem Spiel (M0)

**Kriterien:** keine verworfenen Bilder im Puffer; Bildrate des Spiels sinkt ≤ 10 %
*(vorläufig)*; Schnitt bei Bild N ergibt Bild N; 60-s-Clip in ≤ 15 s *(vorläufig, Q7)*.

| Datum | Ergebnis |
| --- | --- |
| 2026-11-05 | 60-s-Clip auf 10 MB in 14,4–16,5 s bei laufendem Valorant (knapp). Bild N = Bild N: Test `trim`. Bildrate des Spiels und verworfene Bilder offen. |
| 2026-11-05 | Nach Beschleunigung ohne Spiel: 7,2 s (vorher 10,7 s); Grenze ist der Hardware-Decoder (5,4 s). Mit Spiel neu zu messen. |

## G1a – Vergleich mit Game Bar und OBS (Ende M0)

**Größen:** 1. Minuten von der Installation bis zum ersten Clip; 2. RAM und CPU im Leerlauf mit
Puffer und Spiel; 3. Einstellungen bis zu einem brauchbaren Clip; 4. Zeit vom Hotkey bis zu einer
Datei unter 10 MB in der Zwischenablage.

**Schritte für Größe 4:** F8 im Spiel, sofort Tray → „Letzten Clip unter 10 MB kopieren“; die
Dauer steht im Log („G1a: Tray-Aktion bis Zwischenablage … ms“) plus die Zeit bis zum Klick.

| Werkzeug | Größe 2 | Größe 4 |
| --- | --- | --- |
| GameClip 0.1.0.1 | offen | ≈ 0,3 s Speichern + Klick + 14–17 s Export (60 s) |
| Xbox Game Bar | offen | offen |
| OBS Replay Buffer | offen | offen |

## S5, S7, S11, G1b

Ab M3 bzw. M4; Schritte und Kriterien siehe Design.
