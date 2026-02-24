// mod memory_dir;
// pub use memory_dir::*;
mod loader;
pub use loader::*;
pub mod front_matter;
use crate::{
    pico8::{self, Pico8Handle, canvas::N9Canvas},
    run::RunState,
};
use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, PresentMode, Window, WindowResolution, WindowResizeConstraints};
#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    asset::ScriptAsset,
    core::{event::Recipients, script::ScriptComponent},
};
use merge2::Merge;
use serde::{Deserialize, Serialize};

#[cfg(feature = "gameboy")]
pub mod gameboy;

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);
pub const DEFAULT_DECORATIONS: bool = true;

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, (update_asset, warn_load_failed::<crate::pico8::SpriteSheet>, warn_load_failed::<crate::pico8::Pico8Asset>))
        .add_plugins(loader::plugin);
    #[cfg(feature = "gameboy")]
    app.add_plugins(gameboy::plugin);
    app.init_resource::<KeyBindings>()
        .init_asset::<Config>()
        ;
}

/// Minimal plugin for headless tests: only asset loaders and asset types for
/// loading config / Pico8Asset. No update_asset, no window, no RenderApp.
pub fn headless_config_load_plugin(app: &mut App) {
    app.init_asset::<Config>()
        .init_asset::<crate::pico8::Pico8Asset>()
        .init_asset::<crate::pico8::SpriteSheet>()
        .add_plugins(loader::plugin);
}

// #[derive(Default, Debug, Clone, Deserialize, Serialize)]
// pub enum Code {
//     Path(String),
//     Content(String),
// }

/// Nano-9 config
#[derive(Debug, Clone, Deserialize, Serialize, Default, Merge, PartialEq, Reflect, Asset)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Name of the game
    pub name: Option<String>,
    /// Target frames per second rate
    pub frames_per_second: Option<u8>,
    /// Description of game
    pub description: Option<String>,
    /// Nano-9 template, e.g., "pico8" or "gameboy"
    pub template: Option<String>,
    /// Author of game
    pub author: Option<String>,
    /// License of game
    pub license: Option<String>,
    /// Screen config
    pub screen: Option<Screen>,
    /// Defaults
    pub defaults: Option<Defaults>,
    /// Bit depth
    pub bit_depth: Option<u8>,
    /// Palettes
    #[serde(default, rename = "palette")]
    pub palettes: Vec<Palette>,
    // pub nearest_sampling: Option<bool>,
    /// Fonts
    #[serde(default, rename = "font")]
    pub fonts: Vec<Font>,
    /// Images
    #[serde(default, rename = "sprite-sheet")]
    pub sprite_sheets: Vec<SpriteSheet>,
    /// Scripts
    #[serde(default)]
    #[cfg(feature = "scripting")]
    pub scripts: Vec<String>,
    /// Audio banks
    #[serde(default, rename = "audio-bank")]
    pub audio_banks: Vec<AudioBank>,
    /// Maps
    #[serde(default, rename = "map")]
    pub maps: Vec<SpriteMap>,
    /// Meshes
    #[serde(default, rename = "mesh")]
    pub meshes: Vec<Mesh>,
    /// Key bindings
    pub key_bindings: Option<KeyBindings>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, Merge, PartialEq, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Defaults {
    /// Initial palette
    pub initial_palette: Option<usize>,
    /// Initial pen color
    pub initial_pen_color: Option<usize>,
    /// Initial transparent color
    pub initial_transparent_color: Option<usize>,
    /// Clear color
    pub clear_color: Option<usize>,
    /// Font size when unspecified
    pub font_size: Option<f32>,
    /// Time to live, an entity caching system
    pub time_to_live: Option<u8>,
    /// Bit depth of canvas
    pub canvas_bit_depth: Option<u8>,
}

/// Key bindings for Pico-8 players.
///
/// This is an override-friendly representation: each field is `Option<Vec<KeyCode>>` so a user can
/// override only what they care about in `Nano9.toml`, while the Pico-8 template provides the full
/// defaults.
#[derive(Debug, Default, Clone, Deserialize, Serialize, Merge, PartialEq, Resource, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct KeyBindings {
    pub players: Vec<PlayerKeyBindings>,
}

impl KeyBindings {
    /// Returns the Pico-8 key bindings for two players.
    pub fn pico8() -> Self {
        use bevy::prelude::KeyCode::*;
        Self {
            players: vec![
                PlayerKeyBindings {
                    left: vec![ArrowLeft],
                    right: vec![ArrowRight],
                    up: vec![ArrowUp],
                    down: vec![ArrowDown],
                    o: vec![KeyZ, KeyC, KeyN, NumpadSubtract],
                    x: vec![KeyX, KeyV, KeyM, Numpad8],
                },
                PlayerKeyBindings {
                    left: vec![KeyS],
                    right: vec![KeyF],
                    up: vec![KeyE],
                    down: vec![KeyD],
                    o: vec![ShiftLeft, Tab],
                    x: vec![KeyA, KeyQ],
                },
            ],
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, Merge, PartialEq, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct PlayerKeyBindings {
    /// Button 0
    #[serde(default)]
    pub left: Vec<KeyCode>,
    /// Button 1
    #[serde(default)]
    pub right: Vec<KeyCode>,
    /// Button 2
    #[serde(default)]
    pub up: Vec<KeyCode>,
    /// Button 3
    #[serde(default)]
    pub down: Vec<KeyCode>,
    /// Button 4 (PICO-8 "O")
    #[serde(default)]
    pub o: Vec<KeyCode>,
    /// Button 5 (PICO-8 "X")
    #[serde(default)]
    pub x: Vec<KeyCode>,
}

/// Audio bank
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Reflect)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub enum AudioBank {
    /// Paths to audio files
    Paths { paths: Vec<String> },
    /// Path to audio file or audio bank
    Path { path: String },
}

impl AudioBank {
    /// Iterate through the paths
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        let (a, b) = match self {
            AudioBank::Paths { paths: v } => (Some(v), None),
            AudioBank::Path { path: s } => (None, Some(s.as_str())),
        };
        a.into_iter().flatten().map(|x| x.as_str()).chain(b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum ResizeConstraints {
    MatchScreen { match_screen: bool },
    Rect { rect: URect },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Merge, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Screen {
    #[merge(skip)]
    /// Canvas size, logical pixels, e.g., [128, 128] for pico8
    pub canvas_size: UVec2,
    /// Screen size, physical pixels, e.g., [512, 512] for pico8
    pub screen_size: Option<UVec2>,
    /// Resize constraints if any for the window
    pub resize_constraints: Option<ResizeConstraints>,
    /// Include title bar
    pub decorations: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Merge, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct SpriteSheet {
    /// Path to image
    pub path: String,
    /// Sprite size, .e.g, [16, 16] for 16x16 sprites
    pub sprite_size: Option<UVec2>,
    /// Sprite count, e.g., [8, 2] for 8 columns and 2 rows of sprites
    pub sprite_counts: Option<UVec2>,
    /// Padding between sprites
    pub padding: Option<UVec2>,
    /// Offset of initial sprite at top left
    pub offset: Option<UVec2>,
    /// Indexed sprite, if true it reads in the palette colors from the image
    /// and uses the current palette when the image is drawn
    pub index_color: Option<bool>,
    // #[merge(skip)]
    // #[serde(default)]
    // pub palette: ImagePalette,
}

/// Sprite map
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
// #[serde(untagged)]
pub struct SpriteMap {
    /// Path to map, can have extensions .p8 or .tmx
    path: String,
}

/// Font
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Font {
    /// Default font
    Default { default: bool },
    /// Font Path
    Path {
        /// Path to font
        path: String,
        /// Height of font
        height: Option<f32>,
    },
    // pub path: String,
    // pub height: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Merge, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Palette {
    /// Path to palette
    pub path: String,
    /// Specify the row of the palette to use.
    pub row: Option<u32>,
    /// Specify the column of the palette to use.
    pub column: Option<u32>,
    /// Extract palette from an indexed image.
    pub extract_index: Option<bool>,
}

impl Palette {
    #[allow(clippy::wrong_self_convention)]
    fn into_settings(&self) -> Option<pico8::PaletteSettings> {
        use pico8::PaletteSettings;
        if self.extract_index.unwrap_or(false) {
            Some(PaletteSettings::FromIndex)
        } else if let Some(row) = self.row {
            Some(PaletteSettings::FromRow(row))
        } else if let Some(column) = self.column {
            Some(PaletteSettings::FromColumn(column))
        } else {
            Some(PaletteSettings::FromImage)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Reflect)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum Mesh {
    Path { path: String },
    Cuboid { cuboid: [f32; 3] },
}

/// Warns when a sprite sheet fails to load (e.g. path wrong or file missing).
fn warn_load_failed<A>(
    mut reader: MessageReader<AssetLoadFailedEvent<A>>,
) where A: Asset {

    for e in reader.read() {
        warn!(
            "{} failed to load at path '{}': {}",
            std::any::type_name::<A>(),
            e.path,
            e.error
        );
    }
}

/// Applies config to the world (defaults, key bindings, canvas, framepace) and primary window.
/// Used on initial load and when config is hot-reloaded (watcher feature).
fn apply_config_to_world_and_window(
    config: &Config,
    commands: &mut Commands,
    primary_windows: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    config.was_plugin_build(commands);
    let window_spec = config.to_window();
    if let Some(mut window) = primary_windows.iter_mut().next() {
        trace!("Updating window");
        window.title = window_spec.title.clone();
        debug!("prior window resolution {:?}", &window.resolution);
        debug!("new window resolution {:?}", &window_spec.resolution);
        window.resolution.set(window_spec.resolution.physical_width() as f32,
                              window_spec.resolution.physical_height() as f32);
        // let scale_factor = window_spec.resolution.scale_factor();
        // window.resolution = WindowResolution::new(window_spec.resolution.physical_width() * scale_factor,
        //                                           window_spec.resolution.physical_height() * scale_factor)
        //     .with_scale_factor_override(scale_factor);
        window.resize_constraints = window_spec.resize_constraints;
        window.decorations = window_spec.decorations;
    } else {
        trace!("Spawning window");
        debug!("spawn with window resolution {:?}", &window_spec.resolution);
        commands.spawn((window_spec, PrimaryWindow));
    }
}

pub fn update_asset(
    mut reader: MessageReader<AssetEvent<pico8::Pico8Asset>>,
    assets: Res<Assets<pico8::Pico8Asset>>,
    configs: Res<Assets<Config>>,
    mut next_state: ResMut<NextState<RunState>>,
    mut pico8_handle: Option<ResMut<Pico8Handle>>,
    mut commands: Commands,
    mut primary_windows: Query<&mut Window, With<PrimaryWindow>>,
    #[cfg(feature = "scripting")] _scripts: ResMut<Assets<ScriptAsset>>,
) {
    for e in reader.read() {
        // TODO: This next line is a bit noisy but reveals a lot of asset
        // modifications due to Pico-8 APIs changing assets rather than changing
        // state, which might be my preference.

        info!("update asset event {e:?}");
        match e {
            AssetEvent::LoadedWithDependencies { id } => {
                if let Some(pico8_handle) = &mut pico8_handle {
                    if let Some(pico8_asset) = assets.get(*id) {
                        if pico8_handle.handle.id() != *id {
                            warn!("Script loaded but does not match Pico8Handle.");
                            continue;
                        }
                        // XXX: It happens here too!
                        #[cfg(feature = "scripting")]
                        {
                            if !pico8_asset.scripts.is_empty() && pico8_handle.main_script.is_none() {
                                // pico8_handle.main_script = Some(Recipients::All);
                                // Spawn another script component for the libraries.
                                let entity = commands
                                    .spawn((
                                        Name::new("scripts"),
                                        ScriptComponent(pico8_asset.scripts.clone()),
                                    ))
                                    .id();
                                info!("Add scripts to entity {}", &entity);
                                pico8_handle.main_script = Some(Recipients::AllContexts);
                            }
                        }
                        if let Some(config) = configs.get(&pico8_asset.config) {
                            apply_config_to_world_and_window(
                                config,
                                &mut commands,
                                &mut primary_windows,
                            );
                        }
                        info!("Goto Loaded state");
                        next_state.set(RunState::Loaded);
                    } else {
                        debug!("Pico8Asset not available for loaded {:?}.", id);
                    }
                } else {
                    warn!("Script loaded but no Pico8Handle is loaded.");
                }
            }
            #[cfg(feature = "watcher")]
            AssetEvent::Modified { id } => {
                // Config file changed; re-apply config to window and resources immediately.
                if let Some(pico8_handle) = &pico8_handle {
                    if pico8_handle.handle.id() != *id {
                        continue;
                    }
                    if let Some(pico8_asset) = assets.get(*id) {
                        if let Some(config) = configs.get(&pico8_asset.config) {
                            info!("Config changed, re-applying to window and resources");
                            apply_config_to_world_and_window(
                                config,
                                &mut commands,
                                &mut primary_windows,
                            );
                            commands.insert_resource(pico8::DespawnClearablesOnNextClear(true));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn run_pico8_when_loaded(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
) {
    match **state {
        RunState::Loaded => {
            info!("Goto Init state.");
            next_state.set(RunState::Init);
        }
        RunState::Init => {
            info!("Goto Run state.");
            next_state.set(RunState::Run);
        }
        _ => (),
    }
}

pub fn pause_pico8_when_loaded(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
) {
    match **state {
        RunState::Loaded => {
            next_state.set(RunState::Init);
        }
        RunState::Init => {
            next_state.set(RunState::Pause);
        }
        _ => (),
    }
}

impl std::str::FromStr for Config {
    type Err = ConfigError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str::<Config>(s)?)
    }
}

impl Config {
    /// The pico8 configuration
    pub fn pico8() -> Self {
        Config {
            frames_per_second: Some(30),
            screen: Some(Screen {
                canvas_size: UVec2::splat(128),
                screen_size: Some(UVec2::splat(512)),
                decorations: Some(true),
                resize_constraints: None,
            }),
            palettes: vec![Palette {
                path: pico8::PICO8_PALETTE.into(),
                row: None,
                column: None,
                extract_index: None,
            }],
            fonts: vec![Font::Path {
                path: pico8::PICO8_FONT.into(),
                height: None,
            }],
            defaults: Some(Defaults {
                font_size: Some(5.0),
                initial_pen_color: Some(6),
                clear_color: Some(0),
                initial_transparent_color: Some(0),
                time_to_live: Some(1),
                ..default()
            }),
            key_bindings: Some(KeyBindings::pico8()),
            ..default()
        }
    }

    /// The gameboy configuration
    #[cfg(feature = "gameboy")]
    pub fn gameboy() -> Self {
        Config {
            frames_per_second: Some(60),
            screen: Some(Screen {
                canvas_size: UVec2::new(160, 144),
                screen_size: Some(4 * UVec2::new(160, 144)),
                decorations: Some(true),
                resize_constraints: None,
            }),
            palettes: vec![Palette {
                path: gameboy::PALETTES.into(),
                row: Some(15),
                column: None,
                extract_index: None,
            }],
            fonts: vec![Font::Path {
                path: gameboy::FONT.into(),
                height: None,
            }],
            defaults: Some(Defaults {
                font_size: Some(5.0),
                initial_pen_color: Some(1),
                clear_color: Some(3),
                initial_transparent_color: None,
                time_to_live: Some(1),
                ..default()
            }),
            key_bindings: Some(KeyBindings::pico8()),
            ..default()
        }
    }

    pub fn inject_template(&mut self, template_name: Option<&str>) -> Result<(), ConfigError> {
        if let Some(template_name) = template_name.or(self.template.as_deref()) {
            let mut template = match template_name {
                #[cfg(feature = "gameboy")]
                "gameboy" => Config::gameboy(),
                "pico8" => Config::pico8(),
                x => {
                    return Err(ConfigError::InvalidTemplate(x.to_string()));
                }
            };
            self.merge(&mut template)
        }
        Ok(())
    }

    pub fn with_default_font(mut self) -> Self {
        if self.fonts.is_empty() {
            self.fonts.push(Font::Default { default: true });
        }
        self
    }

    /// Build a [`Window`] from this config's screen settings (resolution, title, decorations, etc.).
    pub fn to_window(&self) -> Window {
        let screen_size = self
            .screen
            .as_ref()
            .and_then(|s| s.screen_size)
            .unwrap_or(DEFAULT_SCREEN_SIZE);
        let decorations = self
            .screen
            .as_ref()
            .and_then(|s| s.decorations)
            .unwrap_or(DEFAULT_DECORATIONS);
        let resize_constraints = self
            .screen
            .as_ref()
            .and_then(|s| s.resize_constraints.clone())
            .unwrap_or(ResizeConstraints::MatchScreen {
                match_screen: false,
            });
        let resolution =
            WindowResolution::new(screen_size.x, screen_size.y);
        let resize_constraints = match resize_constraints {
            ResizeConstraints::MatchScreen { match_screen: true } => WindowResizeConstraints {
                min_width: resolution.width(),
                max_width: resolution.width(),
                min_height: resolution.height(),
                max_height: resolution.height(),
            },
            ResizeConstraints::MatchScreen {
                match_screen: false,
            } => WindowResizeConstraints::default(),
            ResizeConstraints::Rect { rect } => WindowResizeConstraints {
                min_width: rect.min.x as f32,
                max_width: rect.max.x as f32,
                min_height: rect.min.y as f32,
                max_height: rect.max.y as f32,
            },
        };
        Window {
            title: self.name.as_deref().unwrap_or("Nano-9").into(),
            present_mode: PresentMode::AutoVsync,
            resize_constraints,
            decorations,
            resolution,
            ..default()
        }
    }

    pub(crate) fn was_plugin_build(&self, commands: &mut Commands) {
        commands.insert_resource(
            self
                .defaults
                .as_ref()
                .map(pico8::Defaults::from_config)
                .unwrap_or_default(),
        );

        commands.insert_resource(self.key_bindings.clone().unwrap_or_default());

        let canvas_size: UVec2 = self
            .screen
            .as_ref()
            .map(|s| s.canvas_size)
            .unwrap_or(DEFAULT_CANVAS_SIZE);

        commands.insert_resource(N9Canvas {
            size: canvas_size,
            ..default()
        });

        if let Some(fps) = self.frames_per_second {
            info!("Set FPS {}", &fps);

            #[cfg(feature = "framepace")]
            {
                let limiter = bevy_framepace::Limiter::from_framerate(fps as f64);
                commands.insert_resource(
                    bevy_framepace::FramepaceSettings::default().with_limiter(limiter),
                );
            }
            // app.insert_resource(Time::<Fixed>::from_seconds(
            //     1.0 / fps as f64,
            // ));
        }

    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "level")]
    use std::path::PathBuf;

    #[test]
    fn test_config_0() {
        let config: Config = toml::from_str(
            r#"
sprite_sheet = []
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets.len(), 0);
        assert!(config.screen.is_none());
    }

    #[test]
    fn test_config_1() {
        let config: Config = toml::from_str(
            r#"
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [8, 8]
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, "sprites.png");
        assert_eq!(config.sprite_sheets[0].sprite_size, Some(UVec2::splat(8)));
    }

    #[test]
    fn test_palete_0() {
        let config: Config = toml::from_str(
            r#"
[[palette]]
path = "sprites.png"
"#,
        )
        .unwrap();
        assert_eq!(
            config.palettes,
            vec![Palette {
                path: "sprites.png".into(),
                row: None,
                column: None,
                extract_index: None,
            }]
        );
    }

    #[test]
    fn test_config_2() {
        let config: Config = toml::from_str(
            r#"
[screen]
canvas_size = [128,128]
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [8, 8]
"#,
        )
        .unwrap();
        assert_eq!(
            config.screen.map(|s| s.canvas_size),
            Some(UVec2::splat(128))
        );
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, "sprites.png");
        assert_eq!(config.sprite_sheets[0].sprite_size, Some(UVec2::splat(8)));
    }

    #[test]
    fn test_config_3() {
        let config: Config = toml::from_str(
            r#"
[[audio_bank]]
paths = ["blah.p8"]
"#,
        )
        .unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(
            config.audio_banks[0].paths().collect::<Vec<_>>(),
            vec!["blah.p8"],
        );
    }

    #[test]
    fn test_config_4() {
        let config: Config = toml::from_str(
            r#"
[[audio_bank]]
paths = [
"blah.mp3"
]
"#,
        )
        .unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(
            config.audio_banks[0].paths().collect::<Vec<_>>(),
            vec!["blah.mp3"],
        );
    }

    #[test]
    fn test_config_5() {
        let config: Config = toml::from_str(
            r#"
[[font]]
path = "blah.tff"
[[font]]
path = "dee.tff"
height = 3.0
[[font]]
default = true
"#,
        )
        .unwrap();
        assert_eq!(config.fonts.len(), 3);
        // assert_eq!(config.fonts[0].path, "blah.tff");
    }

    #[test]
    #[cfg(feature = "level")]
    fn test_config_6() {
        let config: Config = toml::from_str(
            r#"
[[map]]
path = "blah.ldtk"
[[map]]
path = "blah.p8"
"#,
        )
        .unwrap();
        assert_eq!(config.maps.len(), 2);
        assert_eq!(config.maps[0].path, PathBuf::from("blah.ldtk"));
    }

    #[test]
    fn test_config_7() {
        // I didn't know it would let other values through like this. Boo.
        assert!(
            toml::from_str::<Config>(
                r#"
frames_per_second = 70
blah = 7
"#,
            )
            .is_err()
        );
    }

    #[test]
    fn test_config_8() {
        // I didn't know it would let other values through like this. Boo.
        let config: Config = toml::from_str(
            r#"
[screen]
canvas_size = [128, 128]
screen_size = [512, 512]
"#,
        )
        .unwrap();
        assert!(config.screen.is_some());
        let screen = config.screen.unwrap();
        assert_eq!(screen.canvas_size, UVec2::splat(128));
        assert_eq!(screen.screen_size, Some(UVec2::splat(512)));
    }

    #[test]
    fn test_config_9() {
        // I didn't know it would let other values through like this. Boo.
        let config: Config = toml::from_str(
            r#"
[defaults]
font_size = 5
initial_pen_color = 6
initial_transparent_color = 7
clear_color = 8
"#,
        )
        .unwrap();
        assert!(config.defaults.is_some());
        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.font_size.unwrap(), 5.0);
        assert_eq!(defaults.initial_pen_color.unwrap(), 6);
        assert_eq!(defaults.initial_transparent_color.unwrap(), 7);
        assert_eq!(defaults.clear_color.unwrap(), 8);
    }

    #[test]
    fn test_inject0() {
        let mut a = Config::default();
        a.inject_template(Some("pico8")).unwrap();
        let mut b = Config::default();
        b.merge(&mut Config::pico8());
        assert_eq!(a, b);
    }

    #[test]
    fn test_inject1() {
        let mut a = Config {
            frames_per_second: Some(60),
            ..default()
        };
        a.frames_per_second = Some(60);
        a.inject_template(Some("pico8")).unwrap();
        let mut b = Config {
            frames_per_second: Some(60),
            ..default()
        };
        b.merge(&mut Config::pico8());
        assert_eq!(a, b);
    }

    #[test]
    fn test_image_palette0() {
        let config: Config = toml::from_str(
            r#"
[[sprite_sheet]]
path = "sprites.png"
index_color = true
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets[0].index_color, Some(true));
    }

    #[test]
    fn test_image_palette1() {
        let config: Config = toml::from_str(
            r#"
[[sprite_sheet]]
path = "sprites.png"
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets[0].index_color, None);
    }

    #[test]
    fn test_image_palette2() {
        let config: Config = toml::from_str(
            r#"
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [16,16]
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets[0].path, "sprites.png");
    }

    #[test]
    fn test_mesh0() {
        let config: Config = toml::from_str(
            r#"
[[mesh]]
path = "teapot.glb"
"#,
        )
        .unwrap();
        assert_eq!(
            config.meshes[0],
            Mesh::Path {
                path: "teapot.glb".into()
            }
        );
    }

    #[test]
    fn test_mesh1() {
        let config: Config = toml::from_str(
            r#"
[[mesh]]
cuboid = [0.1, 0.2, 0.3]
"#,
        )
        .unwrap();
        assert_eq!(
            config.meshes[0],
            Mesh::Cuboid {
                cuboid: [0.1, 0.2, 0.3]
            }
        );
    }

    #[test]
    fn test_unexpected_name() {
        assert!(
            toml::from_str::<Config>(
                r#"
[[mesh]]
cuboid = [0.1, 0.2, 0.3]
bad_name = 1
"#,
            )
            .is_err()
        );
    }
}
