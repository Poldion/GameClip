# GameClip

Clips der letzten Minute per Tastendruck – offen, offline und ohne Konto.

GameClip nimmt im Hintergrund den Bildschirm und den Ton auf und hält die letzten Sekunden in
einem Ringpuffer im Arbeitsspeicher. Ein Druck auf **F8** speichert sie als MP4. Kodiert wird auf
der Grafikkarte (NVENC, AMF, Quick Sync), die Spiele bleiben unberührt.

> **Stand:** Meilenstein M3 (Vorabversion). Dienst mit Tray-Symbol, Oberfläche mit erstem Start,
> Galerie, Editor zum Kürzen und Exportieren (auch auf eine Zielgröße wie 10 MB für Discord) und
> Einstellungen. Noch nicht signiert veröffentlicht. Siehe [Design](docs/designs/gameclip.md).

## Was GameClip tut – und was nicht

* **Aufnahme:** Windows.Graphics.Capture eines Monitors (Standard: Hauptmonitor), 15 bis 300 s,
  Standard 60 s in 1080p60.
* **Ton:** Systemton, Mikrofon und auf Wunsch Discord auf getrennten Spuren. Im Clip liegt die
  Mischung, die Einzelspuren in einer Begleitdatei; „Mit getrennten Spuren exportieren“ legt alle
  in eine MP4. **Das Mikrofon wird standardmäßig mit aufgenommen**, solange der Dienst läuft
  (pausierbar im Tray-Menü). Die Aufnahme pausiert, solange der PC gesperrt ist.
* **Galerie und Editor:** Clips und Exporte mit Stern gegen das automatische Löschen, bildgenaues
  Kürzen, Export als Mischung, mit getrennten Spuren oder auf eine Zielgröße; Kopieren in die
  Zwischenablage und Ziehen nach Discord oder in den Explorer.
* **Speicherlimit:** Ist der Clip-Ordner voll, löscht GameClip die ältesten eigenen Clips ohne
  Stern endgültig. Fremde Dateien im Ordner fasst es nicht an.
* **Kein Code im Spiel:** GameClip injiziert nichts, setzt keine Hooks und öffnet kein Handle auf
  Spielprozesse. Getestet mit Riot Vanguard (Valorant) am 2026-11-05: Speichern per F8 im Spiel
  funktioniert; der Dauertest ohne Befund (S1) steht noch aus.
* **Offline:** keine Konten, keine Cloud, keine Telemetrie.

## Voraussetzungen

* Windows 11 ab Build 26100 (24H2)
* eine Grafikkarte mit Hardware-Encoder für H.264 (sonst kodiert GameClip über die CPU)

## Installation

Bis zur Signatur über SignPath sind die
[Releases](https://github.com/Poldion/GameClip/releases) Vorabversionen mit einem
Testzertifikat. Am einfachsten: `GameClip-Setup.exe` aus dem neuesten Release laden und starten.
Windows SmartScreen warnt dabei vor einem unbekannten Herausgeber („Weitere Informationen“ →
„Trotzdem ausführen“). Beim ersten Mal fragt Windows nach Administratorrechten, um
`GameClip-Release.cer` unter „Vertrauenswürdige Personen“ einzutragen; danach installiert das
Setup das Paket für den angemeldeten Benutzer. Ohne Setup geht es auch mit den übrigen Dateien
und `Install-GameClip.ps1` (Rechtsklick → „Mit PowerShell ausführen“).

Neue Versionen gibt es über Einstellungen → „Nach Updates suchen“. GameClip geht nur bei diesem
Klick ins Internet.

### Selbst bauen

Das Paket lässt sich auch selbst bauen und mit einem eigenen Testzertifikat signieren (die
Signatur braucht einmal eine UAC-Bestätigung):

```powershell
pwsh packaging/ffmpeg/fetch-ffmpeg.ps1
pwsh packaging/build-msix.ps1 -TestCert
Add-AppxPackage packaging/out/GameClip_0.1.0.0_x64.msix
```

Ein Release entsteht durch einen Tag `vX.Y.Z` (Workflow `release.yml`).

Danach „GameClip“ im Startmenü öffnen. Die Oberfläche startet den Dienst und führt durch den
ersten Start (Mikrofon, Hotkey, Ordner, Test-Clip). Clips landen in `Videos\GameClip`, Exporte in
`Videos\GameClip Export`, jeweils in einem Unterordner pro Spiel (etwa `Videos\GameClip\VALORANT`).
Das Tray-Menü bietet „Clip speichern“, „Letzten Clip kopieren“,
„Letzten Clip unter 10 MB kopieren“, Pause, „Galerie öffnen“, „Einstellungen“ und Beenden.

## Bauen

* Rust (Version laut `rust-toolchain.toml`), Visual Studio Build Tools mit dem Windows-SDK,
  LLVM (für die FFmpeg-Bindings) und PowerShell 7.
* `pwsh packaging/ffmpeg/fetch-ffmpeg.ps1` lädt den gepinnten FFmpeg-Build
  ([ffmpeg.lock](packaging/ffmpeg/ffmpeg.lock)), prüft die SHA-256 und erzeugt die Bindings.
* Tests brauchen die FFmpeg-DLLs im `PATH`:

  ```powershell
  $env:PATH = "$PWD\third_party\ffmpeg\bin;$env:PATH"
  cargo test --workspace --all-features
  ```

  Einige Tests laufen auf der echten GPU und nehmen kurz den Bildschirm auf; die Dateien werden
  danach gelöscht.
* Oberfläche (Node.js 24, in `apps/gameclip-ui`):

  ```powershell
  npm ci
  npm test            # Tests gegen die Attrappe ipc.mock.ts
  npm run dev:mock    # Oberfläche im Browser mit Beispieldaten, ohne Dienst (http://localhost:1420/?ready)
  npm run build       # dist/, wird von cargo build -p gameclip-ui eingebettet
  ```

## Code signing policy

Releases werden in GitHub Actions aus diesem Repository gebaut und signiert
([`release.yml`](.github/workflows/release.yml)); niemand signiert Dateien von Hand. Die Signatur
über die [SignPath Foundation](https://signpath.org) ist beantragt. Bis dahin sind die Releases
Vorabversionen mit einem eigenen Testzertifikat ([`GameClip-Release.cer`](packaging/GameClip-Release.cer)).

* **Autoren** (ändern den Quellcode ohne weitere Prüfung): [Poldion](https://github.com/Poldion)
* **Prüfer** (prüfen Beiträge von außen, etwa Pull Requests): [Poldion](https://github.com/Poldion)
* **Freigabe** (gibt jede Signatur frei): [Poldion](https://github.com/Poldion)

Mitgeliefert und nicht selbst signiert werden die FFmpeg-DLLs aus dem gepinnten
GPL-Build ([ffmpeg.lock](packaging/ffmpeg/ffmpeg.lock)).

**Datenschutz:** GameClip überträgt keine Daten an andere Rechner, außer wenn der Benutzer es
ausdrücklich anfordert. Das ist nur „Nach Updates suchen“: Dieser Klick fragt
`api.github.com` nach den Releases dieses Repositorys, „Herunterladen und installieren“ lädt das
Paket von `github.com`. Dabei gelten die
[Datenschutzbestimmungen von GitHub](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement).
Clips, Einstellungen und Logs bleiben auf dem Rechner.

## Lizenz

GameClip steht unter der [GNU General Public License v3.0 oder später](LICENSE). Das Paket
enthält FFmpeg (GPL-3.0, Quelle siehe [ffmpeg.lock](packaging/ffmpeg/ffmpeg.lock)); die
Lizenzen aller Abhängigkeiten stehen in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)
(Rust) und [THIRD_PARTY_LICENSES_JS.md](THIRD_PARTY_LICENSES_JS.md) (Oberfläche).
