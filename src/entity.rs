use bevy::prelude::*;

use crate::{
    translate::{Position, Rotation},
    pico8::{Clearable, Pico8State},
};
use bevy_mod_scripting::{
    bindings::function::{
        from::Val, namespace::NamespaceBuilder, script_function::FunctionCallContext,
    },
    lua::mlua::{self, FromLua, Lua, UserData, Value},
};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    bindings::{InteropError, IntoScriptRef, ReflectReference, WorldAccessGuard},
    prelude::ScriptValue,
};

#[derive(Debug, Clone, Copy, Reflect)]
pub enum DropPolicy {
    Nothing,
    Despawn,
}

impl UserData for DropPolicy {}

impl FromLua for DropPolicy {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl Drop for N9Entity {
    fn drop(&mut self) {
        if matches!(self.drop, DropPolicy::Despawn) {
            warn!("Retained entity leaked {:?}.", self.entity);
        }
    }
}

#[derive(Clone, Reflect)]
pub struct N9Entity {
    pub entity: Entity,
    pub drop: DropPolicy,
}

impl N9Entity {
    #[cfg(feature = "scripting")]
    pub fn into_script_ref(self, world: WorldAccessGuard) -> Result<ScriptValue, InteropError> {
        let reference = {
            let allocator = world.allocator();
            let mut allocator = allocator.write();
            ReflectReference::new_allocated(self, &mut allocator)
        };
        ReflectReference::into_script_ref(reference, world)
    }
}

pub(crate) fn plugin(app: &mut App) {
    NamespaceBuilder::<N9Entity>::new(app.world_mut())
        .register(
            "retain",
            |ctx: FunctionCallContext, this: Val<N9Entity>, z: Option<f32>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    let mut commands = world.commands();
                    commands.entity(this.entity).remove::<Clearable>();
                    // if let Some(mut transform) = world.get_mut::<Transform>(this.entity) {
                    //     transform.translation.z = z.unwrap_or(0.0);
                    // }
                })?;
                Ok(this)
            },
        )
        .register(
            "pos",
            |ctx: FunctionCallContext,
             this: Val<N9Entity>,
             x: Option<f32>,
             y: Option<f32>,
             // z: Option<f32>
                | {
                let world = ctx.world()?;
                let pos = world.with_global_access(|world| {
                    let camera_position_delta = world
                        .get_resource::<Pico8State>()
                        .and_then(|state| state.draw_state.camera_position_delta);
                    if x.is_some() || y.is_some() { // || z.is_some() {
                        world
                            .get_mut::<Position>(this.entity)
                            .map(|mut position| {
                                let last = position.0;
                                if let Some(x) = x {
                                    position.0.x = x;
                                }
                                if let Some(y) = y {
                                    position.0.y = y;
                                }
                                // if let Some(z) = z {
                                //     transform.translation.z = z;
                                // }
                                last
                            })
                    } else {
                        world
                            .get::<Position>(this.entity)
                            .map(|position| position.0)
                    }
                })?;
                if let Some(pos) = pos {
                    Ok(Some(vec![pos.x, pos.y]))
                } else {
                    Ok(None)
                }
            },
        )
        .register(
            "rot",
            |ctx: FunctionCallContext,
             this: Val<N9Entity>,
             z: Option<f32>,
             y: Option<f32>,
             x: Option<f32>| {
                use std::f32::consts::PI;
                let world = ctx.world()?;
                let rot = world.with_global_access(|world| {
                    if x.is_some() || y.is_some() || z.is_some() {
                        world
                            .get_mut::<Rotation>(this.entity)
                            .map(|mut rotation| {
                                let last = rotation.0;
                                // let last = transform.rotation.to_euler(EulerRot::ZYX);
                                let mut turns = last;
                                if let Some(z) = z {
                                    turns.z = z;
                                }
                                if let Some(y) = y {
                                    turns.y = y;
                                }
                                if let Some(x) = x {
                                    turns.x = x;
                                }
                                rotation.0 = turns;
                                // transform.rotation =
                                //     Quat::from_euler(EulerRot::ZYX, turns.0, turns.1, turns.2);
                                last
                            })
                    } else {
                        world
                            .get::<Rotation>(this.entity)
                            .map(|rotation| rotation.0)
                    }
                })?;
                if let Some(rot) = rot {
                    Ok(Some(vec![rot.z, rot.y, rot.x]))
                } else {
                    Ok(None)
                }
            },
        )
        .register(
            "name",
            |ctx: FunctionCallContext, this: Val<N9Entity>, new_name: Option<String>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    if let Some(name) = new_name {
                        let mut commands = world.commands();
                        commands.entity(this.entity).insert(Name::new(name));
                        None
                    } else {
                        world
                            .get::<Name>(this.entity)
                            .map(|n| n.as_str().to_string())
                    }
                })
            },
        )
        .register(
            "vis",
            |ctx: FunctionCallContext, this: Val<N9Entity>, vis: Option<bool>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    if let Some(vis) = vis {
                        if let Some(mut visible) = world.get_mut::<Visibility>(this.entity) {
                            *visible = match vis {
                                // None => Visibility::Inherited,
                                true => Visibility::Visible,
                                false => Visibility::Hidden,
                            };
                        }
                        None
                    } else {
                        world
                            .get::<Visibility>(this.entity)
                            .map(|v| !matches!(v, Visibility::Hidden))
                    }
                })
            },
        )
        .register(
            "despawn",
            |ctx: FunctionCallContext, this: Val<N9Entity>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    let mut commands = world.commands();
                    commands.entity(this.entity).despawn();
                })?;
                Ok(())
            },
        );
}
