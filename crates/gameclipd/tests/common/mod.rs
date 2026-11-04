// SPDX-License-Identifier: GPL-3.0-or-later
//! Gemeinsame Hilfen der Integrationstests.
#![allow(dead_code)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use gameclip_media::mux::{Track, write_mp4};
use gameclip_media::{EncodeProfile, EncoderConfig, VideoEncoder};
use gameclipd::paths::Paths;
use gameclipd::retention_exec::FileProbe;

pub fn paths(dir: &Path) -> Paths {
    let p = Paths {
        clips: dir.join("clips"),
        exports: dir.join("exports"),
        tracks: dir.join("tracks"),
        index: dir.join("index.sqlite"),
        settings: dir.join("settings.toml"),
        logs: dir.join("logs"),
    };
    p.create_all().unwrap();
    p
}

/// Kleine MP4 (1 s, 320 × 240) mit `gameclip_id = tag` (leer: ohne Tag); `pad` Bytes Füllung
/// in einem eigenen Tag, damit Dateien verschieden groß sein können.
pub fn tagged_mp4(path: &Path, tag: &str, pad: usize) {
    let cfg = EncoderConfig {
        width: 320,
        height: 240,
        fps: 30,
        bitrate_bps: 300_000,
        profile: EncodeProfile::Capture,
    };
    let mut enc = VideoEncoder::open("libx264", &cfg, None).unwrap();
    let mut pk = Vec::new();
    for i in 0..30 {
        let mut f = gameclip_testsupport::synth::nv12_frame(i, 320, 240);
        enc.send(&mut f, i64::from(i) * 33_333, false).unwrap();
        while let Some(p) = enc.receive().unwrap() {
            pk.push(p);
        }
    }
    enc.flush().unwrap();
    while let Some(p) = enc.receive().unwrap() {
        pk.push(p);
    }
    let par = enc.codec_parameters();
    let filler = "x".repeat(pad);
    let mut tags = vec![("comment", filler.as_str())];
    if !tag.is_empty() {
        tags.push(("gameclip_id", tag));
    }
    write_mp4(
        path,
        &Track {
            params: &par,
            time_base: rsmpeg::ffi::AVRational {
                num: 1,
                den: 1_000_000,
            },
            packets: &pk,
            title: "Video",
            default: true,
        },
        &[],
        &tags,
        0,
        false,
    )
    .unwrap();
}

/// Attrappe: Platzhalter sind die Pfade in der Menge.
#[derive(Default)]
pub struct FakeProbe(pub Mutex<HashSet<PathBuf>>);

impl FakeProbe {
    pub fn mark(&self, p: &Path) {
        self.0.lock().unwrap().insert(p.to_path_buf());
    }
}

impl FileProbe for FakeProbe {
    fn is_placeholder(&self, path: &Path) -> bool {
        self.0.lock().unwrap().contains(path)
    }
}
