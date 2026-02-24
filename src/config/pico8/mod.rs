use bevy::{asset::embedded_asset, prelude::*};

pub const PALETTE: &str = "embedded://nano9/config/pico8/pico-8-palette.png";
pub const BORDER: &str = "embedded://nano9/config/pico8/rect-border.png";
pub const FONT: &str = "embedded://nano9/config/pico8/pico-8.ttf";
pub const CONFIG: &str = "embedded://nano9/config/pico8/Nano9.toml";
pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "pico-8-palette.png");
    embedded_asset!(app, "rect-border.png");
    embedded_asset!(app, "pico-8.ttf");
    embedded_asset!(app, "Nano9.toml");
}
