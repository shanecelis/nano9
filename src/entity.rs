use bevy::prelude::*;
use bevy::reflect::TypeRegistry;

use crate::{
    pico8::Clearable,
    translate::{Position, Rotation},
};
use bevy_mod_scripting::bindings::function::{
    namespace::NamespaceBuilder, script_function::FunctionCallContext,
};
use bevy_mod_scripting::lua::mlua::{self, FromLua, Lua, UserData, Value};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    ArgMeta, FromScript, GetTypeDependencies, IntoScript,
    bindings::{
        InteropError, ReflectReference, ScriptValue, WorldAccessGuard, WorldExtensions,
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        function::into_ref::IntoScriptRef,
    },
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
#[cfg_attr(
    feature = "scripting",
    derive(FromScript, IntoScript, GetTypeDependencies, ArgMeta)
)]
pub struct N9Entity {
    pub entity: Entity,
    pub drop: DropPolicy,
}

#[cfg(feature = "scripting")]
impl TypedThrough for N9Entity {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<Self as bevy::reflect::Typed>::type_info())
    }
}

impl N9Entity {
    #[cfg(feature = "scripting")]
    pub fn into_script_ref(self, world: WorldAccessGuard) -> Result<ScriptValue, InteropError> {
        let reference = {
            let allocator = world.allocator();
            let mut allocator = allocator.write();
            ReflectReference::new_allocated(self, &mut allocator)
        };
        <ReflectReference as IntoScriptRef>::into_script_ref(reference, world)
    }
}

pub(crate) fn plugin(app: &mut App) {
    NamespaceBuilder::<N9Entity>::new(app.world_mut())
        .register(
            "retain",
            |ctx: FunctionCallContext, this: N9Entity, _z: Option<f32>| {
                let world = ctx.world()?;
                world.with_world_mut_access_and_then(|world| -> Result<(), InteropError> {
                    let mut commands = world.commands();
                    commands.entity(this.entity).remove::<Clearable>();
                    Ok(())
                })?;
                Ok(this)
            },
        )
        .register(
            "pos",
            |ctx: FunctionCallContext,
             this: N9Entity,
             x: Option<f32>,
             y: Option<f32>| {
                let world = ctx.world()?;
                let pos = world.with_world_mut_access_and_then(|world| -> Result<_, InteropError> {
                    if x.is_some() || y.is_some() {
                        Ok(world
                            .get_mut::<Position>(this.entity)
                            .map(|mut position| {
                                let last = position.0;
                                if let Some(x) = x {
                                    position.0.x = x;
                                }
                                if let Some(y) = y {
                                    position.0.y = y;
                                }
                                last
                            }))
                    } else {
                        Ok(world
                            .get::<Position>(this.entity)
                            .map(|position| position.0))
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
             this: N9Entity,
             z: Option<f32>,
             y: Option<f32>,
             x: Option<f32>| {
                let world = ctx.world()?;
                let rot = world.with_world_mut_access_and_then(|world| -> Result<_, InteropError> {
                    if x.is_some() || y.is_some() || z.is_some() {
                        Ok(world.get_mut::<Rotation>(this.entity).map(|mut rotation| {
                            let last = rotation.0;
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
                            last
                        }))
                    } else {
                        Ok(world
                            .get::<Rotation>(this.entity)
                            .map(|rotation| rotation.0))
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
            |ctx: FunctionCallContext, this: N9Entity, new_name: Option<String>| {
                let world = ctx.world()?;
                world.with_world_mut_access_and_then(|world| -> Result<_, InteropError> {
                    if let Some(name) = new_name {
                        let mut commands = world.commands();
                        commands.entity(this.entity).insert(Name::new(name));
                        Ok(None)
                    } else {
                        Ok(world
                            .get::<Name>(this.entity)
                            .map(|n| n.as_str().to_string()))
                    }
                })
            },
        )
        .register(
            "vis",
            |ctx: FunctionCallContext, this: N9Entity, vis: Option<bool>| {
                let world = ctx.world()?;
                world.with_world_mut_access_and_then(|world| -> Result<_, InteropError> {
                    if let Some(vis) = vis {
                        if let Some(mut visible) = world.get_mut::<Visibility>(this.entity) {
                            *visible = match vis {
                                true => Visibility::Visible,
                                false => Visibility::Hidden,
                            };
                        }
                        Ok(None)
                    } else {
                        Ok(world
                            .get::<Visibility>(this.entity)
                            .map(|v| !matches!(v, Visibility::Hidden)))
                    }
                })
            },
        )
        .register(
            "despawn",
            |ctx: FunctionCallContext, this: N9Entity| {
                let world = ctx.world()?;
                world.with_world_mut_access_and_then(|world| -> Result<(), InteropError> {
                    let mut commands = world.commands();
                    commands.entity(this.entity).despawn();
                    Ok(())
                })?;
                Ok(())
            },
        );
}
