// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows-Benachrichtigungen (Design, Datei 52).
//!
//! `ToastNotificationManager::CreateToastNotifierWithId(AUMID)`. Ohne MSIX-Paket hat der Dienst
//! keine registrierte AUMID; Windows verwirft den Toast dann still oder meldet einen Fehler. Das
//! wird geloggt; der Ton bleibt das Hauptsignal. Ob Toasts im Paket mit `AppListEntry="none"`
//! erscheinen, prüft S1.

use gameclip_core::ids::APP_ID;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows_core::HSTRING;

/// AUMID der Paket-App: `<Paketfamilie>!Service`. Die Familie enthält den Hash des Publishers
/// (z. B. `io.github.poldion.GameClip_r7hhd5js3fz4t`); mit dem bloßen Paketnamen verwirft
/// Windows den Toast still (gemessen 2026-11). Ohne Paket gibt es keine gültige AUMID.
pub fn aumid() -> String {
    if let Ok(id) = std::env::var("GAMECLIP_AUMID") {
        return id;
    }
    match package_family_name() {
        Some(pfn) => format!("{pfn}!Service"),
        None => format!("{APP_ID}!Service"),
    }
}

fn package_family_name() -> Option<String> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFamilyName;
    let mut len = 0u32;
    unsafe {
        let _ = GetCurrentPackageFamilyName(&mut len, None);
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u16; len as usize];
        if GetCurrentPackageFamilyName(&mut len, Some(windows_core::PWSTR(buf.as_mut_ptr())))
            != ERROR_SUCCESS
        {
            return None;
        }
        Some(String::from_utf16_lossy(&buf[..len.saturating_sub(1) as usize]))
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Zeigt einen Toast; `reveal_id` fügt den Knopf „Im Ordner anzeigen“ hinzu.
pub fn show(title: &str, text: &str, reveal_id: Option<u32>) -> windows::core::Result<()> {
    let actions = reveal_id
        .map(|id| {
            format!(
                "<actions><action content=\"Im Ordner anzeigen\" activationType=\"protocol\" \
                 arguments=\"gameclip:reveal?id={id}\"/></actions>"
            )
        })
        .unwrap_or_default();
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text>\
         </binding></visual>{actions}</toast>",
        escape(title),
        escape(text)
    );
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(aumid()))?.Show(&toast)
}

/// „1:05“ für 65 s.
pub fn format_duration(us: u64) -> String {
    let s = us / 1_000_000;
    format!("{}:{:02}", s / 60, s % 60)
}

/// „142 MB“ bzw. „0,8 MB“.
pub fn format_size(bytes: u64) -> String {
    let mb = bytes as f64 / 1_000_000.0;
    if mb >= 10.0 {
        format!("{mb:.0} MB")
    } else {
        format!("{mb:.1} MB").replace('.', ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_match_the_design_text() {
        assert_eq!(format_duration(60_000_000), "1:00");
        assert_eq!(format_duration(65_400_000), "1:05");
        assert_eq!(format_size(142_300_000), "142 MB");
        assert_eq!(format_size(840_000), "0,8 MB");
        assert_eq!(escape("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
    }
}
