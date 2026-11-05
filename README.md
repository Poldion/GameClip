# GameClip

Clips der letzten Minute per Tastendruck – offen, offline und ohne Konto.

GameClip nimmt im Hintergrund den Bildschirm und den Ton auf und hält die letzten Sekunden in
einem Ringpuffer im Arbeitsspeicher. Ein Druck auf **F8** speichert sie als MP4. Kodiert wird auf
der Grafikkarte (NVENC, AMF, Quick Sync), die Spiele bleiben unberührt.

> **Stand:** Meilenstein M0 (Prototyp). Es gibt den Dienst mit Tray-Symbol, noch keine Galerie
> und keine Einstellungsoberfläche. Siehe [Design](docs/designs/gameclip.md).

## Was GameClip tut – und was nicht

* **Aufnahme:** Windows.Graphics.Capture des Hauptmonitors, Standard 60 s in 1080p60.
* **Ton:** Systemton und Mikrofon auf getrennten Spuren. Im Clip liegt die Mischung, die
  Einzelspuren in einer Begleitdatei. **Das Mikrofon wird standardmäßig mit aufgenommen**, solange
  der Dienst läuft (pausierbar im Tray-Menü). Die Aufnahme pausiert, solange der PC gesperrt ist.
* **Kein Code im Spiel:** GameClip injiziert nichts, setzt keine Hooks und öffnet kein Handle auf
  Spielprozesse. Getestet mit Riot Vanguard (Valorant) am 2026-11-05: Speichern per F8 im Spiel
  funktioniert; der Dauertest ohne Befund (S1) steht noch aus.
* **Offline:** keine Konten, keine Cloud, keine Telemetrie.

## Voraussetzungen

* Windows 11 ab Build 26100 (24H2)
* eine Grafikkarte mit Hardware-Encoder für H.264 (sonst kodiert GameClip über die CPU)

## Installation (Testpaket)

Bis zur signierten Veröffentlichung wird das Paket selbst gebaut und mit einem
Testzertifikat signiert (die Signatur braucht einmal eine UAC-Bestätigung):

```powershell
pwsh packaging/ffmpeg/fetch-ffmpeg.ps1
pwsh packaging/build-msix.ps1 -TestCert
Add-AppxPackage packaging/out/GameClip_0.1.0.0_x64.msix
$p = (Get-AppxPackage io.github.poldion.GameClip).PackageFamilyName
explorer.exe "shell:AppsFolder\$p!Service"
```

Clips landen in `Videos\GameClip`. Das Tray-Menü bietet „Clip speichern“, „Letzten Clip
kopieren“, „Letzten Clip unter 10 MB kopieren“, Pause und Beenden.

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

## Lizenz

GameClip steht unter der [GNU General Public License v3.0 oder später](LICENSE). Das Paket
enthält FFmpeg (GPL-3.0, Quelle siehe [ffmpeg.lock](packaging/ffmpeg/ffmpeg.lock)); die
Lizenzen aller Abhängigkeiten stehen in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).
