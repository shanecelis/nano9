use super::*;
use bevy::{
    utils::hashbrown::hash_map::DefaultHashBuilder,
};
use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}
impl super::Pico8<'_, '_> {
    fn sprite_map(&self, map_index: Option<usize>) -> Result<&Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset()?
            .maps
            .get(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    fn sprite_map_mut(&mut self, map_index: Option<usize>) -> Result<&mut Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset_mut()?
            .maps
            .get_mut(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    pub fn map(
        &mut self,
        map_pos: UVec2,
        mut screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error> {

        screen_start = self.state.draw_state.apply_camera_delta(screen_start);
        if cfg!(feature = "negate-y") {
            screen_start.y = -screen_start.y;
        }
        let hash = { let mut hasher = DefaultHashBuilder::default().build_hasher();
                     "map".hash(&mut hasher);
                     map_pos.hash(&mut hasher);
                     size.hash(&mut hasher);
                     mask.inspect(|m| m.hash(&mut hasher));
                     map_index.inspect(|i| i.hash(&mut hasher));
                     hasher.finish()
        };
        // See if there's already an entity available.
        if let Some(id) = self.resurrect(hash, screen_start) {
            return Ok(id);
        }

        match self.sprite_map(map_index)?.clone() {
            Map::P8(map) => {
                let palette = self.palette(None)?.clone();

                let sprite_sheets = &self.pico8_asset()?.sprite_sheets.clone();
                map.map(
                    map_pos,
                    screen_start,
                    size,
                    mask,
                    sprite_sheets,
                    Some(hash),
                    &mut self.commands,
                    |handle| {
                        self.gfx_handles.get_or_create(
                            &palette,
                            &self.state.pal_map,
                            None,
                            handle,
                            &self.gfxs,
                            &mut self.images,
                        )
                    },
                )
            }
            #[cfg(feature = "level")]
            Map::Level(map) => Ok(map.map(screen_start, 0, &mut self.commands)),
        }
    }

    pub fn mget(
        &self,
        pos: Vec2,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Option<usize> {
        let map: &Map = self.sprite_map(map_index).ok()?;
        match *map {
            Map::P8(ref map) => {
                Some(map[(pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize] as usize)
            }

            #[cfg(feature = "level")]
            Map::Level(ref map) => self.tiled.mget(map, pos, map_index, layer_index),
        }
    }

    pub fn mset(
        &mut self,
        pos: Vec2,
        sprite_index: usize,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<(), Error> {
        let map = self.sprite_map_mut(map_index)?;
        match map {
            Map::P8(ref mut map) => map
                .get_mut((pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize)
                .map(|value| *value = sprite_index as u8)
                .ok_or(Error::NoSuch("map entry".into())),
            #[cfg(feature = "level")]
            Map::Level(ref mut map) => {
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
    use crate::{pico8::lua::with_pico8, DropPolicy, N9Entity};

    use bevy_mod_scripting::core::bindings::{
        function::{
            into_ref::IntoScriptRef,
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        ReflectReference,
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
                        Ok(pico8.mget(Vec2::new(x, y), map_index, layer_index))
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
                 map_index: Option<usize>| {
                    let id = with_pico8(&ctx, move |pico8| {
                        pico8.map(
                            UVec2::new(celx.unwrap_or(0), cely.unwrap_or(0)),
                            Vec2::new(sx.unwrap_or(0.0), sy.unwrap_or(0.0)),
                            UVec2::new(celw.unwrap_or(16), celh.unwrap_or(16)),
                            layer,
                            map_index,
                        )
                    })?;

                    let entity = N9Entity {
                        entity: id,
                        drop: DropPolicy::Nothing,
                    };
                    let world = ctx.world()?;
                    let reference = {
                        let allocator = world.allocator();
                        let mut allocator = allocator.write();
                        ReflectReference::new_allocated(entity, &mut allocator)
                    };
                    ReflectReference::into_script_ref(reference, world)
                },
            );
    }
}
