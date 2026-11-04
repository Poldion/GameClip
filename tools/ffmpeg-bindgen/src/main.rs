// SPDX-License-Identifier: GPL-3.0-or-later
//! Erzeugt die FFmpeg-Bindings für `rusty_ffmpeg` mit einer neueren bindgen-Version.
//!
//! Aufruf: `ffmpeg-bindgen <include-dir> <ausgabe.rs>`. Header-Liste und Optionen entsprechen
//! `build.rs` von `rusty_ffmpeg` 0.16.7 (MIT), damit `rsmpeg` dieselben Namen findet.

use std::collections::HashSet;
use std::path::PathBuf;

use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};

/// Header in derselben Reihenfolge wie bei `rusty_ffmpeg`. Fehlende (z. B. `avfft.h`, in
/// FFmpeg 8 entfernt) werden übersprungen.
const HEADERS: &[&str] = &[
    "libavcodec/ac3_parser.h",
    "libavcodec/adts_parser.h",
    "libavcodec/avcodec.h",
    "libavcodec/avdct.h",
    "libavcodec/avfft.h",
    "libavcodec/bsf.h",
    "libavcodec/codec.h",
    "libavcodec/codec_desc.h",
    "libavcodec/codec_id.h",
    "libavcodec/codec_par.h",
    "libavcodec/defs.h",
    "libavcodec/dirac.h",
    "libavcodec/dv_profile.h",
    "libavcodec/jni.h",
    "libavcodec/mediacodec.h",
    "libavcodec/packet.h",
    "libavcodec/version.h",
    "libavcodec/version_major.h",
    "libavcodec/vorbis_parser.h",
    "libavdevice/avdevice.h",
    "libavdevice/version.h",
    "libavdevice/version_major.h",
    "libavfilter/avfilter.h",
    "libavfilter/buffersink.h",
    "libavfilter/buffersrc.h",
    "libavfilter/version.h",
    "libavfilter/version_major.h",
    "libavformat/avformat.h",
    "libavformat/avio.h",
    "libavformat/version.h",
    "libavformat/version_major.h",
    "libavutil/adler32.h",
    "libavutil/aes.h",
    "libavutil/aes_ctr.h",
    "libavutil/ambient_viewing_environment.h",
    "libavutil/attributes.h",
    "libavutil/audio_fifo.h",
    "libavutil/avassert.h",
    "libavutil/avconfig.h",
    "libavutil/avstring.h",
    "libavutil/avutil.h",
    "libavutil/base64.h",
    "libavutil/blowfish.h",
    "libavutil/bprint.h",
    "libavutil/bswap.h",
    "libavutil/buffer.h",
    "libavutil/camellia.h",
    "libavutil/cast5.h",
    "libavutil/channel_layout.h",
    "libavutil/common.h",
    "libavutil/cpu.h",
    "libavutil/crc.h",
    "libavutil/csp.h",
    "libavutil/des.h",
    "libavutil/detection_bbox.h",
    "libavutil/dict.h",
    "libavutil/display.h",
    "libavutil/dovi_meta.h",
    "libavutil/downmix_info.h",
    "libavutil/encryption_info.h",
    "libavutil/error.h",
    "libavutil/eval.h",
    "libavutil/executor.h",
    "libavutil/ffversion.h",
    "libavutil/fifo.h",
    "libavutil/file.h",
    "libavutil/film_grain_params.h",
    "libavutil/frame.h",
    "libavutil/hash.h",
    "libavutil/hdr_dynamic_metadata.h",
    "libavutil/hdr_dynamic_vivid_metadata.h",
    "libavutil/hmac.h",
    "libavutil/hwcontext.h",
    "libavutil/imgutils.h",
    "libavutil/intfloat.h",
    "libavutil/intreadwrite.h",
    "libavutil/lfg.h",
    "libavutil/log.h",
    "libavutil/lzo.h",
    "libavutil/macros.h",
    "libavutil/mastering_display_metadata.h",
    "libavutil/mathematics.h",
    "libavutil/md5.h",
    "libavutil/mem.h",
    "libavutil/motion_vector.h",
    "libavutil/murmur3.h",
    "libavutil/opt.h",
    "libavutil/parseutils.h",
    "libavutil/pixdesc.h",
    "libavutil/pixelutils.h",
    "libavutil/pixfmt.h",
    "libavutil/random_seed.h",
    "libavutil/rational.h",
    "libavutil/rc4.h",
    "libavutil/replaygain.h",
    "libavutil/ripemd.h",
    "libavutil/samplefmt.h",
    "libavutil/sha.h",
    "libavutil/sha512.h",
    "libavutil/spherical.h",
    "libavutil/stereo3d.h",
    "libavutil/tea.h",
    "libavutil/threadmessage.h",
    "libavutil/time.h",
    "libavutil/timecode.h",
    "libavutil/timestamp.h",
    "libavutil/tree.h",
    "libavutil/twofish.h",
    "libavutil/tx.h",
    "libavutil/uuid.h",
    "libavutil/version.h",
    "libavutil/video_enc_params.h",
    "libavutil/video_hint.h",
    "libavutil/xtea.h",
    "libswresample/swresample.h",
    "libswresample/version.h",
    "libswresample/version_major.h",
    "libswscale/swscale.h",
    "libswscale/version.h",
    "libswscale/version_major.h",
];

/// Die `FP_*`-Makros aus `math.h` sind doppelt definiert (rust-bindgen#687).
#[derive(Debug)]
struct IgnoreMacros(HashSet<&'static str>);

impl ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if self.0.contains(name) {
            MacroParsingBehavior::Ignore
        } else {
            MacroParsingBehavior::Default
        }
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let (Some(include), Some(out)) = (args.next(), args.next()) else {
        eprintln!("Aufruf: ffmpeg-bindgen <include-dir> <ausgabe.rs>");
        std::process::exit(2);
    };
    let include = PathBuf::from(include);

    let ignore = ["FP_NAN", "FP_INFINITE", "FP_ZERO", "FP_SUBNORMAL", "FP_NORMAL"];
    let mut builder = bindgen::builder()
        .impl_debug(true)
        .rust_target(bindgen::RustTarget::stable(68, 0).expect("Rust 1.68"))
        .parse_callbacks(Box::new(IgnoreMacros(ignore.into_iter().collect())))
        .clang_arg(format!("-I{}", include.display()))
        .blocklist_type("__mingw_ldbl_type_t")
        .prepend_enum_name(false);
    for header in HEADERS {
        let path = include.join(header);
        if path.exists() {
            builder = builder.header(path.to_string_lossy());
        } else {
            eprintln!("übersprungen (fehlt): {header}");
        }
    }
    builder
        .generate()
        .expect("bindgen fehlgeschlagen")
        .write_to_file(&out)
        .expect("Ausgabe nicht schreibbar");
}
