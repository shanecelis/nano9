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
    pub fn from_image(image: &Image, row: Option<u32>) -> Self {
        let size = image.size();
        let mut data = Vec::new();
        if let Some(row) = row {
            for i in 0..size.x {
                let color: Srgba = image.get_color_at(i, row).unwrap().into();
                data.push(color.to_u8_array());
            }
        } else {
            for j in 0..size.y {
                for i in 0..size.x {
                    let color: Srgba = image.get_color_at(i, j).unwrap().into();
                    data.push(color.to_u8_array());
                }
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
        let data = self
            .data
            .get(index)
            .ok_or(PalError::NoSuchColor(index))?;
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
