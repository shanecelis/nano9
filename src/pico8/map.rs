use crate::pico8::{self, Clearable, Error, Gfx, SprHandle};
use bevy::prelude::*;

#[cfg(feature = "level")]
use crate::level;
use bevy_ecs_tilemap::prelude::*;

#[derive(Clone, Debug, Reflect)]
pub enum SpriteMap {
    P8(P8Map),
    #[cfg(feature = "level")]
    Level(level::Tiled),
}

#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct P8Map {
    #[deref]
    pub entries: Vec<u8>,
    pub sheet_index: usize,
}

impl From<P8Map> for SpriteMap {
    fn from(map: P8Map) -> Self {
        SpriteMap::P8(map)
    }
}

impl P8Map {
    pub fn map(
        &self,
        map_pos: UVec2,
        screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        sprite_sheets: &[pico8::SpriteSheet],
        hash: Option<u64>,
        commands: &mut Commands,
    ) -> Result<Entity, pico8::Error> {
        let map_size = TilemapSize::from(size);
        let mut clearable = Clearable::new(2);
        clearable.hash = hash;
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = commands.spawn_empty().id();

        let map_entity = commands
            .spawn((
                Name::new("map"),
                Transform::from_translation(screen_start.extend(clearable.suggest_z())),
                Visibility::Inherited,
                clearable,
            ))
            .id();
        commands
            .entity(tilemap_entity)
            .with_children(|builder| {
                for x in 0..map_size.x {
                    for y in 0..map_size.y {
                        let texture_index = self
                            .entries
                            .get((map_pos.x + x + (map_pos.y + y) * pico8::MAP_COLUMNS) as usize)
                            .and_then(|index| {
                                if let Some(mask) = mask {
                                    sprite_sheets
                                        .get(self.sheet_index)
                                        .and_then(|sprite_sheet| {
                                            (sprite_sheet.flags[*index as usize] & mask == mask)
                                                .then_some(index)
                                        })
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
            .set_parent(map_entity);

        let sprites = &sprite_sheets[self.sheet_index];
        let tile_size: TilemapTileSize = sprites.sprite_size.as_vec2().into();
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();
        let transform = get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, 0.0);
        let mut gfx_handle = None;

        commands.entity(tilemap_entity)
                .insert(TilemapBundle {
            grid_size,
            map_type,
            size: map_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(match &sprites.handle {
                SprHandle::Image(handle) => handle.clone(),
                SprHandle::Gfx(ref handle) => {
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
            commands.queue(move |world: &mut World| {
                match world.run_system_cached_with(pico8::gfx::compute_image_sys, gfx_handle) {
                    Ok(result) => match result {
                        Ok(image_handle) => {
                            world.entity_mut(tilemap_entity)
                                .insert(TilemapTexture::Single(image_handle));
                        }
                        Err(e) => {
                            error!("Unable to create sprite from gfx for tilemap {}", tilemap_entity);

                        }
                    }
                    Err(e) => {
                        error!("Unable to run system to create sprite from gfx for tilemap {}", tilemap_entity);
                    }
                }

            })

        }
        Ok(map_entity)
    }
}

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
