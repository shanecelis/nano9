#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
}

struct GfxPaletteLookupMaterial {
    palette_size: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var index_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var palette_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var remap_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var transparency_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var tex_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> material: GfxPaletteLookupMaterial;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;
    let index_float = textureSample(index_texture, tex_sampler, uv).r;
    let index_u = u32(round(index_float * 255.0));
    let index_f = f32(index_u) / 256.0;

    let remapped_float = textureSample(remap_texture, tex_sampler, vec2(index_f, 0.5)).r;
    let remapped_u = u32(round(remapped_float * 255.0));
    let remapped_f = (f32(remapped_u) + 0.5) / 256.0;
    let palette_uv_x = (f32(remapped_u) + 0.5) / max(material.palette_size, 1.0);

    var color = textureSample(palette_texture, tex_sampler, vec2(palette_uv_x, 0.5));
    let trans = textureSample(transparency_texture, tex_sampler, vec2(remapped_f, 0.5)).r;
    if trans > 0.5 {
        color.a = 0.0;
    }
    return color;
}
