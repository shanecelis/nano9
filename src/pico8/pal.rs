use bevy::prelude::*;

#[derive(Debug, Clone, Reflect, Default)]
pub struct Palette {
    pub data: Vec<[u8; 4]>,
}

#[derive(thiserror::Error, Debug)]
pub enum PalError {
    #[error("no such palette {0}")]
    NoSuchPalette(usize),
    #[error("no such color {0}")]
    NoSuchColor(usize),
    #[error("no such color {color} in palette {palette}")]
    NoSuchPaletteColor { color: usize, palette: usize },
}

impl Palette {

    pub fn from_png_palette(bytes: &[u8]) -> Result<Option<Self>, png::DecodingError> {
        let cursor = std::io::Cursor::new(bytes);
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        Ok(Self::from_png_palette_info(&info))
    }

    pub fn from_png_palette_info(info: &png::Info<'static>) -> Option<Self> {
        info.palette.as_ref().map(|png_palette| {
            let colors = png_palette.chunks(3);
            let mut data = vec![[0x00, 0x00, 0x00, 0xff]; colors.len()];
            for (i, rgb) in colors.enumerate() {
                data[i][0..3].copy_from_slice(rgb);
            }
            Palette { data }
        })
    }

    pub fn from_image_column(image: &Image, column: u32) -> Self {
        let size = image.size();
        let mut data = vec![[0;4]; size.y as usize];
        for j in 0..size.y {
            let color: Srgba = image.get_color_at(column, j).unwrap().into();
            data[j as usize] = color.to_u8_array();
        }
        Palette { data }
    }

    pub fn from_image_row(image: &Image, row: u32) -> Self {
        let size = image.size();
        let mut data = vec![[0;4]; size.x as usize];
        for i in 0..size.x {
            let color: Srgba = image.get_color_at(i, row).unwrap().into();
            data[i as usize] = color.to_u8_array();
        }
        Palette { data }
    }

    pub fn from_image(image: &Image) -> Self {
        let size = image.size();
        let mut data = vec![[0;4]; (size.x * size.y) as usize];
        for j in 0..size.y {
            for i in 0..size.x {
                let color: Srgba = image.get_color_at(i, j).unwrap().into();
                data[(j * size.x + i) as usize] = color.to_u8_array();
            }
        }
        Palette { data }
    }

    pub fn from_slice(slice: &[[u8; 4]]) -> Self {
        Palette {
            data: Vec::from(slice),
        }
    }

    pub fn write_color(&self, index: usize, pixel_bytes: &mut [u8]) -> Result<(), PalError> {
        let data = self.data.get(index).ok_or(PalError::NoSuchColor(index))?;
        pixel_bytes.copy_from_slice(&data[0..pixel_bytes.len()]);
        Ok(())
    }

    pub fn get_color(&self, index: usize) -> Result<Srgba, PalError> {
        self.data
            .get(index)
            .ok_or(PalError::NoSuchColor(index))
            .map(|a| Srgba::rgba_u8(a[0], a[1], a[2], a[3]))
    }
}

#[derive(Default)]
struct PaletteLoader;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub enum PaletteSettings {
    #[default]
    FromImage,
    FromIndexPalette,
    FromRow(usize),
    FromColumn(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum PaletteError {
    // #[error("Could not read str: {0}")]
    // Utf8(#[from] std::str::Utf8Error),
    // #[error("io error: {0}")]
    // Io(#[from] std::io::Error),
    // #[error("Could not read asset: {0}")]
    // AssetBytes(#[from] bevy::asset::ReadAssetBytesError),
    // #[error("Decoding error: {0}")]
    // Decoding(#[from] png::DecodingError),
    // #[error("image ({image_size:?}) does not fit sprite size {sprite_size:?}")]
    // InvalidSpriteSize {
    //     image_size: UVec2,
    //     sprite_size: UVec2,
    // },
    // #[error("image ({image_size:?}) does not fit sprite counts {sprite_counts:?}")]
    // InvalidSpriteCounts {
    //     image_size: UVec2,
    //     sprite_counts: UVec2,
    // },
    // #[error("Could not load dependency: {0}")]
    // Load(#[from] bevy::asset::LoadDirectError),
    // #[error("Could not load cart: {0}")]
    // Cart(#[from] Box<pico8::CartLoaderError>),
    // #[error("Could not load image: {0}")]
    // Image(#[from] bevy::image::ImageLoaderError),
}

// impl AssetLoader for PaletteLoader {
//     type Asset = Palette;
//     type Settings = PaletteSettings;
//     type Error = PaletteError;

//     async fn load(
//         &self,
//         reader: &mut dyn Reader,
//         settings: &Self::Settings,
//         load_context: &mut LoadContext<'_>,
//     ) -> Result<Self::Asset, Self::Error> {
//         match settings {


//         }
//         let extension = load_context
//             .path()
//             .extension()
//             .and_then(|x| x.to_str())
//             .unwrap_or_default();
//         let index_color = settings.index_color.unwrap_or_else(|| extension == "p8");
//         let mut extract_palette = None;
//         let mut sprite_size = settings.sprite_size;
//         let (handle, layout_maybe, flags_maybe) = if index_color {
//             let mut bytes = Vec::new();
//             let _ = reader.read_to_end(&mut bytes).await?;
//             match extension {
//                 "p8" => {
//                     let settings = pico8::CartLoaderSettings::default();
//                     let parts = pico8::Cart::from_bytes(&bytes, &settings).map_err(Box::new)?;
//                     let gfx = parts.gfx.expect("no gfx in cart");
//                     let image_size = UVec2::new(gfx.width as u32, gfx.height as u32);
//                     sprite_size = Some(UVec2::splat(8));
//                     let layout = get_layout(image_size, &mut sprite_size, None, None, None)?
//                         .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));
//                     (
//                         pico8::SprHandle::Gfx(
//                             load_context.add_labeled_asset("gfx".to_string(), gfx),
//                         ),
//                         layout,
//                         Some(parts.flags),
//                     )
//                 }
//                 "png" => {
//                     let mut palette = pico8::Palette::default();
//                     let is_extract = settings.extract_palette;
//                     let gfx = Gfx::from_png(&bytes, is_extract.then_some(&mut palette))?;
//                     if is_extract {
//                         trace!("Extract palette from image {:?}", &palette);
//                         extract_palette = Some(palette);
//                     }
//                     let image_size = UVec2::new(gfx.width as u32, gfx.height as u32);
//                     let layout = get_layout(
//                         image_size,
//                         &mut sprite_size,
//                         settings.sprite_counts,
//                         settings.padding,
//                         settings.offset,
//                     )?
//                     .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));
//                     (
//                         pico8::SprHandle::Gfx(
//                             load_context.add_labeled_asset("gfx".to_string(), gfx),
//                         ),
//                         layout,
//                         None,
//                     )
//                 }
//                 x => {
//                     panic!(
//                         "Can't load {:?} with extension {x:?} as sprite sheet.",
//                         load_context.path().display()
//                     );
//                 }
//             }
//         } else {
//             let sampler = settings.sampler.clone().or_else(image_sampler);
//             let loader = bevy::image::ImageLoader::new(bevy::image::CompressedImageFormats::all());
//             let mut image_settings = ImageLoaderSettings::default();
//             if let Some(sampler) = &sampler {
//                 image_settings.sampler = sampler.clone();
//             }
//             let mut image_context = load_context.begin_labeled_asset();
//             let image = loader.load(reader, &image_settings, &mut image_context)
//                               .await?;
//             let image_size = image.size();
//             let loaded = image_context.finish(image);
//             let layout = get_layout(
//                 image_size,
//                 &mut sprite_size,
//                 settings.sprite_counts,
//                 settings.padding,
//                 settings.offset,
//             )?
//             .map(|layout| load_context.add_labeled_asset("atlas".to_string(), layout));

//             (
//                 pico8::SprHandle::Image(
//                     load_context.add_loaded_labeled_asset("image".to_string(), loaded),
//                 ),
//                 layout,
//                 None,
//             )
//         };
//         Ok(pico8::Palette {
//             handle,
//             sprite_size: sprite_size.expect("computed sprite size"),
//             flags: flags_maybe.unwrap_or(vec![]),
//             layout: layout_maybe.unwrap_or(Handle::default()),
//             palette: extract_palette,
//         })
//     }

//     fn extensions(&self) -> &[&str] {
//         // This can load "lua" files, but bevy_mod_scripting has a loader as
//         // well, so having it here generates a warning. We don't need to load
//         // .lua files ourselves, so we're dropping it.

//         // static EXTENSIONS: &[&str] = &["lua", "p8lua"];
//         static EXTENSIONS: &[&str] = &["png"];
//         EXTENSIONS
//     }
// }
