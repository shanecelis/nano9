use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};

#[derive(Asset, Debug, Clone, Reflect, Default)]
pub struct Palette {
    pub data: Vec<[u8; 4]>,
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
}

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<Palette>()
        .init_asset_loader::<PaletteLoader>();
}

impl Palette {
    pub fn from_png_palette(bytes: &[u8]) -> Result<Option<Self>, png::DecodingError> {
        let cursor = std::io::Cursor::new(bytes);
        let decoder = png::Decoder::new(cursor);
        let reader = decoder.read_info()?;
        let info = reader.info();
        Ok(Self::from_png_palette_info(info))
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
        let mut data = vec![[0; 4]; size.y as usize];
        for j in 0..size.y {
            let color: Srgba = image.get_color_at(column, j).unwrap().into();
            data[j as usize] = color.to_u8_array();
        }
        Palette { data }
    }

    pub fn from_image_row(image: &Image, row: u32) -> Self {
        let size = image.size();
        let mut data = vec![[0; 4]; size.x as usize];
        for i in 0..size.x {
            let color: Srgba = image.get_color_at(i, row).unwrap().into();
            data[i as usize] = color.to_u8_array();
        }
        Palette { data }
    }

    pub fn from_image(image: &Image) -> Self {
        let size = image.size();
        let mut data = vec![[0; 4]; (size.x * size.y) as usize];
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
            // Load from png.
            let mut bytes = Vec::new();
            let _ = reader.read_to_end(&mut bytes).await?;
            return if let Some(palette) = Palette::from_png_palette(&bytes)? {
                Ok(palette)
            } else {
                Err(PaletteError::NoIndex)
            };
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
        Ok(match settings {
            PaletteSettings::FromIndex => unreachable!(),
            PaletteSettings::FromImage => Palette::from_image(&image),
            PaletteSettings::FromRow(row) => Palette::from_image_row(&image, *row),
            PaletteSettings::FromColumn(column) => Palette::from_image_column(&image, *column),
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
