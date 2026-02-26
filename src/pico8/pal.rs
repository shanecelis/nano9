use bevy::asset::RenderAssetUsages;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Asset, Debug, Clone, Reflect)]
pub struct Palette {
    /// Canonical palette as a 1×N or N×1 RGBA image.
    pub image: Handle<Image>,
    pub access: PaletteAccess,
}

#[derive(Debug, Default, Clone, Copy, Reflect)]
/// How palette indices map to pixels in the palette image.
///
/// - **LinearByRow**: color 0 → (0,0), 1 → (1,0), … width → (0,1). Length = width × height.
/// - **LinearByColumn**: color 0 → (0,0), 1 → (0,1), … height → (1,0). Length = width × height.
/// - **FromRow(row)**: only that row; color i → (i, row). Length = width.
/// - **FromColumn(col)**: only that column; color i → (col, i). Length = height.
pub enum PaletteAccess {
    #[default]
    /// Access the palette linearly by row.
    LinearByRow,
    /// Access the palette linearly by column.
    LinearByColumn,
    /// Only use the specified row. Palette length is image width.
    FromRow(u32),
    /// Only use the specified column. Palette length is image height.
    FromColumn(u32),
}

/// Number of palette entries for this access mode and image size.
pub fn palette_len(access: &PaletteAccess, width: u32, height: u32) -> usize {
    match access {
        PaletteAccess::LinearByRow | PaletteAccess::LinearByColumn => {
            (width as usize).saturating_mul(height as usize)
        }
        PaletteAccess::FromRow(_) => width as usize,
        PaletteAccess::FromColumn(_) => height as usize,
    }
}

/// Map palette index to (x, y) in the image. Returns `None` if index is out of range.
pub fn palette_index_to_xy(
    access: &PaletteAccess,
    width: u32,
    height: u32,
    index: usize,
) -> Option<(u32, u32)> {
    let w = width as usize;
    let h = height as usize;
    match access {
        PaletteAccess::LinearByRow => {
            if index >= w * h {
                return None;
            }
            Some(((index % w) as u32, (index / w) as u32))
        }
        PaletteAccess::LinearByColumn => {
            if index >= w * h {
                return None;
            }
            Some(((index / h) as u32, (index % h) as u32))
        }
        PaletteAccess::FromRow(row) => {
            if index >= w || *row >= height {
                return None;
            }
            Some((index as u32, *row))
        }
        PaletteAccess::FromColumn(col) => {
            if index >= h || *col >= width {
                return None;
            }
            Some((*col, index as u32))
        }
    }
}

/// GPU shader access mode: 0 = LinearByRow, 1 = LinearByColumn, 2 = FromRow, 3 = FromColumn.
pub const PALETTE_ACCESS_LINEAR_BY_ROW: u32 = 0;
pub const PALETTE_ACCESS_LINEAR_BY_COLUMN: u32 = 1;
pub const PALETTE_ACCESS_FROM_ROW: u32 = 2;
pub const PALETTE_ACCESS_FROM_COLUMN: u32 = 3;

/// Encode `PaletteAccess` for GPU uniforms: (access_kind, access_param).
/// `access_param` is the row for FromRow, column for FromColumn; 0 otherwise.
pub fn palette_access_to_gpu(access: &PaletteAccess) -> (u32, u32) {
    match access {
        PaletteAccess::LinearByRow => (PALETTE_ACCESS_LINEAR_BY_ROW, 0),
        PaletteAccess::LinearByColumn => (PALETTE_ACCESS_LINEAR_BY_COLUMN, 0),
        PaletteAccess::FromRow(r) => (PALETTE_ACCESS_FROM_ROW, *r),
        PaletteAccess::FromColumn(c) => (PALETTE_ACCESS_FROM_COLUMN, *c),
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

/// Build a 1×N RGBA strip image from raw palette data.
/// Only used when constructing a palette from data that is not already an image:
/// e.g. FromIndex (extracting from an indexed PNG’s color table) or from_slice / default palette.
/// For FromImage, FromRow, FromColumn we use the source image directly and do not create a strip.
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

/// Read palette colors from the image in palette index order, according to `access`.
pub fn palette_data_from_image(image: &Image, access: &PaletteAccess) -> Vec<[u8; 4]> {
    let size = image.size();
    let n = palette_len(access, size.x, size.y);
    let mut data = Vec::with_capacity(n);
    for index in 0..n {
        if let Some((x, y)) = palette_index_to_xy(access, size.x, size.y, index) {
            if let Ok(color) = image.get_color_at(x, y) {
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

    /// Number of colors when read from the given image according to `self.access`.
    pub fn len_in(&self, image: &Image) -> usize {
        let size = image.size();
        palette_len(&self.access, size.x, size.y)
    }

    /// Get color at palette index using the loaded palette image and `self.access`.
    pub fn get_color_in(&self, index: usize, image: &Image) -> Result<Srgba, PalError> {
        let size = image.size();
        let (x, y) = palette_index_to_xy(&self.access, size.x, size.y, index)
            .ok_or(PalError::NoSuchColor(index))?;
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
    pub fn from_slice_with_context(slice: &[[u8; 4]], load_context: &mut LoadContext<'_>) -> Self {
        let strip = strip_image_from_data(slice);
        let image = load_context.add_labeled_asset("palette_image".into(), strip);
        Palette {
            image,
            access: PaletteAccess::default(),
        }
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
            let data = Palette::from_png_palette(&bytes)?.ok_or(PaletteError::NoIndex)?;
            let strip = strip_image_from_data(&data);
            trace!(
                "Loading palette from image for path {:?}",
                load_context.path()
            );
            let image = load_context.add_labeled_asset("palette_image".into(), strip);
            return Ok(Palette {
                image,
                access: PaletteAccess::default(),
            });
        }

        // Use the image as-is; access (row/column/linear) is stored on the Palette.
        let loader = bevy::image::ImageLoader::new(bevy::image::CompressedImageFormats::all());
        let mut image_settings = bevy::image::ImageLoaderSettings::default();
        if let Some(sampler) = crate::pico8::image::image_sampler() {
            image_settings.sampler = sampler;
        }
        let mut image_context = load_context.begin_labeled_asset();
        let image = loader
            .load(reader, &image_settings, &mut image_context)
            .await?;
        let access = match settings {
            PaletteSettings::FromIndex => unreachable!(),
            PaletteSettings::FromImage => PaletteAccess::LinearByRow,
            PaletteSettings::FromRow(row) => PaletteAccess::FromRow(*row),
            PaletteSettings::FromColumn(column) => PaletteAccess::FromColumn(*column),
        };
        let image_handle = load_context.add_labeled_asset("palette_image".into(), image);
        Ok(Palette {
            image: image_handle,
            access,
        })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["png"];
        EXTENSIONS
    }
}
