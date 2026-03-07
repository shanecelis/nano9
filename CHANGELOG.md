# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]

## [0.1.0-alpha.7] - 2026-03-07
- feat: Add [bobtail](https://github.com/shanecelis/bobtail) macros for Pico-8 functions.
``` rust
// In Lua you'd write: spr(n, x, y)
// In Rust you would have written this:
// pico8.spr(
//     n,
//     Vec2::new(x, y),
//     None,
//     None,
//     None,
// )?;
// But now you can write this:
spr!(pico8, n, Vec2::new(x,y))?;
```
- feat: Hot-reload config at runtime.
- chore: Change repo name from "nano-9" to "nano9".
- chore: Rename `NANO9_ASSETS_DIR` to `NANO9_ASSETS`.
- feat: Add indexed-sprite Rust example.
- refactor: Use kebab-case in Nano9.toml.
- feat: Report asset load failures.
- fix: Handle palettes over 16 colors.

## [0.1.0-alpha.6] - 2026-02-07
- bug: Fix handling of URLs on CLI.
- excise: Cannot use `Option<impl T>` for anything.
- bug: Don't show FPS overlay when "inspector" enabled.
- bug: Fix non-nearest sampler images for `map()` calls.
- bug: Remove an `unwrap()` panic.

## [0.1.0-alpha.5] - 2026-02-06
- refactor: Update to Bevy 0.17 and BMS 0.18.
- feat: Nano-9.toml directory defines the default asset directory.
- feat: Add `check` sub-command.
- feat: Add config for key bindings.
- doc: Update compatibility document.
- feat: tline supports using image but not map yet.
- feat: Add bobtail macros: cls!(), spr!(), print!(), and camera!().
- binary: Add logo files.
- feat: mget works with tiled files.
- test: Add lua test harness.
- fix: Use AppExit code in main.
- feat: Implement peek for print cursor y.
- refactor: Use Palettes from assets not resources.

## [0.1.0-alpha.4] - 2025-11-12
- refactor: Update to Bevy 0.16 and BMS 0.16.
- binary: Flop the gameboy color palettes.
- feature: Add resize_constraints and decorations options to screen config.
- feature: Add generic rnd() for Rust.
- feat: Error on unknown field in TOML.
- binary: Add Utah teapot and mesh example.
- refactor: Update to Rust 2024.
- feature: Load a palette from an indexed PNG.
- feat: Add indexed-sprite example.
- feat: Add rot() rotation, etc.
- fix: Use ImageLoader directly to avoid [stackoverflow on reload](https://github.com/bevyengine/bevy/pull/21619).
- feature: Use specific Nano-9 schedule: `nano9::schedule::{Init, Update, Draw}`.
- feature: Make Image and Gfx reflect and inspectable assets.
- fix: Add EguiPlugin if needed.

## [0.1.0-alpha.3] - 2025-10-05
- Recycle print, sprite, and map entities.
- Add '--shared-data=<map|sprite>' option to n9 CLI.
- Add '--pause' option to n9 CLI.
- Add 'new --language=<lua|rust|lua-rust> project' subcommand to n9 CLI.
- Add '[defaults]' section to Nano9.toml.
- Add 'unpack()' to Lua.
- Can specify multiple scripts in Nano9.toml.
- Does 'cls()' on screen covering rectfills.
- Actually limits framerate based on Nano9.toml.

## [0.1.0-alpha.2] - 2025-06-05
- Add `time_to_live` to `Clearable`, makes `map()` more performant.

## [0.1.0-alpha.1] - 2025-05-31

- Initial release for Bevy 0.15.3.
- Released prematurely for [Bevy Jam 6](https://itch.io/jam/bevy-jam-6).

