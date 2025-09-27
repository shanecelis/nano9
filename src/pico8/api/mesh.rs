use bevy::prelude::*;
use crate::pico8::Error;

#[derive(Debug, Clone, Reflect)]
pub enum MeshHandle {
    Mesh(Handle<Mesh>),
    Vox(Handle<bevy_vox_scene::VoxelModel>),
}

pub(crate) fn plugin(app: &mut App) {
    app
        .add_plugins(bevy_vox_scene::VoxScenePlugin::default());

    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    // mesh(n, [x,] [y,] [z,])
    fn mesh(&mut self, n: usize, pos: Vec3) -> Result<Entity, Error> {
        let mesh_handle = self.pico8_asset()?.meshes.get(n).ok_or(Error::NoSuch("mesh".into()))?.clone();
        match mesh_handle {
            MeshHandle::Mesh(mesh) => {
                let id = self.commands.spawn_empty().id();
                self.commands.queue(move |world: &mut World| {
                    let material = {
                        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                        materials.add(Color::srgb(0.8, 0.7, 0.6))
                    };
                    world.entity_mut(id)
                        .insert((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(pos),
                    ));
                });
                Ok(id)
            }
            _ => todo!()
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{pico8::lua::with_pico8, DropPolicy, N9Entity};

    use bevy_mod_scripting::core::bindings::{
        function::{
            from::FromScript,
            into_ref::IntoScriptRef,
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        script_value::ScriptValue,
        ReflectReference,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            // mesh(n, [x,] [y,] [z,])
            .register(
                "mesh",
                |ctx: FunctionCallContext,
                 n: usize,
                 x: Option<f32>,
                 y: Option<f32>,
                 z: Option<f32>,
                | {
                    let pos = Vec3::new(x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0));
                    let id = with_pico8(&ctx, move |pico8| pico8.mesh(n, pos))?;
                    Ok(())
                },
            );
    }
}
