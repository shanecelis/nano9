use crate::{level, pico8};
use bevy::{ecs::system::SystemParam, prelude::*};
use tiled::{PropertyValue, Tileset};

pub(crate) fn plugin(app: &mut App) {
    // app.register_type::<TiledLookup>();
}

#[derive(Debug, Component, Reflect)]
pub enum TiledLookup {
    Object {
        layer: u32,
        idx: u32,
        // TODO: TiledMap is not an Asset
        // handle: Handle<TiledMap>,
        handle: bevy_ecs_tiled::prelude::TiledMap,
    },
}

#[derive(SystemParam)]
pub struct Level<'w, 's> {
    // TODO: TiledMap is not an Asset in bevy_ecs_tiled 0.9.5
    tiled_maps: ResMut<'w, Assets<bevy_ecs_tiled::prelude::TiledMapAsset>>,
    // tiled_worlds: ResMut<'w, Assets<bevy_ecs_tiled::prelude::TiledWorld>>,
    // TODO: Fix when bevy_ecs_tiled API is updated
    // tiled_id_storage: Query<'w, 's, (&'static TiledMapStorage, &'static TiledMapHandle)>,
    // sprites: Query<'w, 's, &'static mut Sprite>,
    tiled_lookups: Query<'w, 's, &'static TiledLookup>,
}
impl Level<'_, '_> {
    pub fn mget(
        &self,
        map: &level::Tiled,
        pos: Vec2,
        _map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<usize> {
        match map {
            level::Tiled::SpriteMap { handle } => {
                // TODO: Fix when TiledMap is an Asset - TiledMap structure may have changed
                // For now, return None as the API needs to be updated
                let asset = self.tiled_maps.get(handle)?;
                asset
                    .map
                    .get_layer(layer_index.unwrap_or(0))
                    .and_then(|layer| {
                        let tile_size = UVec2::new(asset.map.tile_width, asset.map.tile_width);
                        match layer.layer_type() {
                            tiled::LayerType::Tiles(tile_layer) => tile_layer
                                .get_tile(pos.x as i32, pos.y as i32)
                                .map(|layer_tile| layer_tile.id() as usize),
                            tiled::LayerType::Objects(object_layer) => {
                                let mut result = None;
                                let posf = pos * tile_size.as_vec2();
                                for object in object_layer.objects() {
                                    if shape_contains(&object, tile_size, posf) {
                                        result =
                                            object.properties.get("p8flags").and_then(|value| {
                                                match value {
                                                    PropertyValue::IntValue(i) => Some(*i as usize),
                                                    _ => None,
                                                }
                                            });
                                        break;
                                    }
                                }
                                result
                            }
                            _ => None,
                        }
                    })
            }
            level::Tiled::World { handle: _ } => {
                todo!()
            }
        }
    }

    pub fn mgetp(
        &self,
        map: &level::Tiled,
        _prop_by: pico8::PropBy,
        _map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Option<tiled::Properties> {
        match map {
            level::Tiled::SpriteMap { handle: _handle } => {
                // TODO: Fix when TiledMap is an Asset - TiledMap structure may have changed
                // For now, return None as the API needs to be updated
                None
                /* OLD CODE - needs API update
                let tile_size = UVec2::new(handle.map.tile_width, handle.map.tile_width);
                handle
                    .map
                    .get_layer(layer_index.unwrap_or(0))
                    .and_then(|layer| match layer.layer_type() {
                            tiled::LayerType::Tiles(tile_layer) => match prop_by {
                                PropBy::Pos(pos) => tile_layer
                                    .get_tile(pos.x as i32, pos.y as i32)
                                    .and_then(|layer_tile| {
                                        layer_tile.get_tile().map(|tile| tile.properties.clone())
                                    }),
                                PropBy::Name(name) => {
                                    warn!("Cannot look up by name {name:?} on a tile layer.");
                                    None
                                }
                                PropBy::Rect(_) => {
                                    warn!("Cannot look up by rect");
                                    None
                                }
                            },
                            tiled::LayerType::Objects(object_layer) => match prop_by {
                                PropBy::Pos(pos) => {
                                    let posf = pos * tile_size.as_vec2();
                                    for object in object_layer.objects() {
                                        if shape_contains(&object, tile_size, posf) {
                                            let mut properties = object.properties.clone();

                                            insert_object_fields(&mut properties, &object);
                                            return Some(properties);
                                        }
                                    }
                                    None
                                }
                                PropBy::Rect(rect) => {
                                    for object in object_layer.objects() {
                                        if shape_intersects(&object, tile_size, rect) {
                                            let mut properties = object.properties.clone();

                                            insert_object_fields(&mut properties, &object);
                                            return Some(properties);
                                        }
                                    }
                                    None
                                }
                                PropBy::Name(name) => {
                                    for object in object_layer.objects() {
                                        if object.name == name {
                                            let mut properties = object.properties.clone();
                                            insert_object_fields(&mut properties, &object);
                                            return Some(properties);
                                        }
                                    }
                                    None
                                }
                            },
                            _ => None,
                        })
                */
            }
            level::Tiled::World { handle: _ } => {
                // todo!()
                None
            }
        }
    }

    pub fn mset(
        &mut self,
        map: &level::Tiled,
        _pos: Vec2,
        _sprite_index: usize,
        _map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<(), pico8::Error> {
        match map {
            level::Tiled::SpriteMap {
                handle: _map_handle,
            } => {
                // TODO: Fix when TiledMap is an Asset and tiled_id_storage is available
                // For now, return error as the API needs to be updated
                Err(pico8::Error::Unsupported(
                    "TiledMap API needs update".into(),
                ))
                /* OLD CODE - needs API update
                let tile_size =
                    UVec2::new(map_handle.map.tile_width, map_handle.map.tile_width);
                map_handle
                    .map
                    .get_layer(layer_index.unwrap_or(0))
                    .ok_or(pico8::Error::NoSuch("layer".into()))
                    .and_then(|layer| {
                        match layer.layer_type() {
                            tiled::LayerType::Tiles(_tile_layer) => {
                                // TODO: Implement tile setting
                                Ok(())
                            }
                            tiled::LayerType::Objects(_object_layer) => {
                                // TODO: Fix when tiled_id_storage is available
                                Err(pico8::Error::Unsupported(
                                    "setting object layers requires tiled_id_storage".into(),
                                ))
                            }
                            _ => Err(pico8::Error::Unsupported(
                                "setting tile and object layers in map".into(),
                            )),
                        }
                    })
                */
            }
            level::Tiled::World { handle: _ } => {
                todo!()
            }
        }
    }

    // Return the properties for an entity that has a `TiledLookup` component.
    pub fn props(&self, id: Entity) -> Result<tiled::Properties, pico8::Error> {
        let tiled_lookup = self
            .tiled_lookups
            .get(id)
            .map_err(|_| pico8::Error::NoSuch("TiledLookup".into()))?;
        match tiled_lookup {
            TiledLookup::Object {
                layer: _layer,
                idx: _idx,
                handle: _handle,
            } => {
                // TODO: Fix when TiledMap is an Asset - TiledMap structure may have changed
                // For now, return error as the API needs to be updated
                Err(pico8::Error::Unsupported(
                    "TiledMap API needs update".into(),
                ))
                /* OLD CODE - needs API update
                let layer = handle
                    .map
                    .get_layer(*layer as usize)
                    .ok_or(pico8::Error::NoSuch("layer".into()))?;
                let object_layer = layer
                    .as_object_layer()
                    .ok_or(pico8::Error::NoSuch("layer as object layer".into()))?;
                let object = object_layer
                    .get_object(*idx as usize)
                    .ok_or(pico8::Error::NoSuch("object".into()))?;
                let mut properties = object.properties.clone();
                insert_object_fields(&mut properties, &object);
                Ok(properties)
                */
            } // _ => unreachable!(),
        }
    }
}

fn shape_contains(object: &tiled::ObjectData, tile_size: UVec2, point: Vec2) -> bool {
    match object.shape {
        tiled::ObjectShape::Rect { width, height } => {
            Rect::new(object.x, object.y, object.x + width, object.y + height).contains(point)
        }
        tiled::ObjectShape::Point(_x, _y) => !Rect::new(
            object.x,
            object.y - tile_size.y as f32,
            object.x + tile_size.x as f32,
            object.y,
        )
        .contains(point),
        ref x => {
            todo!("{:?}", x)
            // Rect::new(object.x,
            //           object.y - tile_size.y as f32,
            //           object.x + tile_size.x as f32,
            //           object.y).contains(point)
        }
    }
}

fn shape_intersects(object: &tiled::ObjectData, tile_size: UVec2, rect: Rect) -> bool {
    match object.shape {
        tiled::ObjectShape::Rect { width, height } => {
            !Rect::new(object.x, object.y, object.x + width, object.y + height)
                .intersect(rect)
                .is_empty()
        }
        tiled::ObjectShape::Point(_x, _y) => !Rect::new(
            object.x,
            object.y - tile_size.y as f32,
            object.x + tile_size.x as f32,
            object.y,
        )
        .intersect(rect)
        .is_empty(),
        // _ => {
        ref x => {
            todo!("{:?}", x)
        }
    }
}

fn insert_object_fields(properties: &mut tiled::Properties, object: &tiled::Object) {
    properties.insert("x".to_owned(), tiled::PropertyValue::FloatValue(object.x));
    properties.insert("y".to_owned(), tiled::PropertyValue::FloatValue(object.y));
    // match object.shape {
    //     tiled::ObjectShape::Rect { width, height } => {
    //         properties.insert("width".to_owned(), tiled::PropertyValue::FloatValue(width));
    //         properties.insert(
    //             "height".to_owned(),
    //             tiled::PropertyValue::FloatValue(height),
    //         );
    //     }
    //     _ => {}
    // }
    properties.insert(
        "class".to_owned(),
        tiled::PropertyValue::StringValue(object.user_type.clone()),
    );

    properties.insert(
        "name".to_owned(),
        tiled::PropertyValue::StringValue(object.name.clone()),
    );
    properties.insert(
        "tile".to_owned(),
        tiled::PropertyValue::BoolValue(object.get_tile().is_some()),
    );
}

pub(crate) fn layout_from_tileset(tileset: &Tileset) -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(
        UVec2::new(tileset.tile_width, tileset.tile_height),
        tileset.columns,
        tileset.tilecount / tileset.columns,
        (tileset.spacing != 0).then_some(UVec2::new(tileset.spacing, tileset.spacing)),
        (tileset.offset_x != 0 || tileset.offset_y != 0)
            .then_some(UVec2::new(tileset.offset_x as u32, tileset.offset_y as u32)),
    )
}

// pub(crate) fn layer_tile_properties(tile: &tiled::LayerTile) -> Option<tiled::Properties> {
//     tile.get_tile().map(|t| t.properties.clone())
// }

pub(crate) fn flags_from_tileset(tileset: &Tileset) -> Vec<u8> {
    let mut flags: Vec<u8> = vec![0; tileset.tilecount as usize];
    for (id, tile) in tileset.tiles() {
        flags[id as usize] = tile
            .properties
            .get("p8flags")
            .map(|value| match value {
                tiled::PropertyValue::IntValue(x) => *x as u8,
                v => panic!("Expected integer value not {v:?}"),
            })
            .unwrap_or(0);
    }
    flags
}
