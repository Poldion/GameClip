// SPDX-License-Identifier: GPL-3.0-or-later
//! Erkennt, ob Windows-HDR auf einem Monitor aktiv ist. v1 nimmt dann in SDR auf und warnt.

use windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020;
use windows::Win32::Graphics::Dxgi::{IDXGIOutput, IDXGIOutput6};
use windows_core::Interface;

use crate::Result;

pub fn is_hdr_active(output: &IDXGIOutput) -> Result<bool> {
    let output6: IDXGIOutput6 = output.cast()?;
    let desc = unsafe { output6.GetDesc1()? };
    Ok(desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020)
}
