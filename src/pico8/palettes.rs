use bevy::prelude::*;

use crate::{
    PColor,
    pico8::{PalError, Palette},
};

pub(crate) fn plugin(_app: &mut App) {}

/// Default Pico-8 palette used when the palette image is not yet loaded (e.g. config-loaded asset).
fn default_pico8_color(index: usize) -> Option<Color> {
    let [r, g, b, a] = *super::cart::PALETTE.get(index)?;
    Some(Color::srgba_u8(r, g, b, a))
}

#[derive(Debug, Default, Deref, DerefMut, Reflect, Clone)]
pub struct Palettes(pub(crate) Vec<Palette>);

impl Palettes {
    pub fn get_pal(&self, palette_index: usize) -> Result<&Palette, PalError> {
        self.0.get(palette_index).ok_or(PalError::NoSuchPalette {
            index: palette_index,
            count: self.0.len(),
        })
    }

    /// Number of colors in the palette at the given index, when loaded from `images`. Returns `None` if the palette image is not loaded.
    pub fn len_in(
        &self,
        palette_index: usize,
        images: &Assets<Image>,
    ) -> Result<Option<usize>, PalError> {
        let pal = self.get_pal(palette_index)?;
        Ok(images.get(&pal.image).map(|img| pal.len_in(img)))
    }

    /// Resolve a PColor into a Color using the palette image from `images`.
    /// When the palette image is not yet loaded (e.g. still loading from config), falls back to the default Pico-8 palette for indices 0..16.
    pub fn get_color(
        &self,
        c: PColor,
        palette_index: usize,
        images: &Assets<Image>,
    ) -> Result<Color, PalError> {
        match c {
            PColor::Palette(n) => {
                let pal = self.get_pal(palette_index)?;
                if let Some(image) = images.get(&pal.image) {
                    pal.get_color_in(n, image)
                        .map(|c| c.into())
                        .map_err(|e| match e {
                            PalError::NoSuchColor(c) => PalError::NoSuchPaletteColor {
                                color: c,
                                palette: palette_index,
                            },
                            x => x,
                        })
                } else if let Some(color) = default_pico8_color(n) {
                    Ok(color)
                } else {
                    Err(PalError::NoSuchPaletteColor {
                        color: n,
                        palette: palette_index,
                    })
                }
            }
            PColor::Color(c) => Ok(c.into()),
        }
    }
}

impl From<Vec<Palette>> for Palettes {
    fn from(palettes: Vec<Palette>) -> Self {
        Self(palettes)
    }
}
