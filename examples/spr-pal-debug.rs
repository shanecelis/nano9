//! Edit the draw and display palette maps, and stamp one spritesheet cell with `spr`.
//!
//! The bird image is an indexed spritesheet of 16×16 cells. Draw calls
//! `Pico8::spr` for one cell. That sprite keeps the draw map from the moment it
//! was stamped. Display edits recolor every stamped sprite. Draw edits and
//! transparency wait for the next Draw.
//!
//! ```sh
//! cargo run --example spr-pal-debug
//! ```

use bevy::picking::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::window::SystemCursorIcon;
use bevy_feathers::FeathersPlugins;
use bevy_feathers::containers::{pane, pane_body, pane_header};
use bevy_feathers::controls::{
    ButtonVariant, ColorSwatchValue, FeathersButton, FeathersCheckbox, FeathersColorSwatch,
};
use bevy_feathers::dark_theme::create_dark_theme;
use bevy_feathers::display::{caption, label, label_dim};
use bevy_feathers::rounded_corners::RoundedCorners;
use bevy_feathers::theme::UiTheme;
use bevy_input_focus::InputFocusPlugin;
use bevy_ui_widgets::{Activate, Button, ValueChange};
use nano9::pico8::Gfx;
use nano9::pico8::PalModify;
use nano9::prelude::*;

const SPRITE_PNG: &[u8] = include_bytes!("../assets/bird-sprite-indexed.png");
/// Second row of the 8-wide sheet, the same cell `indexed-sprite` starts from.
const SPRITE: usize = 8;
const LUT_LEN: usize = 256;
const CELL: f32 = 28.0;
const COLS: usize = 3;

fn main() {
    App::new()
        .add_plugins((Nano9Plugins, InputFocusPlugin, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (load_cart, setup))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .add_systems(
            Update,
            (
                draw_first_sprite.run_if(in_state(RunState::Run)),
                sync_from_lut,
            ),
        )
        .run();
}

fn load_cart(asset_server: Res<AssetServer>, mut commands: Commands) {
    let pico8_asset = asset_server.load::<Pico8Asset>("spr-pal-debug.toml");
    commands.insert_resource(Pico8Handle::from(pico8_asset));
}

#[derive(Resource)]
struct PalDebug {
    colors: Vec<Color>,
    selected: u8,
    /// Mode 0: source index -> palette index. Applied on the next `spr`.
    draw: Vec<u8>,
    /// Mode 1: palette index -> palette index. Applied after `draw`.
    display: Vec<u8>,
    transparent: Vec<bool>,
    editing_display: bool,
    drawn: usize,
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

fn setup(mut commands: Commands) {
    let mut palette = Vec::new();
    let gfx = Gfx::from_png(SPRITE_PNG, Some(&mut palette)).expect("indexed sprite png");
    assert!(!palette.is_empty(), "indexed png has a palette");
    let _ = gfx;

    let colors: Vec<Color> = palette
        .iter()
        .map(|c| Color::srgba_u8(c[0], c[1], c[2], c[3]))
        .collect();

    commands.insert_resource(PalDebug {
        colors: colors.clone(),
        selected: 0,
        draw: identity_map(),
        display: identity_map(),
        transparent: initial_transparent(),
        editing_display: false,
        drawn: 0,
    });
    commands.spawn_scene(debug_panel(&colors));
}

fn slot_pos(slot: usize) -> Vec2 {
    let col = (slot % COLS) as f32;
    let row = (slot / COLS) as f32;
    Vec2::new(4.0 + col * 20.0, 8.0 + row * 20.0)
}

fn stamp(pico8: &mut Pico8, slot: usize) -> bool {
    match pico8.spr(SPRITE, slot_pos(slot), None, None, None) {
        Ok(_) => true,
        Err(err) => {
            warn!("spr: {err}");
            false
        }
    }
}

fn draw_first_sprite(mut pico8: Pico8, state: Res<State<RunState>>, mut debug: ResMut<PalDebug>) {
    if **state != RunState::Run || debug.drawn != 0 {
        return;
    }
    if stamp(&mut pico8, 0) {
        debug.drawn = 1;
    }
}

fn identity_map() -> Vec<u8> {
    (0..LUT_LEN).map(|i| i as u8).collect()
}

fn initial_transparent() -> Vec<bool> {
    let mut transparent = vec![false; LUT_LEN];
    transparent[0] = true;
    transparent
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
                    @label_dim("Draw calls spr() for one cell of the sheet. Display edits recolor every sprite. Draw edits wait for the next Draw.")
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
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(1),
                        }
                        Children [
                            @FeathersButton {
                                @caption: bsn! { @caption("Edit draw") },
                                @variant: ButtonVariant::selected(true),
                                @corners: RoundedCorners::Left,
                            }
                            EditDraw
                            on(on_edit_draw)
                            --
                            @FeathersButton {
                                @caption: bsn! { @caption("Edit display") },
                                @variant: ButtonVariant::selected(false),
                                @corners: RoundedCorners::Right,
                            }
                            EditDisplay
                            on(on_edit_display)
                        ]
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

fn on_swatch(
    activate: On<Activate>,
    sources: Query<&SourceSwatch>,
    dests: Query<&DestSwatch>,
    mut debug: ResMut<PalDebug>,
    mut pico8: Pico8,
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
        pico8.pal_map(Some((src, dest.index as usize)), Some(PalModify::Present));
    } else {
        debug.draw[src] = dest.index;
        pico8.pal_map(Some((src, dest.index as usize)), Some(PalModify::Following));
    }
}

fn on_draw(_activate: On<Activate>, mut debug: ResMut<PalDebug>, mut pico8: Pico8) {
    let slot = debug.drawn;
    if stamp(&mut pico8, slot) {
        debug.drawn += 1;
    }
}

fn on_reset_all(_activate: On<Activate>, mut debug: ResMut<PalDebug>, mut pico8: Pico8) {
    pico8.pal_map(None, None);
    let _ = pico8.cls(None);
    debug.draw = identity_map();
    debug.display = identity_map();
    debug.transparent = initial_transparent();
    debug.drawn = if stamp(&mut pico8, 0) { 1 } else { 0 };
}

fn on_edit_draw(_activate: On<Activate>, mut debug: ResMut<PalDebug>) {
    debug.editing_display = false;
}

fn on_edit_display(_activate: On<Activate>, mut debug: ResMut<PalDebug>) {
    debug.editing_display = true;
}

fn on_reset(_activate: On<Activate>, mut debug: ResMut<PalDebug>, mut pico8: Pico8) {
    if debug.editing_display {
        debug.display = identity_map();
        pico8.pal_map(None, Some(PalModify::Present));
    } else {
        debug.draw = identity_map();
        debug.transparent = initial_transparent();
        pico8.pal_map(None, Some(PalModify::Following));
    }
}

fn on_transparency(change: On<ValueChange<bool>>, mut debug: ResMut<PalDebug>, mut pico8: Pico8) {
    let selected = debug.selected as usize;
    debug.transparent[selected] = change.value;
    pico8.palt(Some(selected), Some(change.value));
}

fn palette_color(colors: &[Color], index: u8) -> Color {
    colors
        .get(index as usize)
        .copied()
        .unwrap_or(Color::srgb(1.0, 0.0, 1.0))
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
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
    mut mode_buttons: Query<
        (&mut ButtonVariant, Has<EditDisplay>),
        Or<(With<EditDraw>, With<EditDisplay>)>,
    >,
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
    for (mut variant, is_display) in &mut mode_buttons {
        let next = ButtonVariant::selected(is_display == debug.editing_display);
        if *variant != next {
            *variant = next;
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
