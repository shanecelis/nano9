mod memory_dir;

pub use memory_dir::*;
mod loader;
pub use loader::*;
pub mod front_matter;
use crate::{
    run::RunState,
    pico8::{self, Pico8Handle, Palettes},
};
use bevy::prelude::*;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
    event::Recipients,
    asset::{ScriptAsset}, script::ScriptComponent};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use merge2::Merge;

#[cfg(feature = "gameboy")]
pub mod gameboy;

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);

pub(crate) fn plugin(app: &mut App) {
    app
        .add_systems(Update, update_asset)
        .add_plugins(loader::plugin);
    #[cfg(feature = "gameboy")]
    app
        .add_plugins(gameboy::plugin);
}

// #[derive(Default, Debug, Clone, Deserialize, Serialize)]
// pub enum Code {
//     Path(String),
//     Content(String),
// }

/// Nano-9 config
#[derive(Debug, Clone, Deserialize, Serialize, Default, Merge, PartialEq)]
pub struct Config {
    /// Name of the game
    pub name: Option<String>,
    /// Target frames per second rate
    pub frames_per_second: Option<u8>,
    /// Description of game
    pub description: Option<String>,
    /// Nano-9 template, e.g., "pico8" or "gameboy"
    // #[toml_example(default = "pico8")]
    pub template: Option<String>,
    /// Author of game
    pub author: Option<String>,
    /// License of game
    pub license: Option<String>,
    /// Screen config
    pub screen: Option<Screen>,
    /// Defaults
    pub defaults: Option<Defaults>,
    /// Palettes
    #[serde(default, rename = "palette")]
    pub palettes: Vec<Palette>,
    // pub nearest_sampling: Option<bool>,
    /// Fonts
    #[serde(default, rename = "font")]
    pub fonts: Vec<Font>,
    /// Images
    #[serde(default, rename = "sprite_sheet")]
    pub sprite_sheets: Vec<SpriteSheet>,
    /// Scripts
    #[serde(default)]
    #[cfg(feature = "scripting")]
    // #[toml_example(default = "main.lua")]
    pub scripts: Vec<String>,
    /// Audio banks
    #[serde(default, rename = "audio_bank")]
    pub audio_banks: Vec<AudioBank>,
    /// Maps
    #[serde(default, rename = "map")]
    pub maps: Vec<SpriteMap>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, Merge, PartialEq)]
pub struct Defaults {
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
}

/// Audio bank
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
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
        a.into_iter().flatten().map(|x| x.as_str()).chain(b.into_iter())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Merge)]
pub struct Screen {
    #[merge(skip)]
    /// Canvas size, logical pixels, e.g., [128, 128] for pico8
    pub canvas_size: UVec2,
    /// Screen size, physical pixels, e.g., [512, 512] for pico8
    pub screen_size: Option<UVec2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Merge)]
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
    #[serde(default)]
    pub extract_palette: bool,

    // #[merge(skip)]
    // #[serde(default)]
    // pub palette: ImagePalette,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ImagePalette {

    #[serde(rename = "no-index")]
    #[default]
    /// This image is not to use an indexed color palette.
    NoIndex,
    #[serde(rename = "index")]
    /// This image uses an indexed color palette.
    Index,
    #[serde(rename = "extract")]
    /// Extract the palette and add it to the end of the existing palettes.
    Extract,
    // /// This image uses a particular color palette already specified.
    // Palette { index: usize },
}

/// Sprite map
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// #[serde(untagged)]
pub struct SpriteMap {
    /// Path to map, can have extensions .p8 or .tmx
    path: String,
}

/// Font
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Merge)]
pub struct Palette {
    /// Path to palette
    pub path: String,
    /// Specify the row of the palette to use
    pub row: Option<u32>,
}

pub fn update_asset(
    mut reader: EventReader<AssetEvent<pico8::Pico8Asset>>,
    assets: Res<Assets<pico8::Pico8Asset>>,
    mut next_state: ResMut<NextState<RunState>>,
    mut palettes: ResMut<Palettes>,
    mut pico8_handle: Option<ResMut<Pico8Handle>>,
    #[cfg(feature = "scripting")] mut commands: Commands,
    #[cfg(feature = "scripting")] scripts: ResMut<Assets<ScriptAsset>>,
) {
    for e in reader.read() {
        // TODO: This next line is a bit noisy but reveals a lot of asset
        // modifications due to Pico-8 APIs changing assets rather than changing
        // state, which might be my preference.

        // info!("update asset event {e:?}");
        if let AssetEvent::LoadedWithDependencies { id } = e {
            if let Some(ref mut pico8_handle) = pico8_handle {
                if let Some(pico8_asset) = assets.get(*id) {
                    if pico8_handle.handle.id() != *id {
                        warn!("Script loaded but does not match Pico8Handle.");
                        continue;
                    }
                    // Copy the palettes.
                    palettes.0 = pico8_asset.palettes.clone();
                    // XXX: It happens here too!
                    #[cfg(feature = "scripting")]
                    {
                        if !pico8_asset.scripts.is_empty() && pico8_handle.main_script.is_none() {
                            // pico8_handle.main_script = Some(Recipients::All);
                            // Spawn another script component for the libraries.
                            let entity = commands.spawn((Name::new("scripts"),
                                                         ScriptComponent(pico8_asset.scripts.clone()))).id();
                            info!("Add scripts to entity {}", &entity);
                            pico8_handle.main_script = Some(Recipients::Entity(entity));
                        }
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
            }),
            palettes: vec![Palette {
                path: pico8::PICO8_PALETTE.into(),
                row: None,
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
            }),
            ..default()
        }
    }

    /// The gameboy configuration
    #[cfg(feature = "gameboy")]
    pub fn gameboy() -> Self {
        Config {
            frames_per_second: Some(60),
            screen: Some(Screen {
                canvas_size: UVec2::new(240, 160),
                screen_size: Some(UVec2::new(480, 320)),
            }),
            palettes: vec![Palette {
                path: gameboy::PALETTES.into(),
                row: Some(15),
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
            }),
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_0() {
        let config: Config = toml::from_str(
            r#"
image = []
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
                row: None
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
        let config: Config = toml::from_str(
            r#"
frames_per_second = 70
blah = 7
"#,
        )
        .unwrap();
        assert_eq!(config.frames_per_second, Some(70));
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
        let mut a = Config::default();
        a.frames_per_second = Some(60);
        a.inject_template(Some("pico8")).unwrap();
        let mut b = Config::default();
        b.frames_per_second = Some(60);
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
extract_palette = true
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets[0].extract_palette, true);
    }
}
