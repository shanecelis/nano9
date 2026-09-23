pub mod const_bitdepth;
pub mod pal_map_textures;
pub mod palette_lookup_material;
mod var_bitdepth;
use crate::{one_or_map::OneOrMap, pico8::*};
use bevy::asset::{AssetEventSystems, AssetId};
use bevy::platform::collections::{HashMap, HashSet};
use bevy::sprite_render::SpriteMaterial;
use palette_lookup_material::{GfxPaletteLookupMaterial, build_palette_lookup_material};
use std::{
    collections::VecDeque,
    hash::{DefaultHasher, Hash, Hasher},
};
pub use var_bitdepth::Gfx;

pub(crate) fn plugin(app: &mut App) {
    palette_lookup_material::gfx_palette_lookup_material_plugin(app);
    app
        //register_type::<Gfx>()
        .register_asset_reflect::<Gfx>()
        // .register_type::<GfxSprite>()
        // .register_type::<GfxMaterial>()
        .register_asset_reflect::<GfxMaterial>()
        .init_resource::<GfxImageMap>()
        .init_resource::<IndexImageMap>()
        .init_resource::<GfxGpuMaterialMap>()
        .init_asset::<Gfx>()
        .init_asset::<GfxMaterial>()
        .add_systems(
            PostUpdate,
            (
                compute_image_on_asset_event.after(AssetEventSystems),
                compute_image_on_gfx_sprite_change.after(compute_image_on_asset_event),
                check_dirty.after(AssetEventSystems),
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

#[derive(Resource, Default, Reflect, Deref, DerefMut)]
pub struct GfxImageMap(HashMap<AssetId<Gfx>, GfxImage>);

#[derive(Resource, Default, Deref, DerefMut)]
struct IndexImageMap(HashMap<AssetId<Gfx>, Handle<Image>>);

#[derive(Resource, Default)]
struct GfxGpuMaterialMap(HashMap<AssetId<GfxMaterial>, (u64, Handle<GfxPaletteLookupMaterial>)>);

fn material_hash(gfx_material: &GfxMaterial) -> u64 {
    let mut hasher = DefaultHasher::new();
    gfx_material.pal_map.hash(&mut hasher);
    gfx_material.palette.hash(&mut hasher);
    hasher.finish()
}

/// Index-image sprite plus a [`SpriteMaterial`] that does palette lookup on the GPU.
/// Canvases with per-pixel occupancy stay on the CPU path.
fn gpu_sprite_assets(
    gfx_handle: &Handle<Gfx>,
    gfx_changed: bool,
    gfx_material_handle: &Handle<GfxMaterial>,
    gfx_material: &GfxMaterial,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    palette_materials: &mut Assets<GfxPaletteLookupMaterial>,
    palettes: &Palettes,
    index_images: &mut IndexImageMap,
    gpu_materials: &mut GfxGpuMaterialMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<(Handle<Image>, Handle<GfxPaletteLookupMaterial>), Error> {
    let gfx_id = gfx_handle.id();
    let gfx = gfxs.get(gfx_id).ok_or(Error::NoSuch("gfx image".into()))?;
    let index = if let Some(handle) = index_images.get(&gfx_id).cloned() {
        if gfx_changed && let Some(image) = images.get_mut_untracked(&handle) {
            *image = gfx.to_index_image();
            image_events.write(AssetEvent::Modified { id: handle.id() });
        }
        handle
    } else {
        let handle = images.add(gfx.to_index_image());
        image_events.write(AssetEvent::Added { id: handle.id() });
        index_images.insert(gfx_id, handle.clone());
        handle
    };

    let hash = material_hash(gfx_material);
    let material_id = gfx_material_handle.id();
    if let Some((cached_hash, handle)) = gpu_materials.0.get(&material_id).cloned()
        && cached_hash == hash
    {
        return Ok((index, handle));
    }

    let palette = palettes.get_pal(gfx_material.palette)?;
    let (remap, transparency) = pal_map_textures::pal_map_to_images(&gfx_material.pal_map);
    let remap = images.add(remap);
    let transparency = images.add(transparency);
    let palette_image_ref = images.get(&palette.image);
    let material = build_palette_lookup_material(
        palette.image().clone(),
        remap,
        transparency,
        palette,
        palette_image_ref,
    );
    let handle = if let Some((_, handle)) = gpu_materials.0.get(&material_id).cloned() {
        if let Some(mut existing) = palette_materials.get_mut(&handle) {
            *existing = material;
        }
        gpu_materials.0.insert(material_id, (hash, handle.clone()));
        handle
    } else {
        let handle = palette_materials.add(material);
        gpu_materials.0.insert(material_id, (hash, handle.clone()));
        handle
    };
    Ok((index, handle))
}

#[derive(Component, Debug, Default)]
pub struct GfxDirty(pub bool);

fn check_dirty(
    mut events: MessageReader<AssetEvent<Gfx>>,
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

fn write_gfx_pixel(
    gfx: &Gfx,
    gfx_material: &GfxMaterial,
    palette_data: &[[u8; 4]],
    color_index: u8,
    pixel_index: usize,
    bytes: &mut [u8],
) -> Result<(), Error> {
    if let Some(occupancy) = gfx.occupancy.as_ref() {
        if pixel_index >= occupancy.len() || !occupancy[pixel_index] {
            bytes.fill(0);
            return Ok(());
        }
        let pi = gfx_material.pal_map.map_or_mod(color_index as usize);
        let color = palette_data
            .get(pi)
            .ok_or(Error::NoSuch("palette color".into()))?;
        bytes[0..=3].copy_from_slice(color);
        bytes[3] = 0xff;
        return Ok(());
    }
    gfx_material
        .pal_map
        .write_color(palette_data, color_index, bytes)
}

/// Uses the CPU path when the gfx has per-pixel occupancy (screen canvases).
/// Otherwise the sprite image is an R8 index texture and [`GfxPaletteLookupMaterial`]
/// looks up the palette in the sprite shader.
pub(crate) fn compute_image(
    gfx_handle: &Handle<Gfx>,
    gfx_changed: bool,
    gfx_material: &GfxMaterial,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    palettes: &Palettes,
    pairs: &mut GfxImageMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
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
    let palette_data: Vec<[u8; 4]> = if let Some(palette_image) = images.get(&palette.image) {
        crate::pico8::pal::palette_data_from_image(palette_image, &palette.access)
    } else {
        trace_once!(
            "Palette image {:?} not yet in Assets<Image>; using default palette.",
            &palette.image
        );
        // Palette image not yet loaded (e.g. config-loaded asset); use default Pico-8 palette
        // extended to 256 entries so pal_map indices are in range.
        let default = crate::pico8::cart::PALETTE;
        let mut fallback = Vec::with_capacity(256);
        for i in 0..256 {
            fallback.push(default[i % default.len()]);
        }
        fallback
    };
    if let Some(handle) = pairs
        .get(&gfx_id)
        .and_then(|gfx_image| gfx_image.get(&hash).cloned())
    {
        if gfx_changed {
            let _my_span = info_span!("gfx::compute_image", name = "update image").entered();
            let gfx = gfxs.get(gfx_id);
            // `get_mut_untracked` so we can emit `AssetEvent` this frame. `get_mut`
            // only queues the event until the next `AssetEventSystems` tick, which
            // is too late for a same-frame screenshot of `pset`.
            if let Some((gfx, image)) = gfx.zip(images.get_mut_untracked(&handle)) {
                trace!("updating image for gfx {}", gfx_id);
                if let Some(data) = &mut image.data {
                    if let Err(e) = gfx.try_write_bytes(data, |i, pixel_index, bytes| {
                        write_gfx_pixel(gfx, gfx_material, &palette_data, i, pixel_index, bytes)
                    }) {
                        warn!("Unable to write color to handle {:?}: {e}", &handle);
                    } else {
                        image_events.write(AssetEvent::Modified { id: handle.id() });
                    }
                } else {
                    warn_once!("No data for image {}", gfx_id);
                }
            }
        }
        return Ok(handle);
    }
    let _my_span = info_span!("gfx::compute_image", name = "create image").entered();
    let gfx = gfxs
        .get(gfx_handle)
        .ok_or(Error::NoSuch("gfx image".into()))?;
    trace!("creating image for gfx {}", gfx_id);
    let image = images.add(gfx.try_to_image(|i, pixel_index, bytes| {
        write_gfx_pixel(gfx, gfx_material, &palette_data, i, pixel_index, bytes)
    })?);
    image_events.write(AssetEvent::Added { id: image.id() });
    // Update or add image to the map.
    pairs
        .entry(gfx_id)
        .and_modify(|gfx_image| {
            gfx_image.insert(hash, image.clone());
        })
        .or_insert_with(|| GfxImage::new(hash, image.clone()));
    Ok(image)
}

// Informed from Bevy's Sprite::compute_slices_on_asset_event.
#[allow(clippy::too_many_arguments)]
fn compute_image_on_asset_event(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<Gfx>>,
    mut images: ResMut<Assets<Image>>,
    mut palette_materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
    mut index_images: ResMut<IndexImageMap>,
    mut gpu_materials: ResMut<GfxGpuMaterialMap>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    _state: Res<Pico8State>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>)>,
    mut pairs: ResMut<GfxImageMap>,
    mut update_ids: Local<Vec<Entity>>,
    mut update_images: Local<VecDeque<Handle<Image>>>,
    pico8_handle: Option<Res<Pico8Handle>>,
    pico8_assets: Res<Assets<Pico8Asset>>,
    mut image_events: MessageWriter<AssetEvent<Image>>,
    // mut update_images: Local<Vec<(Entity, Handle<Image>)>>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    // We store the asset ids of added/modified image assets.
    let added_handles: HashSet<_> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => Some(*id),
            AssetEvent::Removed { id } => {
                pairs.remove(id);
                index_images.remove(id);
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
        let Some(pico8_asset) = pico8_assets.get(&pico8_handle.handle) else {
            warn!("No pico8 asset setup during clear event.");
            return;
        };
        if gfxs
            .get(&gfx_sprite.image)
            .is_some_and(|gfx| gfx.occupancy.is_none())
        {
            match gpu_sprite_assets(
                &gfx_sprite.image,
                true,
                &gfx_sprite.material,
                gfx_material,
                &gfxs,
                &mut images,
                &mut palette_materials,
                &pico8_asset.palettes,
                &mut index_images,
                &mut gpu_materials,
                &mut image_events,
            ) {
                Ok((image, material)) => {
                    commands.entity(id).insert(SpriteMaterial(material));
                    match sprite {
                        Some(sprite) => {
                            if sprite.image != image {
                                update_ids.push(id);
                                update_images.push_back(image);
                            }
                        }
                        None => {
                            commands.entity(id).insert(Sprite::from_image(image));
                        }
                    }
                }
                Err(e) => {
                    warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
                }
            }
            continue;
        }
        let image_handle = compute_image(
            &gfx_sprite.image,
            true,
            gfx_material,
            &gfxs,
            &mut images,
            &pico8_asset.palettes,
            &mut pairs,
            &mut image_events,
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
    let mut iter = sprites.iter_many_mut(update_ids.iter()).matched();
    while let Some((_, _, sprite)) = iter.fetch_next() {
        match sprite {
            Some(mut sprite) => {
                sprite.image = update_images.pop_front().unwrap();
            }
            _ => unreachable!(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_image_on_gfx_sprite_change(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut palette_materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
    mut index_images: ResMut<IndexImageMap>,
    mut gpu_materials: ResMut<GfxGpuMaterialMap>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    _state: Res<Pico8State>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>), Changed<GfxSprite>>,
    mut pairs: ResMut<GfxImageMap>,
    pico8_handle: Option<Res<Pico8Handle>>,
    pico8_assets: Res<Assets<Pico8Asset>>,
    mut image_events: MessageWriter<AssetEvent<Image>>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    let mut pico8_asset_maybe = None;
    for (id, gfx_sprite, sprite) in &mut sprites {
        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            trace!("No gfx material for gfx sprite {}", id);
            continue;
        };

        if pico8_asset_maybe.is_none() {
            pico8_asset_maybe = pico8_assets.get(&pico8_handle.handle);
        }
        let Some(pico8_asset) = &pico8_asset_maybe else {
            warn!("No pico8 asset setup during gfx sprite change.");
            return;
        };
        if gfxs
            .get(&gfx_sprite.image)
            .is_some_and(|gfx| gfx.occupancy.is_none())
        {
            match gpu_sprite_assets(
                &gfx_sprite.image,
                false,
                &gfx_sprite.material,
                gfx_material,
                &gfxs,
                &mut images,
                &mut palette_materials,
                &pico8_asset.palettes,
                &mut index_images,
                &mut gpu_materials,
                &mut image_events,
            ) {
                Ok((image, material)) => {
                    commands.entity(id).insert(SpriteMaterial(material));
                    match sprite {
                        Some(mut sprite) => {
                            sprite.image = image;
                        }
                        None => {
                            commands.entity(id).insert(Sprite::from_image(image));
                        }
                    }
                }
                Err(e) => {
                    warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
                }
            }
            continue;
        }
        let image_handle = compute_image(
            &gfx_sprite.image,
            false,
            gfx_material,
            &gfxs,
            &mut images,
            &pico8_asset.palettes,
            &mut pairs,
            &mut image_events,
        );
        match image_handle {
            Ok(image) => match sprite {
                Some(mut sprite) => {
                    // trace!("updating existant sprite on {}", id);
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

#[derive(thiserror::Error, Debug)]
pub enum PngError {
    #[error("Not an indexed png")]
    NotIndexed,
    #[error("Unexpected bit-depth of {expected} but was {actual}")]
    BitDepth { expected: u8, actual: u8 },
    #[error("Cannot convert bit-depth for pixel {pixel_index} with value {pixel_value}")]
    BitDepthConversion { pixel_index: usize, pixel_value: u8 },
}
