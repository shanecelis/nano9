//! Custom Material2d that samples an index texture and palette texture with PalMap remap/transparency.

use bevy::asset::{Asset, AssetApp, uuid_handle};
use bevy::reflect::Reflect;
use bevy::render::render_resource::*;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};
use bevy::shader::Shader;
use bevy::{prelude::*, shader::ShaderRef};

const GFX_PALETTE_LOOKUP_SHADER: Handle<Shader> = uuid_handle!("9f4e8a2b-1c3d-4e5f-6a7b-8c9d0e1f2a3b");

/// Material that renders a fullscreen quad using index + palette + PalMap textures (GPU palette lookup).
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
#[reflect(Debug)]
pub struct GfxPaletteLookupMaterial {
    #[texture(0)]
    #[sampler(4)]
    pub index_texture: Handle<Image>,
    #[texture(1)]
    pub palette_texture: Handle<Image>,
    #[texture(2)]
    pub remap_texture: Handle<Image>,
    #[texture(3)]
    pub transparency_texture: Handle<Image>,
    #[uniform(5)]
    pub palette_size: f32,
}

impl Material2d for GfxPaletteLookupMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Handle(GFX_PALETTE_LOOKUP_SHADER.clone())
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// Build a GfxPaletteLookupMaterial from index, palette, remap, and transparency images.
pub fn build_palette_lookup_material(
    index_image: Handle<Image>,
    palette_image: Handle<Image>,
    remap_image: Handle<Image>,
    transparency_image: Handle<Image>,
    palette_size: usize,
) -> GfxPaletteLookupMaterial {
    GfxPaletteLookupMaterial {
        index_texture: index_image,
        palette_texture: palette_image,
        remap_texture: remap_image,
        transparency_texture: transparency_image,
        palette_size: palette_size as f32,
    }
}

pub(crate) fn gfx_palette_lookup_material_plugin(app: &mut App) {
    let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
    shaders
        .insert(
            GFX_PALETTE_LOOKUP_SHADER.id(),
            Shader::from_wgsl(
                include_str!("shaders/gfx_palette_lookup.wgsl"),
                "shaders/gfx_palette_lookup.wgsl",
            ),
        )
        .unwrap();
    app.add_plugins(Material2dPlugin::<GfxPaletteLookupMaterial>::default());
}
