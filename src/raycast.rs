use bevy::{
    ecs::system::SystemParam,
    math::bounding::{Aabb2d, AabbCast2d, IntersectsVolume, RayCast2d},
    prelude::*,
};

#[cfg(feature = "scripting")]
use crate::pico8::lua::with_system_param;
use crate::pico8::{negate_y, Error};
use bevy_mod_scripting::bindings::InteropError;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::bindings::{
    function::{
        from::FromScript,
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    },
    script_value::ScriptValue,
};

#[cfg(feature = "scripting")]
use crate::conversions::RectValue;

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Place>().register_type::<Cover>();

        // XXX: cfg!(feature = "scripting")
        let world = app.world_mut();
        #[cfg(feature = "scripting")]
        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "raydown",
                |ctx: FunctionCallContext,
                 x: f32,
                 y: f32,
                 mask: Option<u32>,
                 shape: Option<ScriptValue>| {
                    let pos = Vec2::new(x, y);
                    let shape = if let Some(v) = shape {
                        let Rect { min, max } = RectValue::from_script(v, ctx.world()?)?;
                        Some(Aabb2d { min, max })
                    } else {
                        None
                    };
                    with_rays(&ctx, move |pico8| {
                        // let ids: Vec<u64> = pico8
                        //    .ray(pos, dir, mask)
                        //    .into_iter()
                        //    .map(|id| id.to_bits()).collect();
                        let ids: Vec<i64> = pico8
                            .raydown(pos, mask, shape)
                            .into_iter()
                            .map(|id| id.to_bits() as i64)
                            .collect();
                        Ok(ids)
                    })
                },
            )
            .register(
                "raycast",
                |ctx: FunctionCallContext,
                 x: f32,
                 y: f32,
                 dx: f32,
                 dy: f32,
                 mask: Option<u32>,
                 shape: Option<ScriptValue>| {
                    let pos = Vec2::new(x, y);
                    let dxdy = Vec2::new(dx, dy);
                    let shape = if let Some(v) = shape {
                        let Rect { min, max } = RectValue::from_script(v, ctx.world()?)?;
                        Some(Aabb2d { min, max })
                    } else {
                        None
                    };
                    with_rays(&ctx, move |pico8| {
                        // let dir = Dir2::new(dxdy).map_err(|_| Error::InvalidArgument("dx, dy direction".into()))?;
                        let Ok(dir) = Dir2::new(dxdy) else {
                            return Ok(ScriptValue::Unit);
                        };
                        let ids_dists: Vec<ScriptValue> = pico8
                            .raycast(pos, dir, mask, shape)
                            .into_iter()
                            .flat_map(|(id, dist)| {
                                [
                                    ScriptValue::Integer(id.to_bits() as i64),
                                    ScriptValue::Float(dist as f64),
                                ]
                            })
                            .collect();
                        Ok(ScriptValue::List(ids_dists))
                    })
                },
            )
            .register("place", |ctx: FunctionCallContext, name: String| {
                with_rays(&ctx, move |pico8| {
                    Ok(pico8.place(&name).map(|v| vec![v.x, v.y]))
                })
            });
    }
}
/// A `ray`-able object.
#[derive(Debug, Component, Reflect)]
pub struct Cover {
    pub aabb: Aabb2d,
    pub flags: u32,
}

#[derive(Debug, Component, Reflect)]
pub struct Place(pub String);

#[derive(SystemParam)]
pub struct Rays<'w, 's> {
    covers: Query<'w, 's, (Entity, &'static Cover, &'static GlobalTransform)>,
    places: Query<'w, 's, (&'static Place, &'static GlobalTransform)>,
}

#[cfg(feature = "scripting")]
fn with_rays<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Rays) -> Result<X, Error>,
) -> Result<X, InteropError> {
    with_system_param::<Rays, X, Error>(ctx, f)
}

impl Rays<'_, '_> {
    pub fn raydown(&self, mut pos: Vec2, mask: Option<u32>, shape: Option<Aabb2d>) -> Vec<Entity> {
        pos.y = negate_y(pos.y);
        self.covers
            .iter()
            .filter_map(|(id, cover, transform)| {
                if let Some(mask) = mask {
                    if cover.flags & mask == 0 {
                        return None;
                    }
                }
                let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                // let min = cover.aabb.min;
                if let Some(mut shape) = shape {
                    shape.min += pos;
                    shape.max += pos;
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    // dbg!(id);
                    shape.intersects(&other).then_some(id)
                } else {
                    (min.x <= pos.x && min.y <= pos.y && {
                        let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                        // let max = cover.aabb.max;
                        max.x > pos.x && max.y > pos.y
                    })
                    .then_some(id)
                }
            })
            .collect()
    }

    // Cast a "ray" either at pos or from pos in direction dir.
    pub fn raycast(
        &self,
        mut pos: Vec2,
        mut dir: Dir2,
        mask: Option<u32>,
        shape: Option<Aabb2d>,
    ) -> Vec<(Entity, f32)> {
        let mut v = dir.as_vec2();
        if cfg!(feature = "negate-y") {
            v.y = -v.y;
            pos.y = -pos.y;
        }
        // dir = Dir2::new_unchecked(v);
        dir = Dir2::new(v).unwrap();
        if let Some(shape) = shape {
            let aabb_cast = AabbCast2d::new(shape, pos, dir, f32::MAX);
            self.covers
                .iter()
                .filter_map(|(id, cover, transform)| {
                    if let Some(mask) = mask {
                        if cover.flags & mask == 0 {
                            return None;
                        }
                    }
                    let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    aabb_cast
                        .aabb_collision_at(other)
                        .map(|distance| (id, distance))
                })
                .collect()
        } else {
            let ray_cast = RayCast2d::new(pos, dir, f32::MAX);
            self.covers
                .iter()
                .filter_map(|(id, cover, transform)| {
                    if let Some(mask) = mask {
                        if cover.flags & mask == 0 {
                            return None;
                        }
                    }
                    let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    ray_cast.aabb_intersection_at(&other).map(|distance| {
                        // dbg!(&other);
                        // dbg!(&ray_cast);
                        dbg!((id, distance))
                    })
                })
                .collect()
        }
    }

    pub fn place(&self, name: &str) -> Option<Vec2> {
        for (place, transform) in &self.places {
            if place.0 == name {
                let mut r = transform.translation().xy();
                if cfg!(feature = "negate-y") {
                    r.y = -r.y;
                }
                return Some(r);
            }
        }
        None
    }
}
