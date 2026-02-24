use bevy::{asset::embedded_asset, prelude::*};
mod api;
pub use api::*;
// pub mod cartridge;
mod cart;
pub use cart::*;
mod clear;
pub use clear::*;
pub mod audio;
mod map;
pub use map::*;
#[cfg(feature = "scripting")]
pub(crate) mod lua;
mod pal_map;
pub(crate) use pal_map::*;
mod pal;
pub(crate) use pal::*;
mod gfx;
#[cfg(feature = "rand")]
pub(crate) mod rand;
pub use gfx::*;
mod fillp;
pub mod p8scii;
pub(crate) use fillp::*;
mod palettes;
pub(crate) use palettes::*;
mod defaults;
pub(crate) mod image;
pub(crate) mod keyboard;
pub(crate) mod mouse;
pub(crate) use defaults::*;
pub(crate) mod sprite_sheet;
pub use sprite_sheet::*;
// mod gfx2;

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(api::plugin)
        .add_plugins(clear::plugin)
        .add_plugins(audio::plugin)
        .add_plugins(gfx::plugin)
        .add_plugins(palettes::plugin)
        .add_plugins(keyboard::plugin)
        .add_plugins(mouse::plugin)
        .add_plugins(sprite_sheet::plugin)
        .add_plugins(map::plugin)
        .add_plugins(pal::plugin)
        .add_plugins(cart::plugin);

    #[cfg(feature = "rand")]
    app.add_plugins(rand::plugin);
}
