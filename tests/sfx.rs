//! Compare Nano-9 SFX against Pico-8 goldens.
//!
//! Three tiers:
//! - `tests/golden/synth/`: tracker render vs `EXPORT SYNTH%D.WAV` (`make golden-synth`)
//! - `tests/golden/sfx/`: playback mix vs `audio_rec` (`make golden`)
//! - `tests/golden/music/`: `music()` pattern playback vs `audio_rec` (`make golden`)
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
//! cargo test-sfx music
//! cargo test-sfx synth
//! cargo test-sfx synth --save-baseline
//! cargo test-sfx synth --save-baseline before
//! cargo test-sfx synth --baseline before
//! ```
//!
//! `cargo test-sfx synth` prints a wave-match summary for the synth slots:
//! frequency (cents), amplitude (gain ratio), phase (samples), and Pico-8
//! shape residual. `--save-baseline` writes `tests/golden/synth/wave-baseline.json`.
//! `--save-baseline before` writes `wave-baseline-before.json`, and
//! `--baseline before` compares against that file. An unknown `--baseline`
//! name fails the run. A drop is a closer Pico-8 match.
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
