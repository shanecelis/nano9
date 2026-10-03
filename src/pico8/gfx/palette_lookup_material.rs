//! Sprite material that looks up palette colors from an index texture and a PalMap LUT.

use bevy::asset::{uuid_handle, Asset};
use bevy::reflect::Reflect;
use bevy::render::render_resource::*;
use bevy::shader::Shader;
use bevy::sprite_render::{AlphaMode2d, MaterialExtension2d, SpriteMaterialPlugin};
use bevy::{prelude::*, shader::ShaderRef};

use crate::pico8::pal::{self, Palette};

const GFX_PALETTE_LOOKUP_SHADER: Handle<Shader> =
    uuid_handle!("9f4e8a2b-1c3d-4e5f-6a7b-8c9d0e1f2a3b");

/// Extension sampled by the sprite renderer. The sprite's image is the R8 index texture.
/// Bindings start at 20 so they do not collide with the sprite material.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
#[reflect(Debug)]
pub struct GfxPaletteLookupMaterial {
    #[texture(20)]
    pub palette_texture: Handle<Image>,
    #[texture(21, sample_type = "u_int")]
    pub lut_texture: Handle<Image>,
    #[uniform(22)]
    pub palette_width: u32,
    #[uniform(22)]
    pub palette_height: u32,
    #[uniform(22)]
    pub access_kind: u32,
    #[uniform(22)]
    pub access_param: u32,
}

impl MaterialExtension2d for GfxPaletteLookupMaterial {
    fn fragment_shader() -> Option<ShaderRef> {
        Some(ShaderRef::Handle(GFX_PALETTE_LOOKUP_SHADER.clone()))
    }

    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        Some(AlphaMode2d::Blend)
    }
}

/// Build a palette-lookup sprite extension from a palette image and a packed PalMap LUT.
pub fn build_palette_lookup_material(
    palette_image: Handle<Image>,
    lut_image: Handle<Image>,
    palette: &Palette,
    palette_image_ref: Option<&Image>,
) -> GfxPaletteLookupMaterial {
    let (width, height) = if let Some(image) = palette_image_ref {
        let size = image.size();
        (size.x, size.y)
    } else {
        (16, 1)
    };
    let (access_kind, access_param) = pal::palette_access_to_gpu(&palette.access);
    GfxPaletteLookupMaterial {
        palette_texture: palette_image,
        lut_texture: lut_image,
        palette_width: width,
        palette_height: height,
        access_kind,
        access_param,
    }
}

pub(crate) fn gfx_palette_lookup_material_plugin(app: &mut App) {
    let mut shaders = app.world_mut().resource_mut::<Assets<Shader>>();
    shaders
        .insert(
            GFX_PALETTE_LOOKUP_SHADER.id(),
            Shader::from_wesl(
                include_str!("shaders/gfx_palette_lookup.wesl"),
                "shaders/gfx_palette_lookup.wesl",
            ),
        )
        .unwrap();
    app.add_plugins(SpriteMaterialPlugin::<GfxPaletteLookupMaterial>::default());
}
