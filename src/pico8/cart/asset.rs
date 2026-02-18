use crate::pico8::{self, image::pixel_art_settings, *};
use crate::config::Config;
use bevy::asset::{AssetLoader, LoadContext, io::Reader};

use super::*;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::asset::{Language, ScriptAsset};

pub(crate) fn plugin(app: &mut App) {
    app.init_asset_loader::<P8AssetLoader>()
        .init_asset_loader::<PngAssetLoader>();

    // #[cfg(feature = "scripting")]
    // app
    //     .init_asset_loader::<P8LuaAssetLoader>()
    //     ;
}

#[derive(Default)]
struct P8AssetLoader;

impl AssetLoader for P8AssetLoader {
    type Asset = Pico8Asset;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;
        let mut cart = Cart::from_str(&content, settings)?;
        #[cfg(feature = "pico8-to-lua")]
        if let Some(patched_code) = translate_pico8_to_lua(&cart.lua, load_context).await? {
            cart.lua = patched_code;
        }
        // Ok(cart)
        // let cart = P8CartLoader.load(reader, settings, load_context).await?;
        log_lua_code(&cart.lua);
        to_asset(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

// #[cfg(feature = "scripting")]
// #[derive(Default)]
// struct P8LuaAssetLoader;

// #[cfg(feature = "scripting")]
// impl AssetLoader for P8LuaAssetLoader {
//     type Asset = ScriptAsset;
//     type Settings = CartLoaderSettings;
//     type Error = CartLoaderError;
//     async fn load(
//         &self,
//         reader: &mut dyn Reader,
//         settings: &CartLoaderSettings,
//         load_context: &mut LoadContext<'_>,
//     ) -> Result<Self::Asset, Self::Error> {
//         let cart = P8CartLoader.load(reader, settings, load_context).await?;
//         let code_path: PathBuf = load_context.path().into();
//         let code = cart.lua;
//         Ok(ScriptAsset {
//             content: code.into_bytes().into_boxed_slice(),
//             language: Language::Lua,
//         })
//     }

//     fn extensions(&self) -> &[&str] {
//         &["p8"]
//     }
// }
#[derive(Default)]
struct PngAssetLoader;

impl AssetLoader for PngAssetLoader {
    type Asset = Pico8Asset;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let cart = PngCartLoader.load(reader, settings, load_context).await?;
        to_asset(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

fn to_asset(cart: Cart, load_context: &mut LoadContext) -> Result<Pico8Asset, CartLoaderError> {
    let layout = load_context.add_labeled_asset(
        "atlas".into(),
        TextureAtlasLayout::from_grid(
            PICO8_SPRITE_SIZE,
            PICO8_TILE_COUNT.x,
            PICO8_TILE_COUNT.y,
            None,
            None,
        ),
    );
    let sprite_sheets: Vec<_> = cart
        .gfx
        .map(|gfx| load_context.add_labeled_asset("gfx".into(), gfx))
        .map(|gfx_handle| {
            load_context.add_labeled_asset(
                "sprite_sheet".into(),
                SpriteSheet {
                    handle: SprHandle::Gfx(gfx_handle),
                    palette: None,
                    sprite_size: UVec2::splat(8),
                    flags: cart.flags,
                    layout,
                },
            )
        })
        .into_iter()
        .collect();
    let code = cart.lua;
    // let code_path: PathBuf = load_context.path().into();
    let asset = Pico8Asset {
        #[cfg(feature = "scripting")]
        scripts: if cfg!(feature = "scripting") {
            vec![load_context.add_labeled_asset(
                "lua".into(),
                ScriptAsset {
                    content: code.into_bytes().into_boxed_slice(),
                    language: Language::Lua,
                },
            )]
        } else {
            vec![]
        },
        palettes: vec![Palette::from_slice(&PALETTE)].into(),
        border: load_context
            .loader()
            .with_settings(pixel_art_settings)
            .load(pico8::PICO8_BORDER),
        maps: vec![
            load_context
                .add_labeled_asset(
                    "map".to_string(),
                    P8Map {
                        entries: cart.map.clone(),
                    },
                )
                .into(),
        ],
        audio_banks: vec![{
            let bank = AudioBank(
                cart.sfx
                    .into_iter()
                    .enumerate()
                    .map(|(n, sfx)| {
                        Audio::Sfx(load_context.add_labeled_asset(format!("sfx{n}"), sfx))
                    })
                    .collect(),
            );
            load_context.add_labeled_asset("audio_bank".into(), bank)
        }],
        sprite_sheets,
        font: vec![N9Font {
            handle: load_context.load(PICO8_FONT),
        }],
        meshes: vec![],
        config: load_context.add_labeled_asset("config".into(), Config::pico8()),
    };
    Ok(asset)
}
