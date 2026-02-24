use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy::asset::RenderAssetUsages;

#[derive(Asset, Debug, Clone, Reflect)]
pub struct Palette {
    /// Canonical palette as a 1×N or N×1 RGBA image.
    pub image: Handle<Image>,
    pub access: PaletteAccess,
}

#[derive(Asset, Debug, Clone, Reflect)]
/// How are the indices for the palette accessed?
pub enum PaletteAccess {
    #[default]
    /// Access the palette linearly by row so the sequence would be:
    /// color 0 -> pixel (0,0)
    /// color 1 -> pixel (1,0)
    /// color w -> pixel (0,1)
    ///
    /// Palette length is image width times height.
    LinearByRow,
    /// Access the palette linearly by column so the sequence would be:
    /// color 0 -> pixel (0,0)
    /// color 1 -> pixel (0,1)
    /// color h -> pixel (1,0)
    ///
    /// Palette length is image width times height.
    LinearByColumn,
    /// Only use the specified row. Palette length is image width.
    FromRow(u32),
    /// Only use the specified column. Palette length is image height.
    FromColumn(u32),
}

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum PalError {
    #[error("no such palette {index} only {count} palettes")]
    NoSuchPalette { index: usize, count: usize },
    #[error("no such color {0}")]
    NoSuchColor(usize),
    #[error("no such color {color} in palette {palette}")]
    NoSuchPaletteColor { color: usize, palette: usize },
    #[error("no Pico8Asset")]
    NoPico8Asset,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<Palette>()
        .init_asset_loader::<PaletteLoader>();
}

/// Build a 1×N RGBA strip image from palette data.
pub(crate) fn strip_image_from_data(data: &[[u8; 4]]) -> Image {
    let n = data.len();
    let pixel_bytes: Vec<u8> = data.iter().flat_map(|c| c.iter().copied()).collect();
    let mut image = Image::new(
        Extent3d {
            width: n as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixel_bytes,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

/// Read palette strip (first row) from a 1×N or N×1 image into a vec of RGBA.
pub fn palette_data_from_image(image: &Image) -> Vec<[u8; 4]> {
    let size = image.size();
    let n = (size.x * size.y) as usize;
    let mut data = Vec::with_capacity(n);
    for j in 0..size.y {
        for i in 0..size.x {
            if let Ok(color) = image.get_color_at(i, j) {
                let srgba: Srgba = color.into();
                data.push(srgba.to_u8_array());
            }
        }
    }
    data
}

impl Palette {
    /// Reference to the palette image handle.
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    /// Number of colors when read from the given image (width of a 1×N strip).
    pub fn len_in(&self, image: &Image) -> usize {
        image.size().x as usize
    }

    /// Get color at index using the loaded palette image.
    pub fn get_color_in(&self, index: usize, image: &Image) -> Result<Srgba, PalError> {
        let size = image.size();
        let n = (size.x * size.y) as usize;
        if index >= n {
            return Err(PalError::NoSuchColor(index));
        }
        let x = (index as u32) % size.x;
        let y = (index as u32) / size.x;
        image
            .get_color_at(x, y)
            .map(|c| c.into())
            .map_err(|_| PalError::NoSuchColor(index))
    }

    /// Write color at index into pixel_bytes using the loaded palette image.
    pub fn write_color_in(
        &self,
        index: usize,
        image: &Image,
        pixel_bytes: &mut [u8],
    ) -> Result<(), PalError> {
        let color = self.get_color_in(index, image)?;
        let arr = color.to_u8_array();
        pixel_bytes.copy_from_slice(&arr[0..pixel_bytes.len()]);
        Ok(())
    }

    pub fn from_png_palette(bytes: &[u8]) -> Result<Option<Vec<[u8; 4]>>, png::DecodingError> {
        let cursor = std::io::Cursor::new(bytes);
        let decoder = png::Decoder::new(cursor);
        let reader = decoder.read_info()?;
        let info = reader.info();
        Ok(Self::from_png_palette_info(info))
    }

    pub fn from_png_palette_info(info: &png::Info<'static>) -> Option<Vec<[u8; 4]>> {
        info.palette.as_ref().map(|png_palette| {
            let colors = png_palette.chunks(3);
            let mut data = vec![[0x00, 0x00, 0x00, 0xff]; colors.len()];
            for (i, rgb) in colors.enumerate() {
                data[i][0..3].copy_from_slice(rgb);
            }
            data
        })
    }

    pub fn from_image_column(image: &Image, column: u32) -> Vec<[u8; 4]> {
        let size = image.size();
        let mut data = vec![[0; 4]; size.y as usize];
        for j in 0..size.y {
            let color: Srgba = image.get_color_at(column, j).unwrap().into();
            data[j as usize] = color.to_u8_array();
        }
        data
    }

    pub fn from_image_row(image: &Image, row: u32) -> Vec<[u8; 4]> {
        let size = image.size();
        let mut data = vec![[0; 4]; size.x as usize];
        for i in 0..size.x {
            let color: Srgba = image.get_color_at(i, row).unwrap().into();
            data[i as usize] = color.to_u8_array();
        }
        data
    }

    pub fn from_image(image: &Image) -> Vec<[u8; 4]> {
        let size = image.size();
        let mut data = vec![[0; 4]; (size.x * size.y) as usize];
        for j in 0..size.y {
            for i in 0..size.x {
                let color: Srgba = image.get_color_at(i, j).unwrap().into();
                data[(j * size.x + i) as usize] = color.to_u8_array();
            }
        }
        data
    }

    /// Palette from slice, creating the strip image and adding it via `load_context`.
    pub fn from_slice_with_context(
        slice: &[[u8; 4]],
        load_context: &mut LoadContext<'_>,
    ) -> Self {
        let strip = strip_image_from_data(slice);
        let image = load_context.add_labeled_asset("palette_image".into(), strip);
        Palette { image }
    }
}

#[derive(Default)]
struct PaletteLoader;

#[allow(clippy::enum_variant_names)]
#[derive(Default, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum PaletteSettings {
    #[default]
    FromImage,
    FromIndex,
    FromRow(u32),
    FromColumn(u32),
}

#[derive(Debug, thiserror::Error)]
pub enum PaletteError {
    #[error("No color palette, not an indexed image")]
    NoIndex,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not read asset: {0}")]
    AssetBytes(#[from] bevy::asset::ReadAssetBytesError),
    #[error("Decoding error: {0}")]
    Decoding(#[from] png::DecodingError),
    #[error("Could not load image: {0}")]
    Image(#[from] bevy::image::ImageLoaderError),
}

impl AssetLoader for PaletteLoader {
    type Asset = Palette;
    type Settings = PaletteSettings;
    type Error = PaletteError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        if matches!(settings, PaletteSettings::FromIndex) {
            let mut bytes = Vec::new();
            let _ = reader.read_to_end(&mut bytes).await?;
            let data = Palette::from_png_palette(&bytes)?
                .ok_or(PaletteError::NoIndex)?;
            let strip = strip_image_from_data(&data);
            let image = load_context.add_labeled_asset("palette_image".into(), strip);
            return Ok(Palette { image });
        }

        let loader = bevy::image::ImageLoader::new(bevy::image::CompressedImageFormats::all());
        let mut image_settings = bevy::image::ImageLoaderSettings::default();
        if let Some(sampler) = crate::pico8::image::image_sampler() {
            image_settings.sampler = sampler;
        }
        let mut image_context = load_context.begin_labeled_asset();
        let image = loader
            .load(reader, &image_settings, &mut image_context)
            .await?;
        let data = match settings {
            PaletteSettings::FromIndex => unreachable!(),
            PaletteSettings::FromImage => Palette::from_image(&image),
            PaletteSettings::FromRow(row) => Palette::from_image_row(&image, *row),
            PaletteSettings::FromColumn(column) => Palette::from_image_column(&image, *column),
        };
        let strip = strip_image_from_data(&data);
        let image_handle = load_context.add_labeled_asset("palette_image".into(), strip);
        Ok(Palette {
            image: image_handle,
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
