# Changelog

All notable changes to this project will be documented in this file.

## [unreleased]
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

