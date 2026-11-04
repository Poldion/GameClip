// SPDX-License-Identifier: GPL-3.0-or-later
//! Umrechnung von `KeyboardEvent.code` auf Windows-Tastencodes für `RegisterHotKey`.
//!
//! Die Werte sind hier als Zahlen hinterlegt, damit das Crate keinen Windows-Code braucht.

use thiserror::Error;

use crate::settings::Hotkey;

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_NOREPEAT: u32 = 0x4000;

const VK_F1: u32 = 0x70;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KeyError {
    #[error("F12 ist für Debugger reserviert")]
    F12Reserved,
    #[error("Buchstaben und Ziffern nur zusammen mit Strg oder Alt")]
    NeedsModifier,
    #[error("unbekannte Taste: {0}")]
    Unknown(String),
}

/// Liefert `(Modifier, VK)` für `RegisterHotKey`. `MOD_NOREPEAT` ist immer gesetzt.
pub fn to_vk(hotkey: &Hotkey) -> Result<(u32, u32), KeyError> {
    let code = hotkey.code.as_str();
    let mut modifiers = MOD_NOREPEAT;
    if hotkey.ctrl {
        modifiers |= MOD_CONTROL;
    }
    if hotkey.alt {
        modifiers |= MOD_ALT;
    }
    if hotkey.shift {
        modifiers |= MOD_SHIFT;
    }

    if let Some(n) = code.strip_prefix('F').and_then(|n| n.parse::<u32>().ok()) {
        return match n {
            12 => Err(KeyError::F12Reserved),
            1..=24 => Ok((modifiers, VK_F1 + n - 1)),
            _ => Err(KeyError::Unknown(code.into())),
        };
    }

    let vk = if let Some(letter) = code.strip_prefix("Key") {
        single_char(letter)
            .filter(char::is_ascii_uppercase)
            .map(|c| c as u32)
    } else if let Some(digit) = code.strip_prefix("Digit") {
        single_char(digit)
            .filter(char::is_ascii_digit)
            .map(|c| c as u32)
    } else {
        None
    };
    let vk = vk.ok_or_else(|| KeyError::Unknown(code.into()))?;
    if !hotkey.ctrl && !hotkey.alt {
        return Err(KeyError::NeedsModifier);
    }
    Ok((modifiers, vk))
}

fn single_char(s: &str) -> Option<char> {
    let mut chars = s.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}
