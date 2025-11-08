use crate::pico8::{self, Error, Gfx, GfxMaterial, SprHandle, SpriteSheet};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use std::collections::VecDeque;

#[cfg(feature = "level")]
use crate::level;
use bevy_ecs_tilemap::prelude::*;

#[derive(Clone, Debug, Reflect)]
pub enum SpriteMap {
    P8(Handle<P8Map>),
    #[cfg(feature = "level")]
    Level(level::Tiled),
}
#[derive(Component, Reflect)]
pub struct GfxTilemapTexture {
    image: Handle<Gfx>,
    material: Handle<GfxMaterial>,
}

impl SpriteMap {
    // pub fn entries(&self) -> &[u8] {
    //     match self {
    //         SpriteMap::P8(p8map) => &p8map.entries,
    //     }
    // }
}

#[derive(Clone, Debug, Deref, DerefMut, Reflect, Asset)]
pub struct P8Map {
    #[deref]
    pub entries: Vec<u8>,
}

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<GfxTilemapTexture>()
        .register_type::<P8SpriteMap>()
        .init_asset::<P8Map>()
        .add_systems(
            PostUpdate,
            (
                add_tilemaps,
                compute_gfx_tilemap_texture_on_asset_event.after(add_tilemaps),
                compute_image_on_gfx_tilemap_texture_change
                    .after(compute_gfx_tilemap_texture_on_asset_event),
            ),
        );
}

impl From<Handle<P8Map>> for SpriteMap {
    fn from(map: Handle<P8Map>) -> Self {
        SpriteMap::P8(map)
    }
}

#[derive(Component, Reflect)]
pub struct P8SpriteMap {
    // TODO: This should really be a Handle<P8Map>.
    pub map_index: usize,
    pub sprite_sheet: Handle<pico8::SpriteSheet>,
    pub gfx_material: Handle<pico8::GfxMaterial>,
    pub rect: URect,
    pub mask: Option<u8>,
}

// Informed from Bevy's Sprite::compute_slices_on_asset_event.
#[allow(clippy::too_many_arguments)]
fn compute_gfx_tilemap_texture_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Gfx>>,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    palettes: Res<pico8::Palettes>,
    mut textures: Query<(Entity, &GfxTilemapTexture, Option<&mut TilemapTexture>)>,
    mut pairs: ResMut<pico8::GfxImageMap>,
    mut update_ids: Local<Vec<Entity>>,
    mut update_images: Local<VecDeque<Handle<Image>>>,
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
    for (id, gfx_sprite, sprite) in &textures {
        if !added_handles.contains(&gfx_sprite.image.id()) {
            continue;
        }
        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            continue;
        };
        let image_handle = crate::pico8::gfx::compute_image(
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
                    Some(tilemap_texture) => {
                        match tilemap_texture {
                            TilemapTexture::Single(handle) if *handle != image => {
                                // trace!("updating existant sprite on {}", id);
                                update_ids.push(id);
                                update_images.push_back(image);
                            }
                            _ => {}
                        }
                    }
                    None => {
                        // trace!("inserting new sprite into {}", id);
                        commands.entity(id).insert(TilemapTexture::Single(image));
                    }
                }
            }
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
    // Try not to trigger a sprite change if we don't have to.
    let mut iter = textures.iter_many_mut(update_ids.iter());
    while let Some((_, _, tilemap_texture)) = iter.fetch_next() {
        match tilemap_texture {
            Some(mut tilemap_texture) => {
                *tilemap_texture = TilemapTexture::Single(update_images.pop_front().unwrap());
            }
            _ => unreachable!(),
        }
    }
}

fn compute_image_on_gfx_tilemap_texture_change(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    palettes: Res<pico8::Palettes>,
    mut sprites: Query<
        (Entity, &GfxTilemapTexture, Option<&mut TilemapTexture>),
        Changed<GfxTilemapTexture>,
    >,
    mut pairs: ResMut<pico8::GfxImageMap>,
) {
    for (id, gfx_sprite, sprite) in &mut sprites {
        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            continue;
        };
        let image_handle = crate::pico8::gfx::compute_image(
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
                Some(mut tilemap_texture) => {
                    trace!("updating existant tilemap texture on {}", id);
                    *tilemap_texture = TilemapTexture::Single(image);
                }
                None => {
                    trace!("inserting new tilemap texture into {}", id);
                    commands.entity(id).insert(TilemapTexture::Single(image));
                }
            },
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
}

fn add_tilemaps(
    query: Query<(Entity, &P8SpriteMap), Added<P8SpriteMap>>,
    sprite_sheets: Res<Assets<SpriteSheet>>,
    pico8_asset: Res<Assets<pico8::Pico8Asset>>,
    pico8_handle: Res<pico8::Pico8Handle>,
    p8_maps: Res<Assets<pico8::P8Map>>,
    mut commands: Commands,
) {
    let mut p8map_maybe = None;
    for (id, p8sprite_map) in &query {
        let size = p8sprite_map.rect.size();
        let map_size = TilemapSize::from(size);
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = commands.spawn_empty().id();
        if p8map_maybe.is_none() {
            match pico8_asset
                .get(&pico8_handle.handle)
                .ok_or(Error::NoSuch("pico8_asset".into()))
                .and_then(|asset| asset.sprite_map(Some(p8sprite_map.map_index)))
            {
                Ok(x) => p8map_maybe = Some(x),
                Err(e) => {
                    warn!("Could not get map: {e}");
                    continue;
                }
            }
        }
        let p8map = p8map_maybe.unwrap();
        let Some(sprite_sheet) = sprite_sheets.get(&p8sprite_map.sprite_sheet) else {
            warn!("Could not get sprite_sheet for map");
            continue;
        };

        let map_pos = p8sprite_map.rect.min;
        // let map_entity = commands
        //     .spawn((
        //         Name::new("map"),
        //         Transform::from_translation(screen_start.extend(clearable.suggest_z())),
        //         Visibility::Inherited,
        //         clearable,
        //     ))
        //     .id();
        let mask = p8sprite_map.mask;
        commands
            .entity(tilemap_entity)
            .with_children(|builder| {
                for x in 0..map_size.x {
                    for y in 0..map_size.y {
                        let entries = match p8map {
                            SpriteMap::P8(handle) => {
                                let Some(p8map_asset) = p8_maps.get(handle) else {
                                    warn!("Unable to get p8 map");
                                    continue;
                                };
                                &p8map_asset.entries
                            }
                        };
                        let texture_index = entries
                            .get((map_pos.x + x + (map_pos.y + y) * pico8::MAP_COLUMNS) as usize)
                            .and_then(|index| {
                                if let Some(mask) = mask {
                                    sprite_sheet
                                        .flags
                                        .get(*index as usize)
                                        .map(|flags| flags & mask == mask)
                                        .unwrap_or(true)
                                        .then_some(index)
                                } else {
                                    Some(index)
                                }
                            })
                            .copied()
                            .unwrap_or(0);
                        if texture_index != 0 {
                            let tile_pos = TilePos {
                                x,
                                y: map_size.y - y - 1,
                            };
                            let tile_entity = builder
                                .spawn((TileBundle {
                                    position: tile_pos,
                                    tilemap_id: TilemapId(tilemap_entity),
                                    texture_index: TileTextureIndex(texture_index as u32),
                                    ..Default::default()
                                },))
                                .id();
                            tile_storage.set(&tile_pos, tile_entity);
                        }
                    }
                }
            })
            .insert(ChildOf(id));

        let tile_size: TilemapTileSize = sprite_sheet.sprite_size.as_vec2().into();
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();
        let transform = get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, 0.0);
        let mut gfx_handle = None;

        commands.entity(tilemap_entity).insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(match &sprite_sheet.handle {
                SprHandle::Image(handle) => handle.clone(),
                SprHandle::Gfx(handle) => {
                    gfx_handle = Some(handle.clone());
                    Handle::default()
                    // gfx_to_image(handle)?,
                }
            }),
            tile_size,
            transform,
            ..Default::default()
        });
        if let Some(gfx_handle) = gfx_handle {
            let material = p8sprite_map.gfx_material.clone();
            commands.entity(tilemap_entity).insert(GfxTilemapTexture {
                image: gfx_handle,
                material,
            });
        }
        //     if let Some(gfx_handle) = gfx_handle {
        //         commands.queue(move |world: &mut World| {
        //             // let gfx_material = world.resource::<pico8::Pico8State>().gfx_material();
        //             // match world.run_system_cached_with(pico8::gfx::compute_image_sys, gfx_handle) {
        //             match world.run_system_cached_with(
        //                 pico8::gfx::compute_image_sys,
        //                 pico8::GfxSprite {
        //                     image: gfx_handle,
        //                     material: p8sprite_map.gfx_material.clone(),
        //                 },
        //             ) {
        //                 Ok(result) => match result {
        //                     Ok(image_handle) => {
        //                         world
        //                             .entity_mut(tilemap_entity)
        //                             .insert(TilemapTexture::Single(image_handle));
        //                     }
        //                     Err(e) => {
        //                         error!(
        //                             "Unable to create sprite from gfx for tilemap {}",
        //                             tilemap_entity
        //                         );
        //                     }
        //                 },
        //                 Err(e) => {
        //                     error!(
        //                         "Unable to run system to create sprite from gfx for tilemap {}",
        //                         tilemap_entity
        //                     );
        //                 }
        //             }
        //         })
        //     }
    }
}

// TODO: Add a P8Map, then have a system construct the tilemap, not this junk.

/// Calculates a [`Transform`] for a tilemap that places it so that its center is at
/// `(0.0, 0.0, 0.0)` in world space.
pub(crate) fn get_tilemap_top_left_transform(
    size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    z: f32,
) -> Transform {
    assert_eq!(map_type, &TilemapType::Square);
    let y = size.y as f32 * grid_size.y;
    Transform::from_xyz(grid_size.x / 2.0, -y + grid_size.y / 2.0, z)
}

#[cfg(feature = "level")]
impl From<level::Tiled> for SpriteMap {
    fn from(map: level::Tiled) -> Self {
        SpriteMap::Level(map)
    }
}
