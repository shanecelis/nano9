use crate::{
    level,
    pico8::{self, PropBy},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_ecs_tiled::{
    map::components::TiledMapStorage,
    prelude::{TiledMap, TiledMapHandle},
};
use tiled::{PropertyValue, Tileset};

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<TiledLookup>();
}

#[derive(Debug, Component, Reflect)]
pub enum TiledLookup {
    Object {
        layer: u32,
        idx: u32,
        handle: Handle<TiledMap>,
    },
}

#[derive(SystemParam)]
pub struct Level<'w, 's> {
    tiled_maps: ResMut<'w, Assets<bevy_ecs_tiled::prelude::TiledMap>>,
    // tiled_worlds: ResMut<'w, Assets<bevy_ecs_tiled::prelude::TiledWorld>>,
    tiled_id_storage: Query<'w, 's, (&'static TiledMapStorage, &'static TiledMapHandle)>,
    sprites: Query<'w, 's, &'static mut Sprite>,
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
                self.tiled_maps.get(handle).and_then(|tiled_map| {
                    tiled_map
                        .map
                        .get_layer(layer_index.unwrap_or(0))
                        .and_then(|layer| {
                            let tile_size =
                                UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                            match layer.layer_type() {
                                tiled::LayerType::Tiles(tile_layer) => tile_layer
                                    .get_tile(pos.x as i32, pos.y as i32)
                                    .map(|layer_tile| layer_tile.id() as usize),
                                tiled::LayerType::Objects(object_layer) => {
                                    let mut result = None;
                                    let posf = pos * tile_size.as_vec2();
                                    for object in object_layer.objects() {
                                        if shape_contains(&object, tile_size, posf) {
                                            result = object.properties.get("p8flags").and_then(
                                                |value| match value {
                                                    PropertyValue::IntValue(i) => Some(*i as usize),
                                                    _ => None,
                                                },
                                            );
                                            break;
                                        }
                                    }
                                    result
                                }
                                _ => None,
                            }
                        })
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
        prop_by: pico8::PropBy,
        _map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<tiled::Properties> {
        match map {
            level::Tiled::SpriteMap { handle } => {
                self.tiled_maps.get(handle).and_then(|tiled_map| {
                    let tile_size = UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                    tiled_map
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
                })
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
        pos: Vec2,
        sprite_index: usize,
        _map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Result<(), pico8::Error> {
        match map {
            level::Tiled::SpriteMap { handle: map_handle } => {
                self.tiled_maps
                    .get(map_handle)
                    .ok_or(pico8::Error::NoSuch("map".into()))
                    .and_then(|tiled_map| {
                        let tile_size =
                            UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                        tiled_map
                            .map
                            .get_layer(layer_index.unwrap_or(0))
                            .ok_or(pico8::Error::NoSuch("layer".into()))
                            .and_then(|layer| {
                                match layer.layer_type() {
                                    tiled::LayerType::Tiles(_tile_layer) => {
                                        // tile_layer.get_tile(pos.x as i32, pos.y as i32)
                                        //           .and_then(|layer_tile| layer_tile.get_tile().map(|tile| tile.properties.clone()))
                                        Ok(())
                                    }
                                    tiled::LayerType::Objects(object_layer) => {
                                        let posf = pos * tile_size.as_vec2();
                                        for object in object_layer.objects() {
                                            if shape_contains(&object, tile_size, posf) {
                                                let mut sprite_id = None;
                                                for (tiled_id_storage, handle) in
                                                    &self.tiled_id_storage
                                                {
                                                    if *map_handle == handle.0 {
                                                        // This is probably the one.
                                                        if let Some(id) = tiled_id_storage
                                                            .objects
                                                            .get(&object.id())
                                                        {
                                                            sprite_id = Some(id);
                                                        }
                                                    }
                                                }
                                                return if let Some(id) = sprite_id {
                                                    self.sprites
                                                        .get_mut(*id)
                                                        .map_err(|_| {
                                                            pico8::Error::NoSuch(
                                                                "object sprite".into(),
                                                            )
                                                        })
                                                        .and_then(|mut sprite| {
                                                            if let Some(atlas) =
                                                                &mut sprite.texture_atlas
                                                            {
                                                                atlas.index = sprite_index;
                                                                Ok(())
                                                            } else {
                                                                Err(pico8::Error::NoSuch(
                                                                    "sprite atlas".into(),
                                                                ))
                                                            }
                                                        })
                                                } else {
                                                    Err(pico8::Error::NoSuch(
                                                        "sprite entity".into(),
                                                    ))
                                                };
                                            }
                                        }
                                        Err(pico8::Error::NoSuch("tile".into()))
                                    }
                                    _ => Err(pico8::Error::Unsupported(
                                        "setting tile and object layers in map".into(),
                                    )),
                                }
                            })
                    })
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
            TiledLookup::Object { layer, idx, handle } => {
                let tiled_map = self
                    .tiled_maps
                    .get(handle)
                    .ok_or(pico8::Error::NoSuch("TiledMap".into()))?;
                let layer = tiled_map
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
