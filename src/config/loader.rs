#[cfg(feature = "level")]
use crate::level::{self};
use crate::{
    config::{self, Mesh, *},
    pico8::{self, MeshHandle, PaletteAccess, Pico8Asset, image::pixel_art_settings},
};
use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, io::Reader},
    prelude::*,
};
#[cfg(feature = "level")]
use bevy_ecs_tiled::prelude::{TiledMapAsset, TiledWorldAsset};
#[cfg(feature = "scripting")]
use bevy_mod_scripting::asset::{Language, ScriptAsset};
use std::io;

pub(crate) fn plugin(app: &mut App) {
    app.init_asset_loader::<ConfigLoader>();
    app.init_asset_loader::<Pico8Loader>();
    #[cfg(feature = "scripting")]
    app.init_asset_loader::<LuaLoader>();
    #[cfg(feature = "scripting")]
    app.init_asset_loader::<P8LuaLoader>();
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not read str: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Could not read string: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
    #[error("Could not load dependency: {0}")]
    Load(#[from] Box<bevy::asset::LoadDirectError>),
    #[error("Could not read asset: {0}")]
    AssetBytes(#[from] bevy::asset::ReadAssetBytesError),
    #[error("Decoding error: {0}")]
    Decoding(#[from] png::DecodingError),
    #[error("image {image_index} ({image_size:?}) does not fit sprite size {sprite_size:?}")]
    InvalidSpriteSize {
        image_index: usize,
        image_size: UVec2,
        sprite_size: UVec2,
    },
    #[error("image {image_index} ({image_size:?}) does not fit sprite counts {sprite_counts:?}")]
    InvalidSpriteCounts {
        image_index: usize,
        image_size: UVec2,
        sprite_counts: UVec2,
    },
    #[error("invalid template {0:?}")]
    InvalidTemplate(String),
    #[error("include error: {0}")]
    Cart(#[from] pico8::CartLoaderError),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("asset path error: {0}")]
    AssetPath(#[from] bevy::asset::ParseAssetPathError),
}

#[derive(Default, TypePath)]
pub struct ConfigLoader;

impl AssetLoader for ConfigLoader {
    type Asset = Config;
    type Settings = ();
    type Error = ConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes).await?;
        let content = std::str::from_utf8(&bytes)?;
        let mut config: Config =
            toml::from_str::<Config>(content).map_err(|e| io::Error::other(format!("{e}")))?;
        config.inject_template(None)?;
        Ok(config)
        // into_asset(config, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &[];
        EXTENSIONS
    }
}

#[derive(Default, TypePath)]
pub struct Pico8Loader;

impl AssetLoader for Pico8Loader {
    type Asset = pico8::Pico8Asset;
    type Settings = ();
    type Error = ConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let config = ConfigLoader::load(&ConfigLoader, reader, settings, load_context).await?;
        // let mut bytes = Vec::new();
        // let _ = reader.read_to_end(&mut bytes).await?;
        // let content = std::str::from_utf8(&bytes)?;
        // let mut config: Config =
        //     toml::from_str::<Config>(content).map_err(|e| io::Error::other(format!("{e}")))?;
        // config.inect_template(None)?;
        into_asset(config, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["toml"];
        EXTENSIONS
    }
}

#[derive(Default, TypePath)]
pub struct LuaLoader;

#[derive(Default, TypePath)]
pub struct P8LuaLoader;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct LuaLoaderSettings {
    pub translate_pico8: Option<bool>,
}

#[cfg(feature = "scripting")]
impl AssetLoader for LuaLoader {
    type Asset = ScriptAsset;
    type Settings = LuaLoaderSettings;
    type Error = ConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;

        // TODO: We should read the config.
        //
        // We don't need config here. We need it at the beginning during App configuration.
        //
        // let config = if let Some(front_matter) = front_matter::LUA.parse_in_place(&mut content) {
        //     let mut config: Config = toml::from_str::<Config>(&front_matter)
        //         .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
        //     if let Some(template) = config.template.take() {
        //         config.inject_template(&template)?;
        //     }
        //     config
        // } else {
        //     Config::pico8()
        // };
        // let mut asset = into_asset(config, load_context).await?;
        // assert!(asset.scripts.is_none());

        let mut code = content;
        let translate = settings
            .translate_pico8
            .or(load_context
                .path()
                .get_full_extension()
                .map(|x| x == "p8lua"))
            .unwrap_or(false);
        if cfg!(feature = "pico8-to-lua") {
            if translate
                && let Some(patched_code) =
                    pico8::translate_pico8_to_lua(&code, load_context).await?
            {
                code = patched_code;
            }
        } else if translate {
            warn!("Pico-8 dialect translation requested but 'pico8-to-lua' feature not active.");
        }

        Ok(ScriptAsset {
            content: code.into_bytes().into_boxed_slice(),
            language: Language::Lua,
        })
    }

    fn extensions(&self) -> &[&str] {
        // This loader can load "lua" files, but `bevy_mod_scripting` has a
        // loader as well, so having it here generates a warning. We don't need
        // to load .lua files ourselves, so we don't.

        static EXTENSIONS: &[&str] = &["p8lua"];
        EXTENSIONS
    }
}

#[cfg(feature = "scripting")]
impl AssetLoader for P8LuaLoader {
    type Asset = Pico8Asset;
    type Settings = LuaLoaderSettings;
    type Error = ConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes).await?;
        let mut content = String::from_utf8(bytes)?;

        // TODO: We should read the config.
        //
        // We don't need config here. We need it at the beginning during App configuration.
        //
        let config = if let Some(front_matter) = front_matter::LUA.parse_in_place(&mut content) {
            let mut config: Config = toml::from_str::<Config>(&front_matter)
                .map_err(|e| io::Error::other(format!("{e}")))?;
            let template = config.template.clone();
            config.inject_template(template.as_deref())?;
            config
        } else {
            Config::pico8()
        };
        let mut asset = into_asset(config, load_context).await?;

        // TODO: This is a copy of LuaLoader, perhaps we should try to call it
        // instead of copying its body.
        let mut code = content;
        let translate = settings
            .translate_pico8
            .or(load_context
                .path()
                .get_full_extension()
                .map(|x| x == "p8lua"))
            .unwrap_or(false);
        if cfg!(feature = "pico8-to-lua") {
            if translate
                && let Some(patched_code) =
                    pico8::translate_pico8_to_lua(&code, load_context).await?
            {
                code = patched_code;
            }
        } else if translate {
            warn!("Pico-8 dialect translation requested but 'pico8-to-lua' feature not active.");
        }

        let script_handle = load_context.add_labeled_asset(
            "script".into(),
            ScriptAsset {
                content: code.into_bytes().into_boxed_slice(),
                language: Language::Lua,
            },
        );
        asset.scripts.push(script_handle);
        Ok(asset)
    }

    fn extensions(&self) -> &[&str] {
        // This loader can load "lua" files, but `bevy_mod_scripting` has a
        // loader as well, so having it here generates a warning. We don't need
        // to load .lua files ourselves, so we don't.

        static EXTENSIONS: &[&str] = &["p8lua", "lua"];
        EXTENSIONS
    }
}

async fn into_asset(
    config: Config,
    load_context: &mut LoadContext<'_>,
) -> Result<Pico8Asset, ConfigError> {
    let whole_config = config.clone();
    let palettes = if config.palettes.is_empty() {
        warn!("No palettes were provided.");
        Vec::new()
    } else {
        let mut palettes = Vec::with_capacity(config.palettes.len());
        for palette in config.palettes.iter() {
            let palette_settings = palette.into_settings().expect("palette");
            use pico8::PaletteSettings;
            if matches!(palette_settings, PaletteSettings::FromIndex) {
                // Extract palette from indexed PNG and add strip as direct dependency of Pico8Asset
                // so the palette image resolves in Assets<Image>.
                let path = AssetPath::try_parse(&palette.path)?;
                let bytes = load_context.read_asset_bytes(path).await?;
                let data = pico8::Palette::from_png_palette(&bytes)
                    .map_err(ConfigError::from)?
                    .ok_or_else(|| {
                        ConfigError::Message("No color palette, not an indexed image".into())
                    })?;
                let strip = pico8::strip_image_from_data(&data);
                let label = format!("palette_{}", palettes.len());
                let image_handle = load_context.add_labeled_asset(label, strip);
                palettes.push(pico8::Palette {
                    image: image_handle,
                    access: PaletteAccess::default(),
                });
            } else {
                // Load palette image as a direct dependency of Pico8Asset so it resolves in Assets<Image>.
                let path = AssetPath::try_parse(&palette.path)?;
                let image_handle = load_context.load::<Image>(path);
                let access = match palette_settings {
                    PaletteSettings::FromImage => PaletteAccess::LinearByRow,
                    PaletteSettings::FromRow(r) => PaletteAccess::FromRow(r),
                    PaletteSettings::FromColumn(c) => PaletteAccess::FromColumn(c),
                    PaletteSettings::FromIndex => unreachable!(),
                };
                palettes.push(pico8::Palette {
                    image: image_handle,
                    access,
                });
            }
            info!("added palette, now have {}", palettes.len());
        }
        palettes
    };
    let mut sprite_sheets = vec![];
    for sheet in config.sprite_sheets.into_iter() {
        let asset_path = AssetPath::try_parse(&sheet.path)?;
        let handle = if asset_path
            .path()
            .extension()
            .map(|ext| ext == "p8")
            .unwrap_or(false)
        {
            load_context
                .loader()
                .load::<pico8::SpriteSheet>(&asset_path)
        } else {
            load_context
                .loader()
                .with_settings(move |settings: &mut pico8::SpriteSheetSettings| {
                    settings.index_color = sheet.index_color;
                    settings.sprite_size = sheet.sprite_size;
                    settings.sprite_counts = sheet.sprite_counts;
                    settings.padding = sheet.padding;
                    settings.offset = sheet.offset;
                    settings.offset = sheet.offset;
                    // TODO: Provide sampler sampler?
                })
                .load::<pico8::SpriteSheet>(&asset_path)
        };
        // We don't account for palettes here.
        sprite_sheets.push(handle);
    }
    #[cfg(feature = "scripting")]
    let mut scripts = vec![];
    #[cfg(feature = "scripting")]
    for p in config.scripts.into_iter() {
        // Load them in order.
        let asset_path = AssetPath::try_parse(&p)?;
        let handle = load_context.load::<ScriptAsset>(&asset_path);
        scripts.push(handle);
    }

    let mut maps: Vec<pico8::SpriteMap> = Vec::with_capacity(config.maps.len());
    for map in config.maps.into_iter() {
        let asset_path = AssetPath::try_parse(&map.path)?;
        match asset_path
            .path()
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("")
        {
            "p8" => {
                let p8map: Handle<pico8::P8Map> = load_context.load::<pico8::P8Map>(map.path);
                maps.push(p8map.into());
            }
            #[cfg(feature = "level")]
            "tmx" => {
                let handle = load_context.load::<TiledMapAsset>(map.path);
                maps.push(pico8::SpriteMap::Level(level::Tiled::SpriteMap { handle }));
            }
            #[cfg(feature = "level")]
            "world" => {
                let handle = load_context.load::<TiledWorldAsset>(map.path);
                maps.push(pico8::SpriteMap::Level(level::Tiled::World { handle }));
            }
            x => {
                panic!("Unexpected map extension {x:?}");
            }
        }
    }
    let mut audio_banks = vec![];
    for (i, bank) in config.audio_banks.into_iter().enumerate() {
        let mut items = vec![];
        for (j, p) in bank.paths().enumerate() {
            let mut asset_path = AssetPath::try_parse(p)?.into_owned();
            let label = asset_path.take_label();
            let erased_loaded = load_context
                .loader()
                .with_unknown_type()
                .immediate()
                .load(&asset_path)
                .await
                .map_err(Box::new)?;
            match erased_loaded.downcast::<pico8::audio::AudioBank>() {
                Ok(loaded) => {
                    let audio_bank = loaded.take();
                    items.extend(audio_bank.0);
                }
                Err(erased_loaded) => match erased_loaded.downcast::<AudioSource>() {
                    Ok(loaded) => {
                        items.push(pico8::audio::Audio::AudioSource(
                            load_context.add_loaded_labeled_asset(
                                format!("audio_bank{}sfx{}", i, j),
                                loaded,
                            ),
                        ));
                    }
                    Err(erased_loaded) => {
                        if let Some(erased_asset) = label.and_then(|label| {
                            info!("getting label {}", &label);
                            erased_loaded.get_labeled(label)
                        }) {
                            if let Some(audio_bank) = erased_asset.get::<pico8::audio::AudioBank>()
                            {
                                items.extend(audio_bank.clone().0);
                            } else {
                                error!(
                                    "unable to add {:?} to audio bank, label had asset type {}",
                                    p,
                                    erased_asset.asset_type_name()
                                );
                            }
                        } else {
                            error!(
                                "unable to add {:?} to audio bank, has asset type {}",
                                p,
                                erased_loaded.asset_type_name()
                            );
                        }
                    }
                },
            }
        }
        audio_banks.push(
            load_context
                .add_labeled_asset(format!("audio_bank{}", i), pico8::audio::AudioBank(items)),
        );
    }

    let mut meshes = vec![];
    for (i, mesh) in config.meshes.into_iter().enumerate() {
        let handle = match mesh {
            Mesh::Path { path } => MeshHandle::Gltf(load_context.load::<bevy::gltf::Gltf>(path)),
            Mesh::Cuboid { cuboid: size } => {
                let cuboid = Cuboid::new(size[0], size[1], size[2]);
                let mesh = cuboid.mesh().build();
                MeshHandle::Mesh(load_context.add_labeled_asset(format!("mesh{}", i), mesh))
            }
        };
        meshes.push(handle);
    }

    let state = pico8::Pico8Asset {
        #[cfg(feature = "scripting")]
        scripts,
        palettes: palettes.into(),
        border: load_context
            .loader()
            .with_settings(pixel_art_settings)
            .load(crate::config::pico8::BORDER),
        maps,
        audio_banks,
        sprite_sheets,
        font: config
            .fonts
            .into_iter()
            .map(|font| match font {
                config::Font::Default { default: yes } if yes => pico8::N9Font {
                    handle: TextFont::default().font,
                },
                config::Font::Path { path, height: _ } => pico8::N9Font {
                    handle: load_context.load(path),
                },
                config::Font::Default { .. } => {
                    panic!("Must use a path if not default font.")
                }
            })
            .collect::<Vec<_>>(),
        meshes,
        config: load_context.add_labeled_asset("config".into(), whole_config),
    };
    Ok(state)
}
