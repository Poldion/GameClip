# GameClip-Protokoll (Version 1)

Oberfläche und Dienst (`gameclipd`) sprechen über eine Named Pipe miteinander. Dieses Dokument
ist der Vertrag dafür, auch für eine spätere Linux-Version. Die Typen stehen in
`crates/gameclip-protocol/src/messages.rs`; ein Test (`crates/gameclip-protocol/tests/roundtrip.rs`)
liest jedes JSON-Beispiel in diesem Dokument gegen diese Typen ein.

## Verbindung

* Pipe-Name: `\\.\pipe\io.github.poldion.GameClip-<SID>`, `<SID>` ist die SID des angemeldeten
  Benutzers (`S-1-5-21-…`). Nur dieser Benutzer hat Zugriff; Verbindungen von anderen Rechnern
  werden abgewiesen.
* Höchstens 4 Verbindungen gleichzeitig.
* Läuft schon ein Dienst, scheitert ein zweiter beim Anlegen der Pipe und beendet sich.

## Rahmung

Jede Nachricht ist ein Rahmen: 4 Byte Länge (u32, Little Endian), danach so viele Byte UTF-8-JSON.
Ein Rahmen ist höchstens 8 000 000 Byte groß. Ist er größer, trennt die Gegenseite die Verbindung.

## Hülle

Eine **Anfrage** hat eine `id` (u32, zählt je Verbindung hoch), den Befehl in `type` und dessen
Felder in `body`. Befehle ohne Felder haben kein `body`.

```json
{"id": 2, "type": "GetStatus"}
```

```json
{"id": 3, "type": "SetStar", "body": {"clip_id": 7, "on": true}}
```

Eine **Antwort** trägt dieselbe `id`. Bei Erfolg ist `ok` wahr und `body` enthält den Antworttyp
(`<Befehl>Reply`), sonst ist `ok` falsch und `error` beschreibt den Fehler.

```json
{"id": 3, "ok": true, "body": {}}
```

```json
{"id": 3, "ok": false, "error": {"code": "NotFound", "message": "Clip 7 gibt es nicht"}}
```

Ein **Ereignis** kommt ungefragt, hat keine `id`, den Namen in `event` und die Felder in `body`.
Ereignisse und Antworten kommen über dieselbe Verbindung; der Client unterscheidet sie am Feld
`event`.

```json
{"event": "NewClip", "body": {"clip_id": 12, "folder": "clips"}}
```

Unbekannte Felder werden ignoriert. Ein unbekannter `type` wird mit `UnknownCommand` beantwortet;
die Verbindung bleibt offen.

## Handshake und Versionsregel

Die erste Nachricht des Clients ist `Hello`. Oberfläche und Dienst kommen im selben Paket, deshalb
gibt es keine Aushandlung: Ist `protocol` ungleich der Version des Dienstes, antwortet er
`VersionMismatch` und schließt die Verbindung. Ist die erste Nachricht kein `Hello`, antwortet er
`InvalidArgument` und schließt ebenfalls.

```json
{"id": 1, "type": "Hello", "body": {"protocol": 1, "client": "gameclip-ui"}}
```

```json HelloReply
{"id": 1, "ok": true, "body": {"protocol": 1, "service_version": "0.1.0"}}
```

```json
{"id": 1, "ok": false, "error": {"code": "VersionMismatch", "message": "Protokoll 2, Dienst 1"}}
```

## Typ-Regeln

* Zeiten sind `u64` in Mikrosekunden Anzeigezeit, Felder enden auf `_us`.
* Größen sind `u64` in Byte, Felder enden auf `_bytes`.
* Fortschritt ist `percent: f32` von 0,0 bis 100,0.
* `clip_id` ist die Zeilennummer im Clip-Index; sie bleibt gleich, solange der Clip existiert.
* `folder` ist `"clips"` oder `"exports"`.
* Datumsangaben sind RFC 3339 in UTC.

## Befehle

| Befehl | Body | Antwort-Body |
| --- | --- | --- |
| `Hello` | `{ protocol, client }` | `HelloReply { protocol, service_version }` |
| `GetStatus` | – | `ServiceStatus` (siehe unten) |
| `GetSettings` | – | `{ settings: Settings, read_only }` |
| `SetSettings` | `{ partial }` (JSON-Merge-Patch, RFC 7386, auf `Settings`) | `{ settings }` |
| `GetForecast` | `{ settings }` | `Forecast` (geschätzter Speicherbedarf des Puffers) |
| `ListDevices` | – | `{ monitors: [{ id, name, primary, width, height, hdr }], mics: [{ id, name, default }] }` |
| `GetCapabilities` | – | `{ separate_tracks, max_buffer_s, forecast, encoders, hdr_supported }` |
| `StartBuffer`, `StopBuffer`, `Restart`, `SaveClip` | – | `{}` |
| `PauseMic` | `{ on }` | `{}` |
| `SuspendHotkey` | `{ on }` | `{}` (gibt die Taste frei, solange die Verbindung das verlangt) |
| `TestRegister` | `{ hotkey }` | `{ free, error }` (registriert kurz und gibt sofort frei) |
| `SubscribeMicLevel` | `{ on }` | `{}` (danach `MicLevel`-Ereignisse) |
| `WatchFolders` | `{ on }` | `{}` (Ordnerüberwachung, solange die Galerie offen ist) |
| `ListClips` | `{ folder }` | `{ clips: [ClipEntry], used_bytes, limit_bytes }` |
| `GetClip` | `{ clip_id }` | `ClipEntry` |
| `SetStar` | `{ clip_id, on }` | `{}` |
| `AdoptClip` | `{ clip_id }` | `{}` (nur für `foreign`: schreibt einen eigenen Tag in die Datei) |
| `DeleteClip` | `{ clip_id }` | `{}` (endgültig) |
| `GetFrameTimes` | `{ clip_id }` | `{ frames_us, duration_us }` |
| `GetExportEstimate` | `{ clip_id, in_us, out_us, target_bytes }` | `{ plan, too_small }` |
| `StartExportJob` | `{ clip_id, in_us, out_us, profile }` | `{ job_id }` |
| `CancelJob` | `{ job_id }` | `{}` |
| `ListJobs` | – | `{ jobs: [{ job_id, clip_id, profile, state, percent }] }` |
| `CopyDiagnostics` | – | `{ text }` (Benutzerordner durch `<USER>` ersetzt) |

`SuspendHotkey`, `SubscribeMicLevel` und `WatchFolders` gelten je Verbindung. Trennt sich eine
Verbindung, nimmt der Dienst ihre Anforderungen zurück.

### Beispiele

```json
{"id": 4, "type": "SetSettings", "body": {"partial": {"capture": {"fps": 30}, "audio": {"mic_enabled": false}}}}
```

Eine ungültige Einstellung wird abgelehnt; `details` nennt jedes Feld mit Grund.

```json
{"id": 4, "ok": false, "error": {"code": "InvalidArgument", "message": "Ungültige Einstellungen", "details": [{"key": "storage.clip_folder", "reason": "Ordner existiert nicht"}]}}
```

```json
{"id": 5, "type": "TestRegister", "body": {"hotkey": {"code": "F8", "ctrl": false, "alt": false, "shift": false}}}
```

```json TestRegisterReply
{"id": 5, "ok": true, "body": {"free": false, "error": 1409}}
```

```json
{"id": 6, "type": "ListClips", "body": {"folder": "clips"}}
```

```json ListClipsReply
{"id": 6, "ok": true, "body": {"clips": [{"clip_id": 12, "tag_id": "01JC4V7Q1S6M8B2X3Y4Z5A6B7C", "path": "C:\\Users\\<USER>\\Videos\\GameClip\\Clips\\Valorant 2026-11-05 21-14-03.mp4", "size_bytes": 48211002, "created_at": "2026-11-05T20:14:03Z", "duration_us": 60000000, "starred": false, "has_tracks": true, "status": "ok", "placeholder": false, "in_current_folder": true}], "used_bytes": 48211002, "limit_bytes": 21474836480}}
```

```json
{"id": 7, "type": "StartExportJob", "body": {"clip_id": 12, "in_us": 5000000, "out_us": 35000000, "profile": {"kind": "TargetSize", "bytes": 10000000}}}
```

```json StartExportJobReply
{"id": 7, "ok": true, "body": {"job_id": 3}}
```

```json ListJobsReply
{"id": 8, "ok": true, "body": {"jobs": [{"job_id": 3, "clip_id": 12, "profile": {"kind": "Mix"}, "state": "running", "percent": 42.5}]}}
```

```json GetStatusReply
{"id": 9, "ok": true, "body": {"state": {"kind": "buffering"}, "restrictions": [], "mic_paused": false, "saves_in_flight": 0, "encoder": "h264_amf", "capture_resolution": [2560, 1440], "buffered_us": 59800000, "forecast_total_bytes": 905969664}}
```

`state.kind` ist `starting`, `buffering`, `paused`, `reconfiguring`, `recovering` (mit `attempt`)
oder `failed`.

## Ereignisse

| Ereignis | Body |
| --- | --- |
| `StateChanged` | `ServiceStatus` |
| `SaveCompleted` | `{ clip_id, path, duration_us, size_bytes, has_tracks, shortened }` |
| `SaveFailed` | `{ code, message }` |
| `NewClip`, `ClipDeleted` | `{ clip_id, folder }` |
| `ClipsChanged` | `{ folder }` (nach einem Abgleich mit dem Ordner) |
| `FormatChanged` | `{ width, height, hdr }` |
| `StorageWarning` | `{ folder, used_bytes, limit_bytes, reason }`, `reason` ist `all_starred` oder `clip_larger_than_limit` |
| `JobProgress` | `{ job_id, percent, encoder }` |
| `JobCompleted` | `{ job_id, clip_id }` |
| `JobFailed` | `{ job_id, code, message }` |
| `MicLevel` | `{ dbfs }` (10 Hz, nur nach `SubscribeMicLevel`) |

```json
{"event": "SaveCompleted", "body": {"clip_id": 12, "path": "C:\\Users\\<USER>\\Videos\\GameClip\\Clips\\Valorant 2026-11-05 21-14-03.mp4", "duration_us": 60000000, "size_bytes": 48211002, "has_tracks": true, "shortened": false}}
```

```json
{"event": "StateChanged", "body": {"state": {"kind": "recovering", "attempt": 2}, "restrictions": ["hotkey_inactive"], "mic_paused": true, "saves_in_flight": 1, "encoder": null, "capture_resolution": null, "buffered_us": 0, "forecast_total_bytes": 0}}
```

```json
{"event": "StorageWarning", "body": {"folder": "clips", "used_bytes": 22000000000, "limit_bytes": 21474836480, "reason": "all_starred"}}
```

```json
{"event": "JobProgress", "body": {"job_id": 3, "percent": 42.5, "encoder": "h264_amf"}}
```

```json
{"event": "JobFailed", "body": {"job_id": 3, "code": "TooLarge", "message": "Ziel 1 MB ist kleiner als das Minimum"}}
```

```json
{"event": "MicLevel", "body": {"dbfs": -23.5}}
```

## Fehlercodes

| Code | Bedeutung |
| --- | --- |
| `NotReady` | Der Dienst kann den Befehl gerade nicht ausführen (z. B. Einstellungen nur lesbar). |
| `InvalidArgument` | Ein Feld ist ungültig; `details` nennt bei `SetSettings` jedes Feld. |
| `NotFound` | Clip oder Auftrag gibt es nicht. |
| `Busy` | Eine gleichartige Aktion läuft schon. |
| `HotkeyTaken` | Ein anderes Programm hat die Taste registriert. |
| `DiskFull` | Kein Platz für den Clip. |
| `FolderMissing` | Der Zielordner fehlt. |
| `BufferEmpty` | Der Puffer enthält noch nichts zum Speichern. |
| `EncoderFailed` | Der Encoder ist ausgefallen. |
| `Timeout` | Der Dienst hat nicht rechtzeitig geantwortet (60 s). |
| `TooLarge` | Ziel zu klein oder Nachricht zu groß. |
| `VersionMismatch` | `Hello.protocol` passt nicht. |
| `UnknownCommand` | Unbekannter `type`. |
| `Internal` | Unerwarteter Fehler im Dienst. |
