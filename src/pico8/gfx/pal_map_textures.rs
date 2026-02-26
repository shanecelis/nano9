//! Build R8 textures from PalMap for use in the GPU palette lookup shader.

use crate::pico8::PalMap;
use bevy::asset::RenderAssetUsages;
use bevy::{
    image::ImageSampler,
    prelude::Image,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const PAL_MAP_SIZE: usize = 256;

/// Create two R8 textures from a PalMap for shader bindings: remap (index → remapped index)
/// and transparency (index → 0 or 255).
pub fn pal_map_to_images(pal_map: &PalMap) -> (Image, Image) {
    let mut remap_bytes = vec![0u8; PAL_MAP_SIZE];
    let mut transparency_bytes = vec![0u8; PAL_MAP_SIZE];
    for i in 0..PAL_MAP_SIZE {
        remap_bytes[i] = pal_map.map_or_mod(i) as u8;
        transparency_bytes[i] = if pal_map
            .transparency
            .get(i)
            .map(|b| *b)
            .unwrap_or(false) { 255 } else { 0 };
    }
    let remap_image = image_from_r8(&remap_bytes);
    let transparency_image = image_from_r8(&transparency_bytes);
    (remap_image, transparency_image)
}

fn image_from_r8(bytes: &[u8]) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: bytes.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes.to_vec(),
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}
