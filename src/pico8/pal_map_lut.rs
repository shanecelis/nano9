//! CPU palette map plus the `Rg8Uint` image the sprite shader samples.
//!
//! The [`PalMap`] is authoritative. The image is a derived cache:
//! `r = display[draw[i]]`, `g` is the draw map's transparency for index `i`.

use crate::pico8::PalMap;
use bevy::asset::{AssetId, Assets, RenderAssetUsages};
use bevy::prelude::{Asset, Handle, Image, Reflect};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const PAL_MAP_SIZE: usize = 256;

/// Draw-palette mapping and its lazily reified lookup texture.
#[derive(Asset, Debug, Clone, Reflect)]
pub struct PalMapLut {
    pub pal_map: PalMap,
    #[reflect(ignore)]
    pub image: Option<Handle<Image>>,
    dirty: bool,
    pub frozen: bool,
}

impl PalMapLut {
    pub fn new(pal_map: PalMap) -> Self {
        Self {
            pal_map,
            image: None,
            dirty: true,
            frozen: false,
        }
    }

    pub(crate) fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Image handle when it already matches the draw map. Callers can clone this without writing.
    pub fn prepared_image(&self) -> Option<Handle<Image>> {
        if self.dirty { None } else { self.image.clone() }
    }

    pub(crate) fn pal_map_mut(&mut self) -> &mut PalMap {
        self.dirty = true;
        &mut self.pal_map
    }

    pub fn ensure_capacity(&mut self, min_len: usize) {
        self.pal_map.ensure_capacity(min_len);
        self.dirty = true;
    }

    /// Create or refresh the image from `display`. A clean existing image is reused.
    pub fn ensure_image(&mut self, display: &PalMap, images: &mut Assets<Image>) -> ImageWrite {
        if let Some(handle) = self.image.clone()
            && !self.dirty
        {
            return ImageWrite {
                handle,
                added: false,
                modified: false,
            };
        }
        if let Some(handle) = self.image.clone() {
            if write_composed_lut(images, &handle, &self.pal_map, display) {
                self.dirty = false;
                return ImageWrite {
                    handle,
                    added: false,
                    modified: true,
                };
            }
        }
        let handle = images.add(composed_lut_image(&self.pal_map, display));
        self.image = Some(handle.clone());
        self.dirty = false;
        ImageWrite {
            handle,
            added: true,
            modified: false,
        }
    }

    /// Rewrite an existing image. Frozen assets and assets with no image are skipped.
    pub fn reify_existing(
        &mut self,
        display: &PalMap,
        images: &mut Assets<Image>,
    ) -> Option<AssetId<Image>> {
        if self.frozen {
            return None;
        }
        let handle = self.image.clone()?;
        if write_composed_lut(images, &handle, &self.pal_map, display) {
            self.dirty = false;
            Some(handle.id())
        } else {
            self.image = None;
            self.dirty = true;
            None
        }
    }

    /// Private copy whose image mode 1 will not rewrite.
    pub fn freeze_clone(&self, display: &PalMap, images: &mut Assets<Image>) -> Self {
        let mut frozen = Self {
            pal_map: self.pal_map.clone(),
            image: None,
            dirty: false,
            frozen: true,
        };
        let bytes = self
            .image
            .as_ref()
            .and_then(|handle| images.get(handle).cloned())
            .unwrap_or_else(|| composed_lut_image(&self.pal_map, display));
        frozen.image = Some(images.add(bytes));
        frozen
    }
}

pub struct ImageWrite {
    pub handle: Handle<Image>,
    pub added: bool,
    pub modified: bool,
}

/// `r = display[draw[i]]`, `g` from the draw map. Display slots past the map stay put.
pub fn write_lut_image(image: &mut Image, draw: &PalMap, display: &PalMap) {
    let Some(bytes) = image.data.as_deref_mut() else {
        return;
    };
    for i in 0..PAL_MAP_SIZE {
        let mapped = draw.map_or_mod(i);
        bytes[i * 2] = display.map_or_identity(mapped) as u8;
        bytes[i * 2 + 1] = u8::from(draw.transparency.get(i).is_some_and(|bit| *bit));
    }
}

pub fn write_composed_lut(
    images: &mut Assets<Image>,
    handle: &Handle<Image>,
    draw: &PalMap,
    display: &PalMap,
) -> bool {
    let Some(mut image) = images.get_mut_untracked(handle) else {
        return false;
    };
    write_lut_image(&mut image, draw, display);
    true
}

/// Draw map composed with the display map, packed as one `Rg8Uint` row.
pub fn composed_lut_image(draw: &PalMap, display: &PalMap) -> Image {
    let mut image = pal_map_to_lut_image(draw);
    // `pal_map_to_lut_image` is the identity-display packing. Overlay the real display map.
    write_lut_image(&mut image, draw, display);
    image
}

/// One `Rg8Uint` row: `r` is the remapped index, `g` is 1 when `palt` hides that index.
pub fn pal_map_to_lut_image(pal_map: &PalMap) -> Image {
    let mut bytes = vec![0u8; PAL_MAP_SIZE * 2];
    for i in 0..PAL_MAP_SIZE {
        bytes[i * 2] = pal_map.map_or_mod(i) as u8;
        bytes[i * 2 + 1] = u8::from(pal_map.transparency.get(i).is_some_and(|bit| *bit));
    }
    Image::new(
        Extent3d {
            width: PAL_MAP_SIZE as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rg8Uint,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn channel(images: &Assets<Image>, handle: &Handle<Image>, index: usize) -> (u8, u8) {
        let data = images.get(handle).unwrap().data.as_ref().unwrap();
        (data[index * 2], data[index * 2 + 1])
    }

    #[test]
    fn attached_draws_share_one_asset_until_the_draw_map_changes() {
        let mut images = Assets::<Image>::default();
        let mut luts = Assets::<PalMapLut>::default();
        let display = PalMap::default();
        let handle = luts.add(PalMapLut::new(PalMap::default()));
        let first = luts
            .get_mut(&handle)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;
        let second = luts
            .get_mut(&handle)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;
        assert_eq!(first, second);
        assert!(
            !luts
                .get_mut(&handle)
                .unwrap()
                .ensure_image(&display, &mut images)
                .modified
        );
    }

    #[test]
    fn unattached_edits_reuse_the_asset_and_attached_edits_copy_it() {
        let mut images = Assets::<Image>::default();
        let mut luts = Assets::<PalMapLut>::default();
        let display = PalMap::default();
        let mut pal_map = PalMap::default();
        pal_map.remap(8, 12);
        let current = luts.add(PalMapLut::new(pal_map));
        luts.get_mut(&current).unwrap().pal_map_mut().remap(8, 9);
        assert!(luts.get(&current).unwrap().image.is_none());
        let image = luts
            .get_mut(&current)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;
        assert_eq!(channel(&images, &image, 8).0, 9);

        let copied = luts.get(&current).unwrap().pal_map.clone();
        let mut next = PalMapLut::new(copied);
        next.pal_map_mut().remap(8, 11);
        let forked = luts.add(next);
        assert!(luts.get(&forked).unwrap().image.is_none());
        let forked_image = luts
            .get_mut(&forked)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;
        assert_ne!(image, forked_image);
        assert_eq!(channel(&images, &image, 8).0, 9);
        assert_eq!(channel(&images, &forked_image, 8).0, 11);
    }

    #[test]
    fn display_reify_uses_each_draw_map_and_skips_frozen_copies() {
        let mut images = Assets::<Image>::default();
        let mut luts = Assets::<PalMapLut>::default();
        let mut display = PalMap::default();

        let mut first_map = PalMap::default();
        first_map.remap(1, 2);
        let first = luts.add(PalMapLut::new(first_map));
        let first_image = luts
            .get_mut(&first)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;

        let mut second_map = PalMap::default();
        second_map.remap(1, 3);
        let second = luts.add(PalMapLut::new(second_map));
        let second_image = luts
            .get_mut(&second)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;

        let frozen = luts.add(
            luts.get(&second)
                .unwrap()
                .freeze_clone(&display, &mut images),
        );
        let frozen_image = luts.get(&frozen).unwrap().image.clone().unwrap();
        let frozen_before = channel(&images, &frozen_image, 1);

        display.remap(2, 4);
        display.remap(4, 7);
        let mut modified = Vec::new();
        for (_, lut) in luts.iter_mut() {
            if let Some(id) = lut.reify_existing(&display, &mut images) {
                modified.push(id);
            }
        }
        assert_eq!(
            modified.iter().copied().collect::<HashSet<_>>().len(),
            modified.len()
        );
        assert_eq!(modified.len(), 2);
        assert_eq!(channel(&images, &first_image, 1), (4, 0));
        assert_eq!(channel(&images, &second_image, 1), (3, 0));
        assert_eq!(channel(&images, &frozen_image, 1), frozen_before);

        display.remap(2, 9);
        for (_, lut) in luts.iter_mut() {
            let _ = lut.reify_existing(&display, &mut images);
        }
        assert_eq!(channel(&images, &first_image, 1).0, 9);
        assert_eq!(channel(&images, &second_image, 1).0, 3);
        assert_eq!(channel(&images, &frozen_image, 1), frozen_before);
    }

    #[test]
    fn display_reify_keeps_draw_transparency() {
        let mut images = Assets::<Image>::default();
        let mut luts = Assets::<PalMapLut>::default();
        let mut display = PalMap::default();
        let mut draw = PalMap::default();
        draw.remap(8, 9);
        draw.transparency.set(8, true);
        let handle = luts.add(PalMapLut::new(draw));
        let image = luts
            .get_mut(&handle)
            .unwrap()
            .ensure_image(&display, &mut images)
            .handle;
        display.remap(9, 12);
        luts.get_mut(&handle)
            .unwrap()
            .reify_existing(&display, &mut images);
        assert_eq!(channel(&images, &image, 8), (12, 1));
    }

    #[test]
    fn image_handle_lives_on_the_pal_map_lut() {
        let mut images = Assets::<Image>::default();
        let mut luts = Assets::<PalMapLut>::default();
        let handle = luts.add(PalMapLut::new(PalMap::default()));
        let image = luts
            .get_mut(&handle)
            .unwrap()
            .ensure_image(&PalMap::default(), &mut images)
            .handle;
        let stored = luts.get(&handle).unwrap().image.clone().unwrap();
        drop(image);
        // `stored` and the handle kept on the asset.
        assert_eq!(strong_count(&stored), 2);
        luts.remove(&handle);
        assert_eq!(strong_count(&stored), 1);
    }

    fn strong_count(handle: &Handle<Image>) -> usize {
        match handle {
            Handle::Strong(strong) => std::sync::Arc::strong_count(strong),
            Handle::Uuid(_, _) => panic!("lut image is a strong handle"),
        }
    }
}
