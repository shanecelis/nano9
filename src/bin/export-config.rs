//! Developer tool: export default configs to TOML files.
//!
//! Writes `Config::pico8()` to `config/pico8/Nano9.toml` and
//! `Config::gameboy()` to `config/gameboy/Nano9.toml` (when built with `--features gameboy`).

use std::env;
use std::fs;
use std::path::PathBuf;

use nano9::config::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let pico8_dir = manifest_dir.join("src").join("config").join("pico8");
    fs::create_dir_all(&pico8_dir)?;
    let pico8_path = pico8_dir.join("Nano9.toml");
    let pico8_toml = toml::ser::to_string_pretty(&Config::pico8())?;
    fs::write(&pico8_path, pico8_toml)?;
    println!("Wrote {}", pico8_path.display());

    #[cfg(feature = "gameboy")]
    {
        let gameboy_dir = manifest_dir.join("src").join("config").join("gameboy");
        fs::create_dir_all(&gameboy_dir)?;
        let gameboy_path = gameboy_dir.join("Nano9.toml");
        let gameboy_toml = toml::ser::to_string_pretty(&Config::gameboy())?;
        fs::write(&gameboy_path, gameboy_toml)?;
        println!("Wrote {}", gameboy_path.display());
    }

    #[cfg(not(feature = "gameboy"))]
    {
        eprintln!("Built without 'gameboy' feature; only pico8 config was exported. Use --features gameboy to export both.");
    }

    Ok(())
}
