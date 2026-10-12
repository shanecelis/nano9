//! Edit the draw and display palette maps for an indexed sprite.
//!
//! The bird sheet is an R8 index texture. Colors come from an RGBA palette texture.
//! A draw map remaps each source index and stores transparency. A display map remaps
//! that result. Draw stamps a sprite that keeps its draw map. Display edits recolor
//! every stamped sprite. Each sprite samples `r = display[draw[i]]`, `g = 1` when
//! that sprite's draw map hides index `i`.
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
                    resolution: (1280, 960).into(),
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
    colors: Vec<Color>,
    selected: u8,
    /// Mode 0: source index -> palette index.
    draw: Vec<u8>,
    /// Mode 1: palette index -> palette index. Applied after `draw`.
    display: Vec<u8>,
    transparent: Vec<bool>,
    editing_display: bool,
    index: Handle<Image>,
    palette: Handle<Image>,
    checker: Handle<Image>,
    palette_width: u32,
    /// Unscaled sprite size in pixels.
    sheet: Vec2,
    drawn: usize,
}

/// A sprite stamped by the Draw button, removed by Reset.
#[derive(Component)]
struct StampedSprite;

/// Draw map captured when this sprite was drawn. Display edits recompose `lut` from it.
#[derive(Component)]
struct RetainedDraw {
    lut: Handle<Image>,
    draw: Vec<u8>,
    transparent: Vec<bool>,
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
struct DrawSwatch {
    index: u8,
}

#[derive(Component, Clone, Default)]
struct DisplaySwatch {
    index: u8,
}

#[derive(Component, Clone, Default)]
struct DrawnSwatch {
    index: u8,
}

#[derive(Component, Clone, Copy, Default)]
struct EditDraw;

#[derive(Component, Clone, Copy, Default)]
struct EditDisplay;

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

    let index = images.add(index_image);
    let palette_handle = images.add(palette_image);
    let checker = images.add(checker_texture());
    let sheet_px = Vec2::new(gfx.width as f32, gfx.height as f32);
    let sheet = sheet_px * 4.0;
    // Keep the sheet in the open area left of the panel.
    let sprite_pos = Vec3::new(-180.0, 24.0, 1.0);

    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: checker.clone(),
            custom_size: Some(sheet + Vec2::splat(32.0)),
            ..default()
        },
        Transform::from_translation(sprite_pos.with_z(0.0)),
    ));

    let debug = PalDebug {
        colors: colors.clone(),
        selected: 0,
        draw: identity_map(),
        display: identity_map(),
        transparent: initial_transparent(),
        editing_display: false,
        index,
        palette: palette_handle,
        checker,
        palette_width,
        sheet: sheet_px,
        drawn: 0,
    };
    spawn_bird(
        &mut commands,
        &mut images,
        &mut materials,
        &debug,
        sprite_pos,
        sheet,
        debug.draw.clone(),
        debug.transparent.clone(),
        false,
    );
    commands.insert_resource(debug);
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

fn identity_map() -> Vec<u8> {
    (0..LUT_LEN).map(|i| i as u8).collect()
}

fn initial_transparent() -> Vec<bool> {
    let mut transparent = vec![false; LUT_LEN];
    transparent[0] = true;
    transparent
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
                    @label("Draw and display palettes")
                ]
                --
                @pane_body() Children [
                    @label_dim("Click a source color, then a dest color. The sprite LUT is display[draw[i]].")
                    --
                    @label_dim("Draw stamps a sprite with the current draw map. Display edits recolor every sprite. Draw edits wait for the next Draw.")
                    --
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(12),
                        flex_wrap: FlexWrap::Wrap,
                        row_gap: px(8),
                    }
                    Children [
                        @FeathersButton {
                            @caption: bsn! { @caption("Edit draw") },
                        }
                        EditDraw
                        on(on_edit_draw)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Edit display") },
                        }
                        EditDisplay
                        on(on_edit_display)
                        --
                        @FeathersCheckbox {
                            @caption: bsn! { @caption("Transparent") },
                        }
                        TransparencyToggle
                        on(on_transparency)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Reset map") },
                        }
                        on(on_reset)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Draw") },
                        }
                        on(on_draw)
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Reset") },
                        }
                        on(on_reset_all)
                    ]
                    --
                    @label("draw  0 -> 0    draw[0] = 0    display[0] = 0    lut[0] = (0, 1)")
                    StatusText
                    --
                    @index_row(colors)
                    --
                    @source_row(colors)
                    --
                    @dest_row(colors)
                    --
                    @draw_row(colors)
                    --
                    @display_row(colors)
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
                width: px(76),
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

fn draw_row(colors: &[Color]) -> impl Scene {
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
                Outline {
                    width: px(2),
                    offset: px(1),
                    color: {Color::NONE},
                }
                DrawSwatch { index: {index} }
            }
        })
        .collect();
    swatch_row("draw", cells)
}

fn display_row(colors: &[Color]) -> impl Scene {
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
                Outline {
                    width: px(2),
                    offset: px(1),
                    color: {Color::NONE},
                }
                DisplaySwatch { index: {index} }
            }
        })
        .collect();
    swatch_row("display", cells)
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
    swatch_row("sprite", cells)
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

fn spawn_bird(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    materials: &mut Assets<GfxPaletteLookupMaterial>,
    debug: &PalDebug,
    pos: Vec3,
    size: Vec2,
    draw: Vec<u8>,
    transparent: Vec<bool>,
    stamped: bool,
) {
    let lut = images.add(identity_lut());
    write_lut(images, &lut, &draw, &transparent, &debug.display);
    let material = materials.add(GfxPaletteLookupMaterial {
        palette_texture: debug.palette.clone(),
        lut_texture: lut.clone(),
        palette_width: debug.palette_width,
        palette_height: 1,
        access_kind: 0,
        access_param: 0,
    });
    if stamped {
        commands.spawn((
            Sprite {
                image: debug.checker.clone(),
                custom_size: Some(size + Vec2::splat(16.0)),
                ..default()
            },
            Transform::from_translation(pos.with_z(0.0)),
            StampedSprite,
        ));
    }
    let mut sprite = commands.spawn((
        Sprite {
            image: debug.index.clone(),
            custom_size: Some(size),
            ..default()
        },
        SpriteMaterial(material),
        Transform::from_translation(pos),
        RetainedDraw {
            lut,
            draw,
            transparent,
        },
    ));
    if stamped {
        sprite.insert(StampedSprite);
    }
}

fn on_swatch(
    activate: On<Activate>,
    sources: Query<&SourceSwatch>,
    dests: Query<&DestSwatch>,
    sprites: Query<&RetainedDraw>,
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
    let src = debug.selected as usize;
    if debug.editing_display {
        debug.display[src] = dest.index;
        recompose_all(&mut images, &debug.display, &sprites);
    } else {
        debug.draw[src] = dest.index;
    }
}

fn on_draw(
    _activate: On<Activate>,
    mut commands: Commands,
    mut debug: ResMut<PalDebug>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
) {
    let stamp = debug.sheet * 2.0;
    let slot = debug.drawn;
    debug.drawn += 1;
    let gap = 16.0;
    let col = slot % 2;
    let row = slot / 2;
    // Sit above the live sheet, left of the panel.
    let left = -180.0 - debug.sheet.x * 2.0;
    let above = 24.0 + debug.sheet.y * 2.0;
    let x = left + stamp.x * 0.5 + col as f32 * (stamp.x + gap);
    let y = above + gap + stamp.y * 0.5 + row as f32 * (stamp.y + gap);
    spawn_bird(
        &mut commands,
        &mut images,
        &mut materials,
        &debug,
        Vec3::new(x, y, 1.0),
        stamp,
        debug.draw.clone(),
        debug.transparent.clone(),
        true,
    );
}

fn on_reset_all(
    _activate: On<Activate>,
    mut commands: Commands,
    mut debug: ResMut<PalDebug>,
    mut images: ResMut<Assets<Image>>,
    mut retained: Query<&mut RetainedDraw, Without<StampedSprite>>,
    stamps: Query<Entity, With<StampedSprite>>,
) {
    debug.draw = identity_map();
    debug.display = identity_map();
    debug.transparent = initial_transparent();
    debug.drawn = 0;
    for mut sprite in &mut retained {
        let lut = sprite.lut.clone();
        sprite.draw.clone_from(&debug.draw);
        sprite.transparent.clone_from(&debug.transparent);
        write_lut(
            &mut images,
            &lut,
            &sprite.draw,
            &sprite.transparent,
            &debug.display,
        );
    }
    for entity in &stamps {
        commands.entity(entity).despawn();
    }
}

fn on_edit_draw(_activate: On<Activate>, mut debug: ResMut<PalDebug>) {
    debug.editing_display = false;
}

fn on_edit_display(_activate: On<Activate>, mut debug: ResMut<PalDebug>) {
    debug.editing_display = true;
}

fn on_reset(
    _activate: On<Activate>,
    sprites: Query<&RetainedDraw>,
    mut debug: ResMut<PalDebug>,
    mut images: ResMut<Assets<Image>>,
) {
    if debug.editing_display {
        debug.display = identity_map();
        recompose_all(&mut images, &debug.display, &sprites);
    } else {
        debug.draw = identity_map();
    }
}

fn on_transparency(change: On<ValueChange<bool>>, mut debug: ResMut<PalDebug>) {
    let selected = debug.selected as usize;
    debug.transparent[selected] = change.value;
}

fn palette_color(colors: &[Color], index: u8) -> Color {
    colors
        .get(index as usize)
        .copied()
        .unwrap_or(Color::srgb(1.0, 0.0, 1.0))
}

fn recompose_all(images: &mut Assets<Image>, display: &[u8], sprites: &Query<&RetainedDraw>) {
    for sprite in sprites {
        write_lut(
            images,
            &sprite.lut,
            &sprite.draw,
            &sprite.transparent,
            display,
        );
    }
}

fn write_lut(
    images: &mut Assets<Image>,
    lut: &Handle<Image>,
    draw: &[u8],
    transparent: &[bool],
    display: &[u8],
) {
    let Some(mut image) = images.get_mut(lut) else {
        return;
    };
    let data = image.data.as_mut().expect("lut pixels");
    for i in 0..LUT_LEN {
        let mapped = draw[i] as usize;
        data[i * 2] = display[mapped];
        data[i * 2 + 1] = u8::from(transparent[i]);
    }
}

fn sync_from_lut(
    debug: Res<PalDebug>,
    mut sources: Query<
        (&SourceSwatch, &mut Outline),
        (
            Without<DestSwatch>,
            Without<DrawSwatch>,
            Without<DisplaySwatch>,
        ),
    >,
    mut dests: Query<
        (&DestSwatch, &mut Outline),
        (
            Without<SourceSwatch>,
            Without<DrawSwatch>,
            Without<DisplaySwatch>,
        ),
    >,
    mut draws: Query<
        (&DrawSwatch, &mut BackgroundColor, &mut Outline),
        (
            Without<DisplaySwatch>,
            Without<SourceSwatch>,
            Without<DestSwatch>,
        ),
    >,
    mut displays: Query<
        (&DisplaySwatch, &mut BackgroundColor, &mut Outline),
        (
            Without<DrawSwatch>,
            Without<SourceSwatch>,
            Without<DestSwatch>,
        ),
    >,
    mut drawn: Query<(&DrawnSwatch, &mut ColorSwatchValue)>,
    mut bits: Query<(&BitReadout, &mut Text), Without<StatusText>>,
    mut status: Query<&mut Text, (With<StatusText>, Without<BitReadout>)>,
    toggles: Query<(Entity, Has<Checked>), With<TransparencyToggle>>,
    mut commands: Commands,
) {
    let sel = debug.selected as usize;
    let draw_to = debug.draw[sel];
    let shown = debug.display[draw_to as usize];
    let target = if debug.editing_display {
        debug.display[sel]
    } else {
        draw_to
    };
    let transparent = debug.transparent[sel];

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
        let color = if swatch.index == target {
            Color::srgb(1.0, 0.82, 0.25)
        } else {
            Color::NONE
        };
        if outline.color != color {
            outline.color = color;
        }
    }
    for (swatch, mut background, mut outline) in &mut draws {
        let index = swatch.index as usize;
        let mapped = debug.draw[index];
        let mut color = palette_color(&debug.colors, mapped);
        if debug.transparent[index] {
            color = color.with_alpha(0.0);
        }
        if background.0 != color {
            background.0 = color;
        }
        let outline_color = if !debug.editing_display && swatch.index == debug.selected {
            Color::WHITE
        } else {
            Color::NONE
        };
        if outline.color != outline_color {
            outline.color = outline_color;
        }
    }
    for (swatch, mut background, mut outline) in &mut displays {
        let mapped = debug.display[swatch.index as usize];
        let color = palette_color(&debug.colors, mapped);
        if background.0 != color {
            background.0 = color;
        }
        let outline_color = if debug.editing_display && swatch.index == debug.selected {
            Color::WHITE
        } else {
            Color::NONE
        };
        if outline.color != outline_color {
            outline.color = outline_color;
        }
    }
    for (swatch, mut value) in &mut drawn {
        let index = swatch.index as usize;
        let mapped = debug.display[debug.draw[index] as usize];
        let mut color = palette_color(&debug.colors, mapped);
        if debug.transparent[index] {
            color = color.with_alpha(0.0);
        }
        if value.0 != color {
            value.0 = color;
        }
    }
    for (bit, mut text) in &mut bits {
        let hidden = debug.transparent[bit.index as usize];
        let next = if hidden { "1" } else { "0" };
        if text.0 != next {
            text.0 = next.to_string();
        }
    }
    let mode = if debug.editing_display {
        "display"
    } else {
        "draw"
    };
    let status_line = format!(
        "{mode}  {sel} -> {target}    draw[{sel}] = {draw_to}    display[{draw_to}] = {shown}    lut[{sel}] = ({shown}, {})",
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
