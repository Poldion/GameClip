-- SPDX-License-Identifier: GPL-3.0-or-later
-- Index der Clips und Exporte, Schema 1 (Design, Datei 45).
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
