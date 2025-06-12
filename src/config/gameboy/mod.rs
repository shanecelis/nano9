use bevy::{
    prelude::*,
    asset::embedded_asset,
};

pub const PALETTES: &str = "embedded://nano9/config/gameboy/palettes.png";
pub const FONT: &str = "embedded://nano9/config/gameboy/font.ttf";
pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "palettes.png");
    embedded_asset!(app, "font.ttf");
}
