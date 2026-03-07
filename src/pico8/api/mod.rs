mod error;
pub use error::*;
mod asset;
use super::*;
pub use asset::*;
mod spr;
pub use spr::*;
mod state;
pub use state::*;
mod handle;
pub use handle::*;
pub mod input;
use input::*;
pub use input::{btn, btnp};
mod camera;
pub use camera::*;
mod camera3d;
pub use camera3d::Nano9Camera3d;
mod param;
pub use param::*;
mod sfx;
pub use sfx::*;
mod circ;
pub use circ::*;
mod map;
pub use map::*;
mod oval;
pub use oval::*;
mod pal;
pub(crate) mod print;
mod rect;
pub use pal::*;
pub use rect::*;
mod line;
pub use line::*;
mod bit_ops;
pub mod canvas;
#[cfg(feature = "level")]
mod level;
mod poke;
#[cfg(feature = "rand")]
mod rand;
mod sys;
#[cfg(feature = "level")]
pub use level::*;
mod mesh;
pub use mesh::*;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    input::gamepad::GamepadConnectionEvent,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Anchor,
    text::TextLayoutInfo,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

use crate::{
    DrawState, FillColor, PColor,
    pico8::{
        self, Clearable, PalMap, Palette, SpriteMap, audio::AudioBank, image::pixel_art_settings,
    },
};
use std::borrow::Cow;

pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

const ANALOG_STICK_THRESHOLD: f32 = 0.1;

pub(crate) fn plugin(app: &mut App) {
    app
        //.register_type::<Pico8Asset>()
        // .register_type::<Pico8State>()
        // .register_type::<N9Font>()
        // .register_type::<Palette>()
        // .register_type::<SpriteSheet>()
        .init_asset::<Pico8Asset>()
        .init_resource::<Pico8State>()
        .init_resource::<PlayerInputs>()
        .add_plugins((
            #[cfg(feature = "rand")]
            rand::plugin,
            asset::plugin,
            mesh::plugin,
            camera3d::plugin,
            state::plugin,
        ))
        .add_plugins((
            sfx::plugin,
            spr::plugin,
            map::plugin,
            input::plugin,
            print::plugin,
            rect::plugin,
            circ::plugin,
            oval::plugin,
            pal::plugin,
            bit_ops::plugin,
            line::plugin,
        ))
        .add_plugins((
            poke::plugin,
            canvas::plugin,
            camera::plugin,
            sys::plugin,
            #[cfg(feature = "level")]
            level::plugin,
        ));
}

#[cfg(test)]
mod test {

    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
    }
}
