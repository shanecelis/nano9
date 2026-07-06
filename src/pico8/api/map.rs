use super::*;
use crate::translate::Position;
use bevy::platform::hash::FixedHasher;
// use bevy_ecs_tilemap::prelude::*;
use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

bobtail::define! {
    #[doc(hidden)]
    pub __map => fn map(
        &mut self,
        map_pos: UVec2,
        screen_start: Vec2,
        size: UVec2,
        #[tail]
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error>;
    #[doc(hidden)]
    pub __mget => fn mget(
        &self,
        pos: Vec2,
        #[tail]
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<usize, Error>;
    #[doc(hidden)]
    pub __mset => fn mset(
        &mut self,
        pos: Vec2,
        sprite_index: usize,
        #[tail]
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<(), Error>;
}
pub use __map as map;
pub use __mget as mget;
pub use __mset as mset;

impl super::Pico8<'_, '_> {
    fn sprite_map(&self, map_index: Option<usize>) -> Result<&SpriteMap, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset()?
            .maps
            .get(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    // fn sprite_map_mut(&mut self, map_index: Option<usize>) -> Result<&mut SpriteMap, Error> {
    //     let index = map_index.unwrap_or(0);
    //     self.pico8_asset_mut()?
    //         .maps
    //         .get_mut(index)
    //         .ok_or(Error::NoSuch(format!("map index {index}").into()))
    // }

    pub fn map(
        &mut self,
        map_pos: UVec2,
        mut screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error> {
        trace!("map");
        let map_index = map_index.unwrap_or(0);
        screen_start = self.state.draw_state.apply_camera_delta(screen_start);
        if cfg!(feature = "negate-y") {
            screen_start.y = -screen_start.y;
        }
        let hash = {
            let mut hasher = FixedHasher.build_hasher();
            "map".hash(&mut hasher);
            map_pos.hash(&mut hasher);
            self.state.palette.hash(&mut hasher);
            self.state.pal_map.hash(&mut hasher);
            size.hash(&mut hasher);
            mask.inspect(|m| m.hash(&mut hasher));
            map_index.hash(&mut hasher);
            hasher.finish()
        };
        let rect = URect {
            min: map_pos,
            max: map_pos + size,
        };
        self.state.draw_state.mark_drawn();
        // See if there's already an entity available.
        if let Some(id) = self.resurrect(hash, screen_start) {
            // trace!("Resurrect map with hash {hash}");
            return Ok(id);
        }
        match self.sprite_map(Some(map_index))?.clone() {
            SpriteMap::P8(_map) => {
                // trace!("Create map with hash {hash}");

                // let map_size = TilemapSize::from(size);
                let clearable = Clearable::new(self.defaults.time_to_live).with_hash(hash);
                // let tile_storage = TileStorage::empty(map_size);
                let gfx_material = self.gfx_material().clone();
                let sprite_sheet = self
                    .pico8_asset()?
                    .sprite_sheets
                    .get(self.state.sprite_sheet_index)
                    .ok_or(Error::NoSuch("Sprite sheet for map".into()))?;

                let map_entity = self
                    .commands
                    .spawn((
                        Name::new("map"),
                        Position::from(screen_start),
                        Visibility::Inherited,
                        clearable,
                        P8SpriteMap {
                            map_index,
                            sprite_sheet: sprite_sheet.clone(),
                            gfx_material,
                            rect,
                            mask,
                        },
                    ))
                    .id();
                Ok(map_entity)
            }
            #[cfg(feature = "level")]
            SpriteMap::Level(map) => Ok(map.map(screen_start, 0, hash, &mut self.commands)),
        }
    }

    pub fn mget(
        &self,
        pos: Vec2,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<usize, Error> {
        let map: &SpriteMap = self.sprite_map(map_index)?;
        match *map {
            SpriteMap::P8(ref handle) => {
                let map = self
                    .p8_maps
                    .get(handle)
                    .ok_or_else(|| Error::NoSuch("map for handle".into()))?;
                let i = (pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize;
                Ok(*map.get(i).ok_or_else(|| {
                    Error::NoSuch(
                        format!(
                            "map index {i} with length {} {}",
                            map.len(),
                            if i > 0x1000 {
                                "; consider using the '--shared-data=map' argument."
                            } else {
                                ""
                            }
                        )
                        .into(),
                    )
                })? as usize)
            }
            #[cfg(feature = "level")]
            SpriteMap::Level(ref map) => self
                .tiled
                .mget(map, pos, map_index, _layer_index)
                .ok_or_else(|| Error::NoSuch("tile".into())),
        }
    }

    pub fn mset(
        &mut self,
        pos: Vec2,
        sprite_index: usize,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<(), Error> {
        let map = self.sprite_map(map_index)?.clone();
        match map {
            SpriteMap::P8(handle) => {
                let mut map = self
                    .p8_maps
                    .get_mut(&handle)
                    .ok_or_else(|| Error::NoSuch("map for handle".into()))?;
                map.get_mut((pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize)
                    .map(|value| *value = sprite_index as u8)
                    .ok_or_else(|| Error::NoSuch("map entry".into()))
            }
            #[cfg(feature = "level")]
            SpriteMap::Level(_map) => {
                todo!()
                // self.tiled
                //     .mset(map, pos, sprite_index, map_index, layer_index)
            }
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{DropPolicy, N9Entity, pico8::lua::with_pico8};

    use bevy_mod_scripting::bindings::{
        InteropError, ScriptValue,
        function::{
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "mget",
                |ctx: FunctionCallContext,
                 x: f32,
                 y: f32,
                 map_index: Option<usize>,
                 layer_index: Option<usize>| {
                    with_pico8(&ctx, move |pico8| {
                        pico8.mget(Vec2::new(x, y), map_index, layer_index)
                    })
                },
            )
            .register(
                "mset",
                |ctx: FunctionCallContext,
                 x: f32,
                 y: f32,
                 v: usize,
                 map_index: Option<usize>,
                 layer_index: Option<usize>| {
                    with_pico8(&ctx, move |pico8| {
                        pico8.mset(Vec2::new(x, y), v, map_index, layer_index)
                    })
                },
            )
            // map( celx, cely, sx, sy, celw, celh, [layer] )
            .register(
                "map",
                |ctx: FunctionCallContext,
                 celx: Option<u32>,
                 cely: Option<u32>,
                 sx: Option<f32>,
                 sy: Option<f32>,
                 celw: Option<u32>,
                 celh: Option<u32>,
                 layer: Option<u8>,
                 map_index: Option<usize>|
                 -> Result<ScriptValue, InteropError> {
                    let id = with_pico8(&ctx, move |pico8| {
                        pico8.map(
                            UVec2::new(celx.unwrap_or(0), cely.unwrap_or(0)),
                            Vec2::new(sx.unwrap_or(0.0), sy.unwrap_or(0.0)),
                            UVec2::new(celw.unwrap_or(128), celh.unwrap_or(128)),
                            layer,
                            map_index,
                        )
                    })?;

                    let entity = N9Entity {
                        entity: id,
                        drop: DropPolicy::Nothing,
                    };
                    let world = ctx.world()?;
                    entity.into_script_ref(world)
                },
            );
    }
}
