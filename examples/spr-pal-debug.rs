//! Edit the palette-lookup texture that draws a sprite.
//!
//! The bird sheet is an R8 index texture. Colors come from an RGBA palette texture.
//! Swap and transparency live in a 256×1 `Rg8Uint` LUT: `r` is the remapped index, `g` is 1
//! when that source index is hidden. The feathers panel writes those bytes. The sprite samples
//! the same texture, so the draw follows the edits.
//!
//! ```sh
//! cargo run --example spr-pal-debug
//! ```

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::picking::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::Checked;
use bevy::ui_widgets::{Activate, Button, ValueChange};
use bevy::window::SystemCursorIcon;
use bevy_feathers::FeathersPlugins;
use bevy_feathers::containers::{pane, pane_body, pane_header};
use bevy_feathers::controls::{
    ColorSwatchValue, FeathersButton, FeathersCheckbox, FeathersColorSwatch,
};
use bevy_feathers::dark_theme::create_dark_theme;
use bevy_feathers::display::{caption, label, label_dim};
use bevy_feathers::theme::UiTheme;
use nano9::pico8::Gfx;
use nano9::pico8::palette_lookup_material::{
    GfxPaletteLookupMaterial, gfx_palette_lookup_material_plugin,
};

const SPRITE_PNG: &[u8] = include_bytes!("../assets/bird-sprite-indexed.png");
const LUT_LEN: usize = 256;
const CELL: f32 = 28.0;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "spr-pal-debug".into(),
                    resolution: (1280, 760).into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
            PaletteLookupPlugin,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.1)))
        .add_systems(Startup, setup)
        .add_systems(Update, sync_from_lut)
        .run();
}

struct PaletteLookupPlugin;

impl Plugin for PaletteLookupPlugin {
    fn build(&self, app: &mut App) {
        gfx_palette_lookup_material_plugin(app);
    }
}

#[derive(Resource)]
struct PalDebug {
    lut: Handle<Image>,
    colors: Vec<Color>,
    selected: u8,
}

#[derive(Component, Clone, Default)]
struct SourceSwatch {
    index: u8,
}

#[derive(Component, Clone, Default)]
struct DestSwatch {
    index: u8,
}

#[derive(Component, Clone, Default)]
struct DrawnSwatch {
    index: u8,
}

#[derive(Component, Clone, Default)]
struct BitReadout {
    index: u8,
}

#[derive(Component, Clone, Copy, Default)]
struct StatusText;

#[derive(Component, Clone, Copy, Default)]
struct TransparencyToggle;

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
) {
    let mut palette = Vec::new();
    let gfx = Gfx::from_png(SPRITE_PNG, Some(&mut palette)).expect("indexed sprite png");
    assert!(!palette.is_empty(), "indexed png has a palette");

    let colors: Vec<Color> = palette
        .iter()
        .map(|c| Color::srgba_u8(c[0], c[1], c[2], c[3]))
        .collect();
    let palette_image = palette_texture(&palette);
    let palette_width = palette_image.width();
    let index_image = gfx.to_index_image();
    let lut_image = identity_lut();

    let index = images.add(index_image);
    let palette_handle = images.add(palette_image);
    let lut = images.add(lut_image);
    let material = materials.add(GfxPaletteLookupMaterial {
        palette_texture: palette_handle,
        lut_texture: lut.clone(),
        palette_width,
        palette_height: 1,
        // LinearByRow: palette index i is texel (i, 0).
        access_kind: 0,
        access_param: 0,
    });

    let sheet = Vec2::new(gfx.width as f32, gfx.height as f32) * 4.0;
    // Keep the sheet in the open area left of the panel.
    let sprite_pos = Vec3::new(-180.0, 24.0, 1.0);

    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: images.add(checker_texture()),
            custom_size: Some(sheet + Vec2::splat(32.0)),
            ..default()
        },
        Transform::from_translation(sprite_pos.with_z(0.0)),
    ));
    commands.spawn((
        Sprite {
            image: index,
            custom_size: Some(sheet),
            ..default()
        },
        SpriteMaterial(material),
        Transform::from_translation(sprite_pos),
    ));

    commands.insert_resource(PalDebug {
        lut,
        colors: colors.clone(),
        selected: 0,
    });
    commands.spawn_scene(debug_panel(&colors));
}

fn palette_texture(colors: &[[u8; 4]]) -> Image {
    let bytes: Vec<u8> = colors.iter().flat_map(|c| c.iter().copied()).collect();
    let mut image = Image::new(
        Extent3d {
            width: colors.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Identity remap. Source index 0 starts transparent, same as Pico-8 `palt`.
fn identity_lut() -> Image {
    let mut bytes = vec![0u8; LUT_LEN * 2];
    for i in 0..LUT_LEN {
        bytes[i * 2] = i as u8;
        bytes[i * 2 + 1] = u8::from(i == 0);
    }
    let mut image = Image::new(
        Extent3d {
            width: LUT_LEN as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rg8Uint,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn checker_texture() -> Image {
    const N: u32 = 16;
    let mut bytes = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let light = (x / 4 + y / 4) % 2 == 0;
            let v = if light { 118 } else { 72 };
            bytes.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn debug_panel(colors: &[Color]) -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: px(16),
            right: px(16),
            width: px(560),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
        }
        Children [
            @pane() Children [
                @pane_header() Children [
                    @label("Sprite palette")
                ]
                --
                @pane_body() Children [
                    @label_dim("The sprite samples an R8 index, an RGBA palette, and this 256x1 Rg8Uint LUT.")
                    --
                    @label_dim("Click a source color, then a dest color, to store a -> b in lut.r.")
                    --
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                    }
                    Children [
                        @FeathersCheckbox {
                            @caption: bsn! { @caption("Transparent") },
                        }
                        TransparencyToggle
                        on(on_transparency)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Reset swap") },
                        }
                        on(on_reset)
                    ]
                    --
                    @label("0 -> 0    lut[0] = (0, 1)")
                    StatusText
                    --
                    @index_row(colors)
                    --
                    @source_row(colors)
                    --
                    @dest_row(colors)
                    --
                    @drawn_row(colors)
                    --
                    @bit_row(colors)
                ]
            ]
        ]
    }
}

fn swatch_row<C: Scene>(title: &'static str, cells: Vec<C>) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
        }
        Children [
            Node {
                width: px(64),
            }
            Children [
                @label(title)
            ]
            --
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(4),
                flex_wrap: FlexWrap::Wrap,
            }
            Children [
                {cells}
            ]
        ]
    }
}

fn index_row(colors: &[Color]) -> impl Scene {
    let cells: Vec<_> = colors
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let index = index as u8;
            bsn! {
                Node {
                    width: px(CELL),
                    height: px(18),
                    flex_shrink: 0.0,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                @label_dim(format!("{index}"))
            }
        })
        .collect();
    swatch_row("index", cells)
}

fn source_row(colors: &[Color]) -> impl Scene {
    let cells: Vec<_> = colors
        .iter()
        .copied()
        .enumerate()
        .map(|(index, color)| {
            let index = index as u8;
            bsn! {
                Node {
                    width: px(CELL),
                    height: px(CELL),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(4)),
                }
                BackgroundColor({color})
                Button
                Hovered
                EntityCursor::System(SystemCursorIcon::Pointer)
                Outline {
                    width: px(2),
                    offset: px(1),
                    color: {Color::NONE},
                }
                SourceSwatch { index: {index} }
                on(on_swatch)
            }
        })
        .collect();
    swatch_row("source", cells)
}

fn dest_row(colors: &[Color]) -> impl Scene {
    let cells: Vec<_> = colors
        .iter()
        .copied()
        .enumerate()
        .map(|(index, color)| {
            let index = index as u8;
            bsn! {
                Node {
                    width: px(CELL),
                    height: px(CELL),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px(4)),
                }
                BackgroundColor({color})
                Button
                Hovered
                EntityCursor::System(SystemCursorIcon::Pointer)
                Outline {
                    width: px(2),
                    offset: px(1),
                    color: {Color::NONE},
                }
                DestSwatch { index: {index} }
                on(on_swatch)
            }
        })
        .collect();
    swatch_row("dest", cells)
}

fn drawn_row(colors: &[Color]) -> impl Scene {
    let cells: Vec<_> = colors
        .iter()
        .copied()
        .enumerate()
        .map(|(index, color)| {
            let index = index as u8;
            bsn! {
                @FeathersColorSwatch
                Node {
                    width: px(CELL),
                    height: px(CELL),
                    flex_shrink: 0.0,
                }
                ColorSwatchValue({color})
                DrawnSwatch { index: {index} }
            }
        })
        .collect();
    swatch_row("drawn", cells)
}

fn bit_row(colors: &[Color]) -> impl Scene {
    let cells: Vec<_> = colors
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let index = index as u8;
            bsn! {
                Node {
                    width: px(CELL),
                    height: px(18),
                    flex_shrink: 0.0,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                @label("0")
                BitReadout { index: {index} }
            }
        })
        .collect();
    swatch_row("bit", cells)
}

fn on_swatch(
    activate: On<Activate>,
    sources: Query<&SourceSwatch>,
    dests: Query<&DestSwatch>,
    mut debug: ResMut<PalDebug>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Ok(source) = sources.get(activate.entity) {
        debug.selected = source.index;
        return;
    }
    let Ok(dest) = dests.get(activate.entity) else {
        return;
    };
    let src = debug.selected;
    write_remap(&mut images, &debug.lut, src, dest.index);
}

fn on_reset(_activate: On<Activate>, debug: Res<PalDebug>, mut images: ResMut<Assets<Image>>) {
    let Some(mut image) = images.get_mut(&debug.lut) else {
        return;
    };
    let data = image.data.as_mut().expect("lut pixels");
    for i in 0..LUT_LEN {
        data[i * 2] = i as u8;
    }
}

fn on_transparency(
    change: On<ValueChange<bool>>,
    debug: Res<PalDebug>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut image) = images.get_mut(&debug.lut) else {
        return;
    };
    let data = image.data.as_mut().expect("lut pixels");
    data[debug.selected as usize * 2 + 1] = u8::from(change.value);
}

fn write_remap(images: &mut Assets<Image>, lut: &Handle<Image>, src: u8, dst: u8) {
    let Some(mut image) = images.get_mut(lut) else {
        return;
    };
    let data = image.data.as_mut().expect("lut pixels");
    data[src as usize * 2] = dst;
}

fn sync_from_lut(
    debug: Res<PalDebug>,
    images: Res<Assets<Image>>,
    mut sources: Query<(&SourceSwatch, &mut Outline), Without<DestSwatch>>,
    mut dests: Query<(&DestSwatch, &mut Outline), Without<SourceSwatch>>,
    mut drawn: Query<(&DrawnSwatch, &mut ColorSwatchValue)>,
    mut bits: Query<(&BitReadout, &mut Text), Without<StatusText>>,
    mut status: Query<&mut Text, (With<StatusText>, Without<BitReadout>)>,
    toggles: Query<(Entity, Has<Checked>), With<TransparencyToggle>>,
    mut commands: Commands,
) {
    let Some(image) = images.get(&debug.lut) else {
        return;
    };
    let Some(data) = image.data.as_deref() else {
        return;
    };
    let sel = debug.selected as usize;
    if sel * 2 + 1 >= data.len() {
        return;
    }
    let remap = data[sel * 2];
    let transparent = data[sel * 2 + 1] != 0;

    for (swatch, mut outline) in &mut sources {
        let color = if swatch.index == debug.selected {
            Color::WHITE
        } else {
            Color::NONE
        };
        if outline.color != color {
            outline.color = color;
        }
    }
    for (swatch, mut outline) in &mut dests {
        let color = if swatch.index == remap {
            Color::srgb(1.0, 0.82, 0.25)
        } else {
            Color::NONE
        };
        if outline.color != color {
            outline.color = color;
        }
    }
    for (swatch, mut value) in &mut drawn {
        let i = swatch.index as usize * 2;
        let (mapped, hidden) = (data[i], data[i + 1] != 0);
        let mut color = debug
            .colors
            .get(mapped as usize)
            .copied()
            .unwrap_or(Color::srgb(1.0, 0.0, 1.0));
        if hidden {
            color = color.with_alpha(0.0);
        }
        if value.0 != color {
            value.0 = color;
        }
    }
    for (bit, mut text) in &mut bits {
        let hidden = data[bit.index as usize * 2 + 1] != 0;
        let next = if hidden { "1" } else { "0" };
        if text.0 != next {
            text.0 = next.to_string();
        }
    }
    let status_line = format!(
        "{sel} -> {remap}    lut[{sel}] = ({remap}, {})",
        u8::from(transparent)
    );
    for mut text in &mut status {
        if text.0 != status_line {
            text.0 = status_line.clone();
        }
    }
    if let Ok((entity, checked)) = toggles.single()
        && checked != transparent
    {
        if transparent {
            commands.entity(entity).insert(Checked);
        } else {
            commands.entity(entity).remove::<Checked>();
        }
    }
}
