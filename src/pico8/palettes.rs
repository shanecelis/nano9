use bevy::{
    prelude::*,
    // platform::hash::DefaultHasher,
};

use crate::{PColor, pico8::{Error, FillPat, Gfx, PalMap, Palette}};

use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{Hash, Hasher, DefaultHasher},
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<Palettes>();
}

#[derive(Debug, Resource, Default)]
pub struct Palettes {
    pub(crate) palettes: Vec<Palette>,
}

impl Palettes {

    // Copied from Pico8Asset.
    pub(crate) fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, Error> {
        match c {
            PColor::Palette(n) => self.palettes[palette_index].get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
    }

}
