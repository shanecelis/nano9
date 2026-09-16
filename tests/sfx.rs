//! Compare Nano-9 SFX mixdowns against Pico-8 goldens in `tests/golden/sfx/`.
//!
//! Generate goldens (Pico-8 required):
//! ```sh
//! make golden
//! ```
//!
//! Run:
//! ```sh
//! cargo test --test sfx
//! cargo test --test sfx -- triangle
//! cargo test-sfx
//! ```
//!
//! `cargo test sfx` also selects this target (and any other tests whose names
//! contain `sfx`). Carts write `{name}-expected.wav` via `extcmd("audio_rec")`
//! and `{name}-actual.wav` from Nano-9's offline mixdown.

mod common;

fn main() {
    common::run_sfx_suite();
}
