import bevy_sprite_render::{
    mesh2d::vertex_output::VertexOutput,
    sprite_mesh::functions as sprite_fn,
};
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;

// access_kind: 0 = LinearByRow, 1 = LinearByColumn, 2 = FromRow, 3 = FromColumn
struct GfxPaletteLookupMaterial {
    palette_size: f32,
    palette_width: f32,
    palette_height: f32,
    access_kind: u32,
    access_param: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(20) var palette_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(21) var remap_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(22) var transparency_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(23) var tex_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(24) var<uniform> material: GfxPaletteLookupMaterial;

fn palette_index_to_uv(index: u32) -> vec2<f32> {
    let w = max(u32(material.palette_width), 1u);
    let h = max(u32(material.palette_height), 1u);
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
    return vec2<f32>(f32(x) + 0.5, f32(y) + 0.5) / vec2<f32>(f32(w), f32(h));
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // The sprite image is an R8 index texture.
    let index_float = sprite_fn::sample_sprite_texture(input.uv, input.instance_index).r;
    let index_u = u32(round(index_float * 255.0));
    let index_f = f32(index_u) / 256.0;

    let remapped_float = textureSample(remap_texture, tex_sampler, vec2(index_f, 0.5)).r;
    let remapped_u = u32(round(remapped_float * 255.0));
    let remapped_f = (f32(remapped_u) + 0.5) / 256.0;

    let palette_uv = palette_index_to_uv(remapped_u);
    var color = textureSample(palette_texture, tex_sampler, palette_uv);
    let trans = textureSample(transparency_texture, tex_sampler, vec2(remapped_f, 0.5)).r;
    if trans > 0.5 {
        color.a = 0.0;
    }
    var output_color = sprite_fn::get_final_color(color, input.instance_index);
@if(SRGB_OUTPUT)
    output_color = vec4(linear_to_srgb(output_color.rgb), output_color.a);
    return output_color;
}
