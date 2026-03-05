use crate::pico8::{self, Gfx, Palette, SprHandle, image::image_sampler};
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    reflect::TypePath,
};

#[derive(Debug, Clone, Reflect, Asset)]
pub struct SpriteSheet {
    pub handle: SprHandle,
    pub layout: Handle<TextureAtlasLayout>,
    pub sprite_size: UVec2,
    pub flags: Vec<u8>,
    pub palette: Option<Palette>,
}

#[derive(Default, TypePath)]
pub struct SpriteSheetLoader;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SpriteSheetSettings {
    pub index_color: Option<bool>,
    pub extract_palette: bool,
    pub sprite_size: Option<UVec2>,
    pub sprite_counts: Option<UVec2>,
    pub padding: Option<UVec2>,
    pub offset: Option<UVec2>,
    pub sampler: Option<ImageSampler>,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<SpriteSheet>()
        .init_asset_loader::<SpriteSheetLoader>();
}

#[derive(Debug, thiserror::Error)]
pub enum SpriteSheetError {
    #[error("Could not read str: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not read asset: {0}")]
    AssetBytes(#[from] bevy::asset::ReadAssetBytesError),
    #[error("Decoding error: {0}")]
    Decoding(#[from] png::DecodingError),
    #[error("image ({image_size:?}) does not fit sprite size {sprite_size:?}")]
    InvalidSpriteSize {
        image_size: UVec2,
        sprite_size: UVec2,
    },
    #[error("image ({image_size:?}) does not fit sprite counts {sprite_counts:?}")]
    InvalidSpriteCounts {
        image_size: UVec2,
        sprite_counts: UVec2,
    },
    #[error("Could not load dependency: {0}")]
    Load(#[from] Box<bevy::asset::LoadDirectError>),
    #[error("Could not load cart: {0}")]
    Cart(#[from] Box<pico8::CartLoaderError>),
    #[error("Could not load image: {0}")]
    Image(#[from] bevy::image::ImageLoaderError),
}

impl AssetLoader for SpriteSheetLoader {
    type Asset = SpriteSheet;
    type Settings = SpriteSheetSettings;
    type Error = SpriteSheetError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let extension = load_context.path().get_full_extension().unwrap_or_default();
        let index_color = settings.index_color.unwrap_or_else(|| extension == "p8");
        let mut extract_palette = None;
        let mut sprite_size = settings.sprite_size;
        let (handle, layout_maybe, flags_maybe) = if index_color {
            let mut bytes = Vec::new();
            let _ = reader.read_to_end(&mut bytes).await?;
            match extension.as_str() {
                "p8" => {
                    let settings = pico8::CartLoaderSettings::default();
                    let parts = pico8::Cart::from_bytes(&bytes, &settings).map_err(Box::new)?;
                    let gfx = parts.gfx.expect("no gfx in cart");
                    let image_size = UVec2::new(gfx.width as u32, gfx.height as u32);
                    sprite_size = Some(UVec2::splat(8));
                    let layout = get_layout(image_size, &mut sprite_size, None, None, None)?
                        .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));
                    (
                        pico8::SprHandle::Gfx(
                            load_context.add_labeled_asset("gfx".to_string(), gfx),
                        ),
                        layout,
                        Some(parts.flags),
                    )
                }
                "png" => {
                    let mut palette_data = Vec::new();
                    let is_extract = settings.extract_palette;
                    let gfx = Gfx::from_png(&bytes, is_extract.then_some(&mut palette_data))?;
                    if is_extract && !palette_data.is_empty() {
                        let palette =
                            pico8::Palette::from_slice_with_context(&palette_data, load_context);
                        trace!("Extract palette from image {:?}", &palette);
                        extract_palette = Some(palette);
                    }
                    let image_size = UVec2::new(gfx.width as u32, gfx.height as u32);
                    let layout = get_layout(
                        image_size,
                        &mut sprite_size,
                        settings.sprite_counts,
                        settings.padding,
                        settings.offset,
                    )?
                    .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));
                    (
                        pico8::SprHandle::Gfx(
                            load_context.add_labeled_asset("gfx".to_string(), gfx),
                        ),
                        layout,
                        None,
                    )
                }
                x => {
                    panic!(
                        "Can't load {:?} with extension {x:?} as sprite sheet.",
                        load_context.path().path().display()
                    );
                }
            }
        } else {
            let sampler = settings.sampler.clone().or_else(image_sampler);
            let loader = bevy::image::ImageLoader::new(bevy::image::CompressedImageFormats::all());
            let mut image_settings = ImageLoaderSettings::default();
            if let Some(sampler) = &sampler {
                image_settings.sampler = sampler.clone();
            }
            let mut image_context = load_context.begin_labeled_asset();
            let image = loader
                .load(reader, &image_settings, &mut image_context)
                .await?;
            let image_size = image.size();
            let loaded = image_context.finish(image);
            let layout = get_layout(
                image_size,
                &mut sprite_size,
                settings.sprite_counts,
                settings.padding,
                settings.offset,
            )?
            .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));

            (
                pico8::SprHandle::Image(
                    load_context.add_loaded_labeled_asset("image".to_string(), loaded),
                ),
                layout,
                None,
            )
        };
        Ok(pico8::SpriteSheet {
            handle,
            sprite_size: sprite_size.expect("computed sprite size"),
            flags: flags_maybe.unwrap_or(vec![]),
            layout: layout_maybe.unwrap_or(Handle::default()),
            palette: extract_palette,
        })
    }

    fn extensions(&self) -> &[&str] {
        // This can load "lua" files, but bevy_mod_scripting has a loader as
        // well, so having it here generates a warning. We don't need to load
        // .lua files ourselves, so we're dropping it.

        // static EXTENSIONS: &[&str] = &["lua", "p8lua"];
        static EXTENSIONS: &[&str] = &["png", "p8"];
        EXTENSIONS
    }
}

fn get_layout(
    image_size: UVec2,
    sprite_size: &mut Option<UVec2>,
    sprite_counts: Option<UVec2>,
    padding: Option<UVec2>,
    offset: Option<UVec2>,
) -> Result<Option<TextureAtlasLayout>, SpriteSheetError> {
    if let Some((size, counts)) = sprite_size.zip(sprite_counts) {
        Ok(Some(TextureAtlasLayout::from_grid(
            size, counts.x, counts.y, padding, offset,
        )))
    } else if let Some(sprite_size) = *sprite_size {
        let counts = image_size / sprite_size;
        let remainders = image_size % sprite_size;
        if remainders == UVec2::ZERO {
            Ok(Some(TextureAtlasLayout::from_grid(
                sprite_size,
                counts.x,
                counts.y,
                padding,
                offset,
            )))
        } else {
            Err(SpriteSheetError::InvalidSpriteSize {
                image_size,
                sprite_size,
            })
        }
    } else if let Some(sprite_counts) = sprite_counts {
        let size = image_size / sprite_counts;
        *sprite_size = Some(size);
        let remainders = image_size % sprite_counts;
        if remainders == UVec2::ZERO {
            Ok(Some(TextureAtlasLayout::from_grid(
                size,
                sprite_counts.x,
                sprite_counts.y,
                padding,
                offset,
            )))
        } else {
            Err(SpriteSheetError::InvalidSpriteCounts {
                image_size,
                sprite_counts,
            })
        }
    } else {
        *sprite_size = Some(image_size);
        Ok(None)
    }
}
