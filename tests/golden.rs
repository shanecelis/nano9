//! Compare Nano-9 screenshots against Pico-8 goldens in `tests/golden/`.
//!
//! Generate goldens (Pico-8 required):
//! ```sh
//! make golden
//! ```
//!
//! Run (headless GPU, `-p headless` so carts screenshot and exit). Some
//! primitives mismatch; that is expected — the point is to see the diff,
//! not to paper over it:
//! ```sh
//! cargo test-golden
//! cargo test-golden pset
//! cargo test-golden cls   # cls and cls-white
//! cargo test --test sfx   # audio carts in tests/golden/sfx/
//! ```
//!
//! Image carts are every `tests/golden/*.p8`. They write `{name}-expected.png`
//! (Pico-8) and `{name}-actual.png` (Nano-9). On mismatch a `{name}-compare.png`
//! is shown with `wezterm imgcat` when a tty is available.

mod common;

fn main() {
    common::run_image_suite();
}
