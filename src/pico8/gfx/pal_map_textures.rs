//! Build a packed LUT texture from PalMap for the GPU palette lookup shader.

use crate::pico8::PalMap;
use bevy::asset::RenderAssetUsages;
use bevy::prelude::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const PAL_MAP_SIZE: usize = 256;

/// One `Rg8Uint` row: `r` is the remapped index, `g` is 1 when `palt` hides that index.
pub fn pal_map_to_lut_image(pal_map: &PalMap) -> Image {
    let mut bytes = vec![0u8; PAL_MAP_SIZE * 2];
    for i in 0..PAL_MAP_SIZE {
        bytes[i * 2] = pal_map.map_or_mod(i) as u8;
        bytes[i * 2 + 1] = u8::from(pal_map.transparency.get(i).is_some_and(|bit| *bit));
    }
    Image::new(
        Extent3d {
            width: PAL_MAP_SIZE as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rg8Uint,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}
