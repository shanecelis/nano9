use bevy::{
    prelude::*,
    // platform::hash::DefaultHasher,
};

use crate::{
    pico8::{PalError, Palette},
    PColor,
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Palettes>();
}

#[derive(Debug, Resource, Default, Deref, DerefMut)]
pub struct Palettes(pub(crate) Vec<Palette>);

impl Palettes {
    pub fn get_pal(&self, palette_index: usize) -> Result<&Palette, PalError> {
        self.0.get(palette_index).ok_or(PalError::NoSuchPalette {
            index: palette_index,
            count: self.0.len(),
        })
    }

    // Resolve a PColor into a Color.
    pub fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, PalError> {
        match c {
            PColor::Palette(n) => self
                .get_pal(palette_index)?
                .get_color(n)
                .map(|c| c.into())
                .map_err(|e| match e {
                    PalError::NoSuchColor(c) => PalError::NoSuchPaletteColor {
                        color: c,
                        palette: palette_index,
                    },
                    x => x,
                }),
            PColor::Color(c) => Ok(c.into()),
        }
    }
}
