pub mod const_bitdepth;
pub mod palette_lookup_material;
mod var_bitdepth;
use crate::{one_or_map::OneOrMap, pico8::*};
use bevy::asset::{AssetEventSystems, AssetId};
use bevy::image::ImageSampler;
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
        .init_resource::<TilesetArrayMap>()
        .init_asset::<Gfx>()
        .init_asset::<GfxMaterial>()
        .add_systems(
            PostUpdate,
            (
                freeze_fixed_palette_luts,
                compute_image_on_asset_event,
                compute_image_on_gfx_sprite_change,
                check_dirty,
            )
                .chain()
                .after(AssetEventSystems),
        );
}

type GfxImage = OneOrMap<u64, Handle<Image>>;

#[derive(Component, Reflect)]
pub struct GfxSprite {
    pub image: Handle<Gfx>,
    pub material: Handle<GfxMaterial>,
    /// Draw palette for this sprite. The asset owns the `Rg8Uint` lookup image.
    pub pal_map_lut: Handle<PalMapLut>,
}

/// Clones this sprite's [`PalMapLut`] into a private asset that mode 1 will not rewrite.
#[derive(Component, Reflect, Debug, Default)]
pub struct FixedPaletteLut;

#[derive(Component)]
struct PaletteLutFrozen;

#[derive(Asset, Debug, Reflect, Clone, Hash, PartialEq, Eq)]
pub struct GfxMaterial {
    pub palette: usize,
    pub pal_map: PalMap,
}

#[derive(Resource, Default, Reflect, Deref, DerefMut)]
pub struct GfxImageMap(HashMap<AssetId<Gfx>, GfxImage>);

#[derive(Resource, Default, Deref, DerefMut)]
struct IndexImageMap(HashMap<AssetId<Gfx>, Handle<Image>>);

/// Baked 2D-array tileset for one Gfx sheet and draw state.
#[derive(Clone)]
struct TilesetEntry {
    array: Handle<Image>,
    gfx: Handle<Gfx>,
    material: GfxMaterial,
    sprite_size: UVec2,
}

/// Array tilesets keyed by source asset. Gfx entries also key on the baked palette hash so two
/// `pal()` states of the same sheet can exist in one frame.
#[derive(Resource, Default)]
pub(crate) struct TilesetArrayMap {
    gfx: HashMap<(AssetId<Gfx>, u64), TilesetEntry>,
    images: HashMap<AssetId<Image>, Handle<Image>>,
}

impl TilesetArrayMap {
    pub(crate) fn remove_gfx(&mut self, id: AssetId<Gfx>) {
        self.gfx.retain(|(gfx_id, _), _| *gfx_id != id);
    }
}

fn baked_image_hash(gfx_material: &GfxMaterial) -> u64 {
    let mut hasher = DefaultHasher::new();
    gfx_material.pal_map.hash(&mut hasher);
    gfx_material.palette.hash(&mut hasher);
    hasher.finish()
}

fn stacked_tileset(sheet: &Image, sprite_size: UVec2) -> Result<Image, Error> {
    if sprite_size.x == 0 || sprite_size.y == 0 {
        return Err(Error::InvalidArgument("sprite size".into()));
    }
    if sheet.data.is_none() {
        return Err(Error::NoAsset("tileset sheet pixels".into()));
    }
    let cols = sheet.width() / sprite_size.x;
    let rows = sheet.height() / sprite_size.y;
    let mut array = sheet
        .create_stacked_array_from_2d_grid(rows, cols)
        .map_err(|err| Error::Message(format!("tileset array: {err}").into()))?;
    array.sampler = ImageSampler::nearest();
    Ok(array)
}

/// RGBA sheet from [`compute_image`], stacked into one array layer per sprite.
pub(crate) fn tileset_array_image(
    gfx_handle: &Handle<Gfx>,
    gfx_changed: bool,
    gfx_material: &GfxMaterial,
    sprite_size: UVec2,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    palettes: &Palettes,
    pairs: &mut GfxImageMap,
    cache: &mut TilesetArrayMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<Handle<Image>, Error> {
    let hash = baked_image_hash(gfx_material);
    let key = (gfx_handle.id(), hash);
    if !gfx_changed && let Some(entry) = cache.gfx.get(&key) {
        return Ok(entry.array.clone());
    }
    let rgba = compute_image(
        gfx_handle,
        gfx_changed,
        gfx_material,
        gfxs,
        images,
        palettes,
        pairs,
        image_events,
    )?;
    let array = stacked_tileset(
        images
            .get(&rgba)
            .ok_or(Error::NoAsset("tileset sheet".into()))?,
        sprite_size,
    )?;
    if let Some(entry) = cache.gfx.get(&key) {
        let handle = entry.array.clone();
        if let Some(image) = images.get_mut_untracked(&handle) {
            *image = array;
            image_events.write(AssetEvent::Modified { id: handle.id() });
        }
        return Ok(handle);
    }
    let handle = images.add(array);
    image_events.write(AssetEvent::Added { id: handle.id() });
    cache.gfx.insert(
        key,
        TilesetEntry {
            array: handle.clone(),
            gfx: gfx_handle.clone(),
            material: gfx_material.clone(),
            sprite_size,
        },
    );
    Ok(handle)
}

/// Rebuild cached Gfx tilesets after the sheet asset changes (`sset`).
pub(crate) fn refresh_tileset_arrays(
    changed: &HashSet<AssetId<Gfx>>,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    palettes: &Palettes,
    pairs: &mut GfxImageMap,
    cache: &mut TilesetArrayMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) {
    if changed.is_empty() {
        return;
    }
    let pending: Vec<TilesetEntry> = cache
        .gfx
        .iter()
        .filter(|((id, _), _)| changed.contains(id))
        .map(|(_, entry)| entry.clone())
        .collect();
    for entry in pending {
        if tileset_array_image(
            &entry.gfx,
            true,
            &entry.material,
            entry.sprite_size,
            gfxs,
            images,
            palettes,
            pairs,
            cache,
            image_events,
        )
        .is_err()
        {
            trace!("tileset array refresh failed for {}", entry.gfx.id());
        }
    }
}

/// Stack an already-RGBA sprite sheet. Cached by the source image id.
pub(crate) fn image_tileset_array(
    image_handle: &Handle<Image>,
    sprite_size: UVec2,
    images: &mut Assets<Image>,
    cache: &mut TilesetArrayMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<Handle<Image>, Error> {
    let id = image_handle.id();
    if let Some(handle) = cache.images.get(&id) {
        return Ok(handle.clone());
    }
    let array = stacked_tileset(
        images
            .get(id)
            .ok_or(Error::NoAsset("sprite sheet image".into()))?,
        sprite_size,
    )?;
    let handle = images.add(array);
    image_events.write(AssetEvent::Added { id: handle.id() });
    cache.images.insert(id, handle.clone());
    Ok(handle)
}

/// Index image for an indexed sheet. The LUT handle lives on [`GfxSprite`].
fn index_sprite_image(
    gfx_handle: &Handle<Gfx>,
    gfx_changed: bool,
    gfxs: &Assets<Gfx>,
    images: &mut Assets<Image>,
    index_images: &mut IndexImageMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<Handle<Image>, Error> {
    let gfx_id = gfx_handle.id();
    let gfx = gfxs.get(gfx_id).ok_or(Error::NoSuch("gfx image".into()))?;
    if let Some(handle) = index_images.get(&gfx_id).cloned() {
        if gfx_changed && let Some(image) = images.get_mut_untracked(&handle) {
            *image = gfx.to_index_image();
            image_events.write(AssetEvent::Modified { id: handle.id() });
        }
        return Ok(handle);
    }
    let handle = images.add(gfx.to_index_image());
    image_events.write(AssetEvent::Added { id: handle.id() });
    index_images.insert(gfx_id, handle.clone());
    Ok(handle)
}

fn lookup_material(
    gfx_material: &GfxMaterial,
    lut: &Handle<Image>,
    images: &Assets<Image>,
    palettes: &Palettes,
) -> Result<GfxPaletteLookupMaterial, Error> {
    let palette = palettes.get_pal(gfx_material.palette)?;
    let palette_image_ref = images.get(&palette.image);
    Ok(build_palette_lookup_material(
        palette.image().clone(),
        lut.clone(),
        palette,
        palette_image_ref,
    ))
}

fn materials_match(existing: &GfxPaletteLookupMaterial, next: &GfxPaletteLookupMaterial) -> bool {
    existing.lut_texture.id() == next.lut_texture.id()
        && existing.palette_texture.id() == next.palette_texture.id()
        && existing.palette_width == next.palette_width
        && existing.palette_height == next.palette_height
        && existing.access_kind == next.access_kind
        && existing.access_param == next.access_param
}

fn assign_lookup_material(
    commands: &mut Commands,
    palette_materials: &mut Assets<GfxPaletteLookupMaterial>,
    entity: Entity,
    current: Option<&SpriteMaterial<GfxPaletteLookupMaterial>>,
    material: GfxPaletteLookupMaterial,
) {
    if let Some(handle) = current.map(|sprite_material| sprite_material.0.clone()) {
        if palette_materials
            .get(&handle)
            .is_some_and(|existing| materials_match(existing, &material))
        {
            return;
        }
        if let Some(mut existing) = palette_materials.get_mut(&handle) {
            *existing = material;
        }
        return;
    }
    // The sprite may have been given a material earlier this frame, before commands apply.
    commands.queue(move |world: &mut World| {
        let existing = world
            .get::<SpriteMaterial<GfxPaletteLookupMaterial>>(entity)
            .map(|sprite_material| sprite_material.0.clone());
        if let Some(handle) = existing {
            let mut materials = world.resource_mut::<Assets<GfxPaletteLookupMaterial>>();
            if materials
                .get(&handle)
                .is_some_and(|current| materials_match(current, &material))
            {
                return;
            }
            if let Some(mut current) = materials.get_mut(&handle) {
                *current = material;
            }
            return;
        }
        let handle = world
            .resource_mut::<Assets<GfxPaletteLookupMaterial>>()
            .add(material);
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.insert(SpriteMaterial(handle));
        }
    });
}

fn freeze_fixed_palette_luts(
    mut commands: Commands,
    state: Res<Pico8State>,
    mut images: ResMut<Assets<Image>>,
    mut pal_map_luts: ResMut<Assets<PalMapLut>>,
    mut sprites: Query<
        (Entity, &mut GfxSprite),
        (With<FixedPaletteLut>, Without<PaletteLutFrozen>),
    >,
) {
    for (entity, mut gfx_sprite) in &mut sprites {
        let Some(source) = pal_map_luts.get(&gfx_sprite.pal_map_lut).cloned() else {
            continue;
        };
        let frozen = source.freeze_clone(&state.display_pal_map, &mut images);
        gfx_sprite.pal_map_lut = pal_map_luts.add(frozen);
        commands.entity(entity).insert(PaletteLutFrozen);
    }
}

fn sprite_lut_image(
    pal_map_lut: &Handle<PalMapLut>,
    display: &PalMap,
    pal_map_luts: &mut Assets<PalMapLut>,
    images: &mut Assets<Image>,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<Handle<Image>, Error> {
    if let Some(image) = pal_map_luts
        .get(pal_map_lut)
        .and_then(PalMapLut::prepared_image)
    {
        return Ok(image);
    }
    let Some(lut) = pal_map_luts.get_mut_untracked(pal_map_lut) else {
        return Err(Error::NoAsset("pal map lut".into()));
    };
    let write = lut.ensure_image(display, images);
    if write.added {
        image_events.write(AssetEvent::Added {
            id: write.handle.id(),
        });
    } else if write.modified {
        image_events.write(AssetEvent::Modified {
            id: write.handle.id(),
        });
    }
    Ok(write.handle)
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
        // `pset` already stored the draw-palette color. Applying `pal_map`
        // again recolors pixels written after `pal()`.
        let color = palette_data
            .get(color_index as usize)
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
    let hash = baked_image_hash(gfx_material);
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
    state: Res<Pico8State>,
    mut images: ResMut<Assets<Image>>,
    mut pal_map_luts: ResMut<Assets<PalMapLut>>,
    mut palette_materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
    mut index_images: ResMut<IndexImageMap>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    mut sprites: Query<(
        Entity,
        &GfxSprite,
        Option<&SpriteMaterial<GfxPaletteLookupMaterial>>,
        Option<&mut Sprite>,
    )>,
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
    for (id, gfx_sprite, sprite_material, sprite) in &sprites {
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
            match index_sprite_image(
                &gfx_sprite.image,
                true,
                &gfxs,
                &mut images,
                &mut index_images,
                &mut image_events,
            )
            .and_then(|image| {
                let lut = sprite_lut_image(
                    &gfx_sprite.pal_map_lut,
                    &state.display_pal_map,
                    &mut pal_map_luts,
                    &mut images,
                    &mut image_events,
                )?;
                lookup_material(gfx_material, &lut, &images, &pico8_asset.palettes)
                    .map(|material| (image, material))
            }) {
                Ok((image, material)) => {
                    assign_lookup_material(
                        &mut commands,
                        &mut palette_materials,
                        id,
                        sprite_material,
                        material,
                    );
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
    while let Some((_, _, _, sprite)) = iter.fetch_next() {
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
    state: Res<Pico8State>,
    mut images: ResMut<Assets<Image>>,
    mut pal_map_luts: ResMut<Assets<PalMapLut>>,
    mut palette_materials: ResMut<Assets<GfxPaletteLookupMaterial>>,
    mut index_images: ResMut<IndexImageMap>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    mut sprites: Query<
        (
            Entity,
            &GfxSprite,
            Option<&SpriteMaterial<GfxPaletteLookupMaterial>>,
            Option<&mut Sprite>,
        ),
        Changed<GfxSprite>,
    >,
    mut pairs: ResMut<GfxImageMap>,
    pico8_handle: Option<Res<Pico8Handle>>,
    pico8_assets: Res<Assets<Pico8Asset>>,
    mut image_events: MessageWriter<AssetEvent<Image>>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    let mut pico8_asset_maybe = None;
    for (id, gfx_sprite, sprite_material, mut sprite) in &mut sprites {
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
            match index_sprite_image(
                &gfx_sprite.image,
                false,
                &gfxs,
                &mut images,
                &mut index_images,
                &mut image_events,
            )
            .and_then(|image| {
                let lut = sprite_lut_image(
                    &gfx_sprite.pal_map_lut,
                    &state.display_pal_map,
                    &mut pal_map_luts,
                    &mut images,
                    &mut image_events,
                )?;
                lookup_material(gfx_material, &lut, &images, &pico8_asset.palettes)
                    .map(|material| (image, material))
            }) {
                Ok((image, material)) => {
                    assign_lookup_material(
                        &mut commands,
                        &mut palette_materials,
                        id,
                        sprite_material,
                        material,
                    );
                    match &mut sprite {
                        Some(sprite) => {
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

#[cfg(test)]
mod fixed_lut_tests {
    use super::*;
    use crate::pico8::PalMap;

    #[test]
    fn fixed_palette_lut_clones_a_private_asset() {
        let mut app = App::new();
        app.init_resource::<crate::pico8::Defaults>();
        app.init_resource::<Assets<Image>>();
        app.init_resource::<Assets<PalMapLut>>();
        app.init_resource::<Pico8State>();
        app.add_systems(Update, freeze_fixed_palette_luts);

        let mut lut = PalMapLut::new(PalMap::default());
        {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            let _ = lut.ensure_image(&PalMap::default(), &mut images);
        }
        let shared = app.world_mut().resource_mut::<Assets<PalMapLut>>().add(lut);
        let shared_id = shared.id();
        let entity = app
            .world_mut()
            .spawn((
                GfxSprite {
                    image: Handle::default(),
                    material: Handle::default(),
                    pal_map_lut: shared,
                },
                FixedPaletteLut,
            ))
            .id();

        app.update();

        let sprite = app.world().get::<GfxSprite>(entity).unwrap();
        assert_ne!(sprite.pal_map_lut.id(), shared_id);
        assert!(app.world().get::<PaletteLutFrozen>(entity).is_some());
        let pal_map_luts = app.world().resource::<Assets<PalMapLut>>();
        let frozen = pal_map_luts.get(&sprite.pal_map_lut).unwrap();
        assert!(frozen.frozen);
        assert!(frozen.image.is_some());
        assert_ne!(
            frozen.image.as_ref().unwrap().id(),
            pal_map_luts
                .get(shared_id)
                .unwrap()
                .image
                .as_ref()
                .unwrap()
                .id()
        );
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
