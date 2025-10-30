pub mod const_bitdepth;
mod var_bitdepth;
pub use var_bitdepth::Gfx;
use crate::{one_or_map::OneOrMap, pico8::*};
use bevy::platform::collections::{HashMap, HashSet};
use bitvec::{prelude::*, view::BitView};
use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
};

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Gfx>()
        .register_asset_reflect::<Gfx>()
        .register_type::<GfxSprite>()
        .init_resource::<GfxImageMap>()
        .init_asset::<Gfx>()
        .init_asset::<GfxMaterial>()
        .add_systems(
            PostUpdate,
            (
                compute_image_on_asset_event,
                compute_image_on_gfx_sprite_change.after(compute_image_on_asset_event),
                check_dirty,
            ),
        );
}

type GfxImage = OneOrMap<u64, Handle<Image>>;

#[derive(Component, Reflect)]
pub struct GfxSprite {
    pub image: Handle<Gfx>,
    pub material: Handle<GfxMaterial>,
}

#[derive(Asset, Debug, Reflect, Clone, Hash, PartialEq, Eq)]
pub struct GfxMaterial {
    pub palette: usize,
    pub pal_map: PalMap,
}

// #[derive(Asset, Debug, Reflect, Clone, Hash, Eq)]
// pub struct GfxMat<'a> {
//     pub palette: usize,
//     pub pal_map: &'a PalMap,
// }

#[derive(Resource, Default, Reflect, Deref, DerefMut)]
pub struct GfxImageMap(HashMap<AssetId<Gfx>, GfxImage>);

#[derive(Component, Debug, Default)]
pub struct GfxDirty(pub bool);

fn check_dirty(
    mut events: EventReader<AssetEvent<Gfx>>,
    mut query: Query<(&mut GfxDirty, &GfxSprite)>,
) {
    let mut modified_handles: Option<HashSet<_>> = None;
    for (mut gfx_dirty, gfx_sprite) in &mut query {
        if gfx_dirty.0 {
            continue;
        }
        if modified_handles.is_none() {
            modified_handles = Some(
                events
                    .read()
                    .filter_map(|e| match e {
                        AssetEvent::Modified { id } => Some(*id),
                        _ => None,
                    })
                    .collect(),
            );
        }

        if modified_handles
            .as_ref()
            .map(|set| set.contains(&gfx_sprite.image.id()))
            .unwrap_or(false)
        {
            gfx_dirty.0 = true;
        }
    }
}

pub(crate) fn compute_image_sys(
    In(gfx_sprite): In<GfxSprite>,
    _state: Res<Pico8State>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    mut images: ResMut<Assets<Image>>,
    palettes: Res<Palettes>,
    mut pairs: ResMut<GfxImageMap>,
) -> Result<Handle<Image>, Error> {
    let _my_span = info_span!("gfx::compute_image", name = "system").entered();
    compute_image(
        &gfx_sprite.image,
        false,
        gfx_materials
            .get(&gfx_sprite.material)
            .ok_or_else(|| Error::NoSuch("gfx material".into()))?,
        &gfxs,
        &mut images,
        &palettes,
        &mut pairs,
    )
}

pub(crate) fn compute_image(
    gfx_handle: &Handle<Gfx>,
    gfx_changed: bool,
    gfx_material: &GfxMaterial,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    palettes: &Palettes,
    pairs: &mut GfxImageMap,
) -> Result<Handle<Image>, Error> {
    let _my_span = info_span!("gfx::compute_image", name = "function").entered();

    if gfx_material.palette >= palettes.len() {
        return Err(Error::NoSuch("palette".into()));
    }
    let mut hasher = DefaultHasher::new();
    gfx_material.pal_map.hash(&mut hasher);
    gfx_material.palette.hash(&mut hasher);
    // let drawing = &state.draw_state;
    // drawing.fill_pat.inspect(|fill_pat| {
    //     fill_pat.hash(&mut hasher);
    // });
    let hash = hasher.finish();
    let gfx_id = gfx_handle.id();
    let palette = palettes.get_pal(gfx_material.palette)?;
    let image_handle: Option<Handle<Image>> = pairs.get(&gfx_id).and_then(|gfx_image| {
        gfx_image
            .get(&hash)
            .inspect(|handle| {
                if gfx_changed {
                    let _my_span = info_span!("gfx::compute_image", name = "update image").entered();
                    let gfx = gfxs.get(gfx_id);
                    // Update existing image.
                    if let Some((gfx, image)) = gfx.zip(images.get_mut(*handle)) {
                        trace!("updating image for gfx {}", gfx_id);
                        if let Some(ref mut data) = image.data {
                            gfx.write_bytes(data, |i, _, bytes| {
                                gfx_material.pal_map.write_color(&palette.data, i, bytes);
                            });
                        } else {
                            warn_once!("No data for image {}", gfx_id);
                        }
                    }
                }
            })
            .cloned()
    });
    let image_handle: Result<Handle<Image>, Error> = image_handle.map(Ok).unwrap_or_else(|| {
        let _my_span = info_span!("gfx::compute_image", name = "create image").entered();
        let gfx = gfxs
            .get(gfx_handle)
            .ok_or(Error::NoSuch("gfx image".into()))?;
        trace!("creating image for gfx {}", gfx_id);
        let image = images.add(gfx.try_to_image(|i, n, bytes| {
            trace!("pixel {} writing color {}", n, i);
            gfx_material.pal_map.write_color(&palette.data, i, bytes)
        })?);
        // Update or add image to the map.
        pairs
            .entry(gfx_id)
            .and_modify(|gfx_image| {
                gfx_image.insert(hash, image.clone());
            })
            .or_insert_with(|| GfxImage::new(hash, image.clone()));
        Ok(image)
    });
    image_handle
}

// Informed from Bevy's Sprite::compute_slices_on_asset_event.
fn compute_image_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Gfx>>,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    _state: Res<Pico8State>,
    palettes: Res<Palettes>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>)>,
    mut pairs: ResMut<GfxImageMap>,
    mut update_ids: Local<Vec<Entity>>,
    mut update_images: Local<VecDeque<Handle<Image>>>,
    // mut update_images: Local<Vec<(Entity, Handle<Image>)>>,
) {
    // We store the asset ids of added/modified image assets.
    let added_handles: HashSet<_> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => Some(*id),
            AssetEvent::Removed { id } => {
                pairs.remove(id);
                None
            }
            _ => None,
        })
        .collect();
    if added_handles.is_empty() {
        return;
    }
    for (id, gfx_sprite, sprite) in &sprites {
        if !added_handles.contains(&gfx_sprite.image.id()) {
            continue;
        }

        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            continue;
        };
        let image_handle = compute_image(
            &gfx_sprite.image,
            true,
            gfx_material,
            &gfxs,
            &mut images,
            &palettes,
            &mut pairs,
        );
        match image_handle {
            Ok(image) => {
                match sprite {
                    Some(sprite) => {
                        if sprite.image != image {
                            // trace!("updating existant sprite on {}", id);
                            // sprite.image = image;
                            update_ids.push(id);
                            update_images.push_back(image);
                        }
                    }
                    None => {
                        // trace!("inserting new sprite into {}", id);
                        commands.entity(id).insert(Sprite::from_image(image));
                    }
                }
            }
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
    // Try not to trigger a sprite change if we don't have to.
    let mut iter = sprites.iter_many_mut(update_ids.iter());
    while let Some((_, _, sprite)) = iter.fetch_next() {
        match sprite {
            Some(mut sprite) => {
                sprite.image = update_images.pop_front().unwrap();
            }
            _ => unreachable!(),
        }
    }
}

fn compute_image_on_gfx_sprite_change(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    _state: Res<Pico8State>,
    palettes: Res<Palettes>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>), Changed<GfxSprite>>,
    mut pairs: ResMut<GfxImageMap>,
) {
    for (id, gfx_sprite, sprite) in &mut sprites {
        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {

            trace!("No gfx material for gfx sprite {}", id);
            continue;
        };
        let image_handle = compute_image(
            &gfx_sprite.image,
            false,
            gfx_material,
            &gfxs,
            &mut images,
            &palettes,
            &mut pairs,
        );
        match image_handle {
            Ok(image) => match sprite {
                Some(mut sprite) => {
                    trace!("updating existant sprite on {}", id);
                    sprite.image = image;
                }
                None => {
                    trace!("inserting new sprite into {}", id);
                    commands.entity(id).insert(Sprite::from_image(image));
                }
            },
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
}

impl<T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy> const_bitdepth::Gfx<1, T> {
    pub fn mirror_horizontal(mut self) -> Self {
        for elem in self.data.chunks_mut(self.width) {
            elem.reverse();
        }
        self
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PngError {
    #[error("Not an indexed png")]
    NotIndexed,
    #[error("Unexpected bit-depth of {expected} but was {actual}")]
    BitDepth { expected: u8, actual: u8 },
    #[error("Cannot convert bit-depth for pixel {pixel_index} with value {pixel_value}")]
    BitDepthConversion { pixel_index: usize, pixel_value: u8 },
}

