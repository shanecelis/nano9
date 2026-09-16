//! Compare Nano-9 SFX against Pico-8 goldens.
//!
//! Two tiers:
//! - `tests/golden/synth/`: tracker render vs `EXPORT SYNTH%D.WAV` (`make golden-synth`)
//! - `tests/golden/sfx/`: playback mix vs `audio_rec` (`make golden`)
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
//! contain `sfx`). Synth slots are rendered in-process from `Sfx::decode`.
//! Playback carts write `{name}-actual.wav` from Nano-9's offline mixdown
//! (`-p headless` mutes the speaker).

mod common;

fn main() {
    common::run_sfx_suite();
}
