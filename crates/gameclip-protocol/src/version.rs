// SPDX-License-Identifier: GPL-3.0-or-later

/// Eine Versionsnummer; Oberfläche und Dienst kommen im selben Paket und müssen gleich sein.
pub const PROTOCOL_VERSION: u32 = 1;

/// Höchstgröße einer Nachricht in Byte (dezimal, 8 MB).
pub const MAX_MESSAGE_BYTES: usize = 8_000_000;
