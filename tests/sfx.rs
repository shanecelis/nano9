//! Compare Nano-9 SFX against Pico-8 goldens.
//!
//! Two tiers:
//! - `tests/golden/synth/`: tracker render vs `EXPORT SYNTH%D.WAV` (`make golden-synth`)
//! - `tests/golden/sfx/`: playback mix vs `audio_rec` (`make golden`)
//!
//! Goldens on disk are Ogg Vorbis (`*-expected.ogg`). Pico-8 still captures
//! WAV; ingest converts with the same encoder the tests use. Runtime
//! `write_wav` / `extcmd("audio_end")` stay WAV.
//!
//! Run:
//! ```sh
//! cargo test --test sfx
//! cargo test --test sfx -- triangle
//! cargo test-sfx
//! cargo test-sfx synth
//! ```
//!
//! `cargo test sfx` also selects this target (and any other tests whose names
//! contain `sfx`). Synth slots are rendered in-process from `Sfx::decode()`,
//! except export goldens start the oscillator at Pico-8's leftover EXPORT
//! phase (test-only). Pico-8 `EXPORT %d.wav` also writes a silent pad before
//! the wave (osc-02 is 93 samples / 0.0042s); Nano-9 does not, and the
//! harness strips that pad from the expected file when comparing. Playback
//! carts write `{name}.wav` from Nano-9's offline mixdown (`-p headless`
//! mutes the speaker); the harness encodes `{name}-actual.ogg` and compares
//! decoded PCM.

mod common;

fn main() {
    common::run_sfx_suite();
}
