// SPDX-License-Identifier: GPL-3.0-or-later
//! Feste Kennungen. Sie dürfen sich nach dem ersten Release nie ändern.

/// MSIX-Identity-Name und Basis aller abgeleiteten Namen.
pub const APP_ID: &str = "io.github.poldion.GameClip";

/// Präfix der Named Pipe; die SID des Benutzers wird zur Laufzeit angehängt.
pub const PIPE_NAME_PREFIX: &str = r"\\.\pipe\io.github.poldion.GameClip-";

/// Benannter Mutex, der eine zweite Instanz des Dienstes verhindert.
pub const MUTEX_NAME: &str = r"Local\io.github.poldion.GameClip";

/// TaskId des MSIX-StartupTask für den Dienst.
pub const STARTUP_TASK_ID: &str = "GameClipServiceStartup";

/// Protokoll-Schema für die Aktivierung aus Benachrichtigungen (`gameclip:reveal?id=…`).
pub const PROTOCOL_SCHEME: &str = "gameclip";

/// Vollständiger Pipe-Name für eine Benutzer-SID.
pub fn pipe_name(user_sid: &str) -> String {
    format!("{PIPE_NAME_PREFIX}{user_sid}")
}
