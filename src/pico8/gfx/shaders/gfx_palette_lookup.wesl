import bevy_sprite_render::{
    mesh2d::vertex_output::VertexOutput,
    sprite_mesh::functions as sprite_fn,
};
@if(BINDLESS)
import bevy_sprite_render::sprite_mesh::bindings::material_indices;
@if(BINDLESS)
import bevy_sprite_render::mesh2d::bindings::mesh;
@if(BINDLESS)
import bevy_render::bindless::bindless_textures_2d;
@if(!BINDLESS)
import bevy_sprite_render::sprite_mesh::bindings::texture;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;

// access_kind: 0 = LinearByRow, 1 = LinearByColumn, 2 = FromRow, 3 = FromColumn
struct GfxPaletteLookupMaterial {
    palette_width: u32,
    palette_height: u32,
    access_kind: u32,
    access_param: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(20) var palette_texture: texture_2d<f32>;
// Rg8Uint, 256×1: r = remapped index, g = 1 if transparent.
@group(constants::MATERIAL_BIND_GROUP) @binding(21) var lut_texture: texture_2d<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(22) var<uniform> material: GfxPaletteLookupMaterial;

/// Index texel after atlas, flip, and rect. `textureLoad` so filtering cannot blend indices.
fn load_sprite_index(uv: vec2<f32>, instance_index: u32) -> u32 {
    let final_uv = sprite_fn::get_final_uv(uv, instance_index);
@if(BINDLESS) {
    let slot = mesh[instance_index].material_bind_group_slot;
    let index_texture = bindless_textures_2d[material_indices[slot].texture];
    let texel = sprite_texel(final_uv, textureDimensions(index_texture));
    // R8Unorm loads as an exact multiple of 1/255.
    return u32(round(textureLoad(index_texture, texel, 0).r * 255.0));
} @else {
    let texel = sprite_texel(final_uv, textureDimensions(texture));
    return u32(round(textureLoad(texture, texel, 0).r * 255.0));
}
}

fn sprite_texel(uv: vec2<f32>, dims: vec2<u32>) -> vec2<u32> {
    let safe = max(dims, vec2<u32>(1u));
    let x = clamp(floor(uv.x * f32(safe.x)), 0.0, f32(safe.x - 1u));
    let y = clamp(floor(uv.y * f32(safe.y)), 0.0, f32(safe.y - 1u));
    return vec2<u32>(u32(x), u32(y));
}

fn palette_index_to_texel(index: u32) -> vec2<u32> {
    let w = max(material.palette_width, 1u);
    let h = max(material.palette_height, 1u);
    var x = 0u;
    var y = 0u;
    if material.access_kind == 0u {
        // LinearByRow: index -> (index % w, index / w)
        x = index % w;
        y = index / w;
    } else if material.access_kind == 1u {
        // LinearByColumn: index -> (index / h, index % h)
        x = index / h;
        y = index % h;
    } else if material.access_kind == 2u {
        // FromRow(row): index -> (index, access_param)
        x = index;
        y = material.access_param;
    } else {
        // FromColumn(col): index -> (access_param, index)
        x = material.access_param;
        y = index;
    }
    return vec2<u32>(min(x, w - 1u), min(y, h - 1u));
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let index_u = load_sprite_index(input.uv, input.instance_index);
    let lut = textureLoad(lut_texture, vec2<u32>(min(index_u, 255u), 0u), 0);
    let color = textureLoad(palette_texture, palette_index_to_texel(lut.r), 0);
    var output_color = sprite_fn::get_final_color(color, input.instance_index);
    if lut.g != 0u {
        output_color.a = 0.0;
    }
@if(SRGB_OUTPUT)
    output_color = vec4(linear_to_srgb(output_color.rgb), output_color.a);
    return output_color;
}
