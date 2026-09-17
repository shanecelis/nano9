//! Encode a 22050 Hz mono 16-bit WAV as Ogg Vorbis q4.
//!
//! Same encoder as the sfx golden harness (`tests/common/audio_ogg.rs`).
//!
//!   cargo run --example wav-to-ogg -- INPUT.wav OUTPUT.ogg

#[path = "../tests/common/audio_ogg.rs"]
mod audio_ogg;

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().unwrap_or_else(|| usage());
    let output = args.next().unwrap_or_else(|| usage());
    if args.next().is_some() {
        usage();
    }
    let pcm = audio_ogg::load_wav(Path::new(&input));
    audio_ogg::write_ogg(Path::new(&output), &pcm).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });
}

fn usage() -> ! {
    eprintln!("usage: wav-to-ogg INPUT.wav OUTPUT.ogg");
    std::process::exit(2);
}
