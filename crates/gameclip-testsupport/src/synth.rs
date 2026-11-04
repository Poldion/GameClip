// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetische Bilder und Töne.
//!
//! Jedes Bild trägt seine Nummer als 32-Bit-Muster oben links: 8 × 4 Felder zu 16 × 16 Pixeln,
//! Bit 0 oben links, zeilenweise; weiß = 1, schwarz = 0. Der Rest des Bildes ist bewegtes
//! Rauschen, damit der Encoder nicht trivial komprimiert. Nach verlustbehafteter Kodierung
//! lässt sich die Nummer über den Mittelwert jedes Feldes zurücklesen.

use rsmpeg::avutil::AVFrame;
use rsmpeg::ffi;

pub const FIELD: usize = 16;
pub const COLS: usize = 8;
pub const ROWS: usize = 4;
/// Breite und Höhe des Musters in Pixeln.
pub const PATTERN_W: usize = FIELD * COLS;
pub const PATTERN_H: usize = FIELD * ROWS;

const WHITE: u8 = 235;
const BLACK: u8 = 16;

/// NV12-Bild in Hauptspeicher mit Nummer `index`. `width` ≥ 128, `height` ≥ 64, beide gerade.
pub fn nv12_frame(index: u32, width: u32, height: u32) -> AVFrame {
    assert!(width as usize >= PATTERN_W && height as usize >= PATTERN_H);
    let mut f = AVFrame::new();
    f.set_format(ffi::AV_PIX_FMT_NV12);
    f.set_width(width as i32);
    f.set_height(height as i32);
    f.get_buffer(0).expect("Puffer für Testbild");
    let (w, h) = (width as usize, height as usize);
    let y_stride = f.linesize[0] as usize;
    let uv_stride = f.linesize[1] as usize;
    let luma = unsafe { std::slice::from_raw_parts_mut(f.data[0], y_stride * h) };
    let chroma = unsafe { std::slice::from_raw_parts_mut(f.data[1], uv_stride * h / 2) };

    // Festes Rauschmuster, das je Bild um 3 Pixel nach links wandert: echte Bewegung, aber
    // kein Szenenwechsel (völlig neues Rauschen je Bild macht bei x264 jedes Bild zum Keyframe).
    let shift = index as usize * 3;
    for y in 0..h {
        for x in 0..w {
            luma[y * y_stride + x] = if x < PATTERN_W && y < PATTERN_H {
                let bit = (y / FIELD) * COLS + x / FIELD;
                if index >> bit & 1 == 1 { WHITE } else { BLACK }
            } else {
                108 + (noise(x + shift, y) % 40) as u8
            };
        }
    }
    chroma.fill(128);
    f
}

/// Liest die Nummer aus einer Y-Ebene (Zeilenabstand `stride`).
pub fn read_number(luma: &[u8], stride: usize) -> u32 {
    let mut n = 0u32;
    for bit in 0..COLS * ROWS {
        let (fx, fy) = (bit % COLS * FIELD, bit / COLS * FIELD);
        // Inneres 8 × 8 des Feldes, Ränder sind nach der Kodierung unscharf.
        let mut sum = 0u32;
        for y in fy + 4..fy + 12 {
            for x in fx + 4..fx + 12 {
                sum += u32::from(luma[y * stride + x]);
            }
        }
        if sum / 64 > 128 {
            n |= 1 << bit;
        }
    }
    n
}

/// Interleaved Stereo-Sinus mit 48 kHz, `frames` Sample-Paare ab Sample `start`.
pub fn sine(start: usize, frames: usize, freq: f32) -> Vec<f32> {
    (start..start + frames)
        .flat_map(|i| {
            let s = (i as f32 * freq * std::f32::consts::TAU / 48_000.0).sin() * 0.3;
            [s, s]
        })
        .collect()
}

/// Pseudozufall je Pixelposition (feste Werte, kein Zustand).
fn noise(x: usize, y: usize) -> u64 {
    let mut v = (x as u64) << 32 ^ y as u64;
    v = (v ^ v >> 33).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    v = (v ^ v >> 33).wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    v ^ v >> 33
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_roundtrips_without_compression() {
        for n in [0, 1, 0xDEAD_BEEF, u32::MAX, 299] {
            let f = nv12_frame(n, 320, 240);
            let luma =
                unsafe { std::slice::from_raw_parts(f.data[0], (f.linesize[0] * 240) as usize) };
            assert_eq!(read_number(luma, f.linesize[0] as usize), n);
        }
    }
}
