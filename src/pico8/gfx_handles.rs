use bevy::prelude::*;

use crate::{PColor, pico8::{Error, FillPat, Gfx, PalMap, Palette}};

use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<GfxHandles>().add_systems(
        PostUpdate,
        |mut gfx_handles: ResMut<GfxHandles>| {
            gfx_handles.tick();
        },
    );
}

/// A double-buffered map of (Gfx, PalMap) -> Handle<Image>
///
/// It hands out strong handles and internally persists a strong handle for
/// a tick or frame. This permits the standard drawing scheme of `cls();
/// spr(1)` to not cause asset churn.
///
/// In cases where one is not drawing from the same sprite sheet each frame, one
/// can retain the sprite to avoid churn.
/// ```lua
/// function _init()
///     local s = spr(0):retain()
///     -- Can set visibility to false so it is retained but not drawn.
///     s:vis(false)
///
/// end
/// ```
#[derive(Debug, Resource, Default)]
pub struct GfxHandles {
    buffers: [HashMap<u64, Handle<Image>>; 2],
    pub(crate) palettes: Vec<Palette>,
    tick: usize,
}

impl GfxHandles {
    /// This returns a strong handle if it was created and caches a weak handle.
    /// Otherwise it returns an extant weak_handle.
    pub fn get_or_create(
        &mut self,
        palette: usize,
        pal_map: &PalMap,
        fill_pat: Option<&FillPat>,
        gfx: &Handle<Gfx>,
        gfxs: &Assets<Gfx>,
        images: &mut Assets<Image>,
    ) -> Result<Handle<Image>, Error> {
        let mut hasher = DefaultHasher::new();
        pal_map.hash(&mut hasher);
        if let Some(fill_pat) = fill_pat {
            fill_pat.hash(&mut hasher);
        }
        gfx.hash(&mut hasher);
        let hash = hasher.finish();
        let other_handle: Option<Handle<Image>> =
            self.buffers[(self.tick + 1) % 2].get(&hash).cloned();
        let map = &mut self.buffers[self.tick % 2];
        let handle: Handle<Image> = match map.entry(hash) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => {
                if let Some(handle) = other_handle {
                    entry.insert(handle).clone()
                } else {
                    let gfx = gfxs.get(gfx).ok_or(Error::NoSuch("gfx asset".into()))?;
                    let image = if let Some(_fill_pat) = fill_pat {
                        todo!();
                    } else {
                        gfx.try_to_image(|i, _, bytes| {
                            pal_map.write_color(&self.palettes[palette].data, i, bytes)
                        })?
                    };
                    entry.insert(images.add(image)).clone()
                }
            }
        };
        Ok(handle)
    }

    // Copied from Pico8Asset.
    pub(crate) fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, Error> {
        match c {
            PColor::Palette(n) => self.palettes[palette_index].get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        self.buffers[self.tick % 2].clear();
    }
}
