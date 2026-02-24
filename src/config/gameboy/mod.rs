use bevy::{asset::embedded_asset, prelude::*};

pub const PALETTES: &str = "embedded://nano9/config/gameboy/palettes.png";
pub const FONT: &str = "embedded://nano9/config/gameboy/font.ttf";
pub const CONFIG: &str = "embedded://nano9/config/gameboy/Nano9.toml";
pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "palettes.png");
    embedded_asset!(app, "font.ttf");
    embedded_asset!(app, "Nano9.toml");
}
