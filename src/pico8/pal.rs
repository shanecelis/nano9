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
    /// CPU cache for get_color, write_color, len; populated at load or when image is created from data.
    pub cached_data: Option<Vec<[u8; 4]>>,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            image: Handle::default(),
            cached_data: None,
        }
    }
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
fn strip_image_from_data(data: &[[u8; 4]]) -> Image {
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

impl Palette {
    /// Reference to the palette image handle.
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    /// CPU cache of palette colors; `None` if not yet populated.
    pub fn data(&self) -> Option<&[[u8; 4]]> {
        self.cached_data.as_deref()
    }

    /// Number of colors (from cache); 0 if cache is not set.
    pub fn len(&self) -> usize {
        self.cached_data.as_ref().map_or(0, Vec::len)
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

    /// Palette from slice; image handle is default (no asset). Use when no LoadContext is available.
    pub fn from_slice(slice: &[[u8; 4]]) -> Self {
        Palette {
            image: Handle::default(),
            cached_data: Some(Vec::from(slice)),
        }
    }

    /// Palette from slice, creating the strip image and adding it via `load_context`.
    pub fn from_slice_with_context(
        slice: &[[u8; 4]],
        load_context: &mut LoadContext<'_>,
    ) -> Self {
        let data = Vec::from(slice);
        let strip = strip_image_from_data(&data);
        let image = load_context.add_labeled_asset("palette_image".into(), strip);
        Palette {
            image,
            cached_data: Some(data),
        }
    }

    pub fn write_color(&self, index: usize, pixel_bytes: &mut [u8]) -> Result<(), PalError> {
        let data = self
            .cached_data
            .as_ref()
            .and_then(|d| d.get(index))
            .ok_or(PalError::NoSuchColor(index))?;
        pixel_bytes.copy_from_slice(&data[0..pixel_bytes.len()]);
        Ok(())
    }

    pub fn get_color(&self, index: usize) -> Result<Srgba, PalError> {
        self.cached_data
            .as_ref()
            .and_then(|d| d.get(index))
            .ok_or(PalError::NoSuchColor(index))
            .map(|a| Srgba::rgba_u8(a[0], a[1], a[2], a[3]))
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
            return Ok(Palette {
                image,
                cached_data: Some(data),
            });
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
            cached_data: Some(data),
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
