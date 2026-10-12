use crate::pico8::{
    self, Error, Gfx, GfxMaterial, Pico8Asset, Pico8Handle, SprHandle, SpriteSheet,
};
use bevy::asset::AssetEventSystems;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::sprite_render::{AlphaMode2d, TileData, TilemapChunk, TilemapChunkTileData};

#[cfg(feature = "level")]
use crate::level;

#[derive(Clone, Debug, Reflect)]
pub enum SpriteMap {
    P8(Handle<P8Map>),
    #[cfg(feature = "level")]
    Level(level::Tiled),
}

#[derive(Clone, Debug, Deref, DerefMut, Reflect, Asset)]
pub struct P8Map {
    #[deref]
    pub entries: Vec<u8>,
}

/// Set once the chunk child exists, so a sheet that is still loading can retry next frame.
#[derive(Component)]
struct MapChunkSpawned;

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<P8Map>()
        .add_systems(PostUpdate, add_tilemap_chunks.after(AssetEventSystems));
}

impl From<Handle<P8Map>> for SpriteMap {
    fn from(map: Handle<P8Map>) -> Self {
        SpriteMap::P8(map)
    }
}

/// This will add the tilemap when it is added to the scene.
#[derive(Component, Reflect)]
pub struct P8SpriteMap {
    pub map_index: usize,
    pub sprite_sheet: Handle<pico8::SpriteSheet>,
    pub gfx_material: Handle<pico8::GfxMaterial>,
    pub rect: URect,
    pub mask: Option<u8>,
}

fn add_tilemap_chunks(
    mut commands: Commands,
    query: Query<(Entity, &P8SpriteMap), Without<MapChunkSpawned>>,
    mut gfx_events: MessageReader<AssetEvent<Gfx>>,
    sprite_sheets: Res<Assets<SpriteSheet>>,
    pico8_assets: Res<Assets<Pico8Asset>>,
    pico8_handle: Option<Res<Pico8Handle>>,
    p8_maps: Res<Assets<P8Map>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut pairs: ResMut<pico8::GfxImageMap>,
    mut tilesets: ResMut<pico8::gfx::TilesetArrayMap>,
    mut image_events: MessageWriter<AssetEvent<Image>>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    let Some(pico8_asset) = pico8_assets.get(&pico8_handle.handle) else {
        return;
    };
    let mut changed_gfx = HashSet::new();
    for event in gfx_events.read() {
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_gfx.insert(*id);
            }
            AssetEvent::Removed { id } => {
                tilesets.remove_gfx(*id);
            }
            _ => {}
        }
    }
    pico8::gfx::refresh_tileset_arrays(
        &changed_gfx,
        &gfxs,
        &mut images,
        &pico8_asset.palettes,
        &mut pairs,
        &mut tilesets,
        &mut image_events,
    );

    for (id, p8sprite_map) in &query {
        let Some(sprite_sheet) = sprite_sheets.get(&p8sprite_map.sprite_sheet) else {
            trace!("sprite sheet for map is not loaded");
            continue;
        };
        let map_pos = p8sprite_map.rect.min;
        let chunk_size = p8sprite_map.rect.size();
        if chunk_size.x == 0 || chunk_size.y == 0 {
            commands.entity(id).insert(MapChunkSpawned);
            continue;
        }
        let Some(sprite_map) = pico8_asset.maps.get(p8sprite_map.map_index) else {
            trace!("no sprite map at index {}", p8sprite_map.map_index);
            continue;
        };
        let map_handle = match sprite_map {
            SpriteMap::P8(handle) => handle,
            #[cfg(feature = "level")]
            SpriteMap::Level(_) => {
                commands.entity(id).insert(MapChunkSpawned);
                continue;
            }
        };
        let Some(map_asset) = p8_maps.get(map_handle) else {
            trace!("p8 map asset is not loaded");
            continue;
        };

        let gfx_changed = match &sprite_sheet.handle {
            SprHandle::Gfx(handle) => changed_gfx.contains(&handle.id()),
            SprHandle::Image(_) => false,
        };
        let tileset = match tileset_handle(
            sprite_sheet,
            &p8sprite_map.gfx_material,
            gfx_changed,
            &gfxs,
            &gfx_materials,
            &mut images,
            &pico8_asset.palettes,
            &mut pairs,
            &mut tilesets,
            &mut image_events,
        ) {
            Ok(tileset) => tileset,
            Err(err) => {
                trace!("map tileset not ready: {err}");
                continue;
            }
        };

        let tile_data = tile_data_from_entries(
            chunk_size,
            map_pos,
            &map_asset.entries,
            p8sprite_map.mask,
            &sprite_sheet.flags,
        );
        let tile_size = sprite_sheet.sprite_size;
        let pixel_size = chunk_size * tile_size;
        commands.entity(id).with_children(|parent| {
            parent.spawn((
                Name::new("map-chunk"),
                Transform::from_xyz(pixel_size.x as f32 / 2.0, -(pixel_size.y as f32) / 2.0, 0.0),
                Visibility::Inherited,
                TilemapChunk {
                    chunk_size,
                    tile_display_size: tile_size,
                    tileset,
                    alpha_mode: AlphaMode2d::Blend,
                },
                TilemapChunkTileData::from_y_down_tiles(chunk_size, tile_data),
            ));
        });
        commands.entity(id).insert(MapChunkSpawned);
    }
}

fn tile_data_from_entries(
    chunk_size: UVec2,
    map_pos: UVec2,
    entries: &[u8],
    mask: Option<u8>,
    flags: &[u8],
) -> Vec<Option<TileData>> {
    let mut data = Vec::with_capacity((chunk_size.x * chunk_size.y) as usize);
    for y in 0..chunk_size.y {
        for x in 0..chunk_size.x {
            let index = entries
                .get((map_pos.x + x + (map_pos.y + y) * pico8::MAP_COLUMNS) as usize)
                .copied()
                .unwrap_or(0);
            // Pico-8: omitted or 0 draws every sprite. A non-zero mask draws a
            // sprite when any of those flag bits is set.
            let passes_mask = mask.is_none_or(|mask| {
                mask == 0
                    || flags
                        .get(index as usize)
                        .is_some_and(|sprite_flags| sprite_flags & mask != 0)
            });
            if index != 0 && passes_mask {
                data.push(Some(TileData::from_tileset_index(index as u16)));
            } else {
                data.push(None);
            }
        }
    }
    data
}

#[allow(clippy::too_many_arguments)]
fn tileset_handle(
    sprite_sheet: &SpriteSheet,
    gfx_material_handle: &Handle<GfxMaterial>,
    gfx_changed: bool,
    gfxs: &Assets<Gfx>,
    gfx_materials: &Assets<GfxMaterial>,
    images: &mut Assets<Image>,
    palettes: &pico8::Palettes,
    pairs: &mut pico8::GfxImageMap,
    tilesets: &mut pico8::gfx::TilesetArrayMap,
    image_events: &mut MessageWriter<AssetEvent<Image>>,
) -> Result<Handle<Image>, Error> {
    match &sprite_sheet.handle {
        SprHandle::Gfx(handle) => {
            let gfx_material = gfx_materials
                .get(gfx_material_handle)
                .ok_or(Error::NoSuch("gfx material".into()))?;
            pico8::gfx::tileset_array_image(
                handle,
                gfx_changed,
                gfx_material,
                sprite_sheet.sprite_size,
                gfxs,
                images,
                palettes,
                pairs,
                tilesets,
                image_events,
            )
        }
        SprHandle::Image(handle) => pico8::gfx::image_tileset_array(
            handle,
            sprite_sheet.sprite_size,
            images,
            tilesets,
            image_events,
        ),
    }
}

#[cfg(feature = "level")]
impl From<level::Tiled> for SpriteMap {
    fn from(map: level::Tiled) -> Self {
        SpriteMap::Level(map)
    }
}
