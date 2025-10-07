use crate::pico8::Error;
use bevy::prelude::*;

#[derive(Debug, Clone, Reflect)]
pub enum MeshHandle {
    Mesh(Handle<Mesh>),
    Gltf(Handle<bevy::gltf::Gltf>),
    Vox(Handle<bevy_vox_scene::VoxelModel>),
}

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(bevy_vox_scene::VoxScenePlugin::default());

    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    // mesh(n, [x,] [y,] [z,] [sx,] [sy,] [sz,])
    fn mesh(&mut self, n: usize, pos: Vec3, scale: Vec3) -> Result<Entity, Error> {
        let mesh_handle = self
            .pico8_asset()?
            .meshes
            .get(n)
            .ok_or(Error::NoSuch("mesh".into()))?
            .clone();
        match mesh_handle {
            MeshHandle::Mesh(mesh) => {
                let id = self.commands.spawn_empty().id();
                self.commands.queue(move |world: &mut World| {
                    let material = {
                        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                        materials.add(Color::srgb(0.8, 0.7, 0.6))
                    };
                    world.entity_mut(id).insert((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(pos).with_scale(scale),
                    ));
                });
                Ok(id)
            }
            MeshHandle::Gltf(gltf) => {
                let id = self.commands.spawn_empty().id();
                self.commands.queue(move |world: &mut World| {
                    let scene = {
                        let gltfs = world.resource::<Assets<Gltf>>();
                        let Some(gltf) = gltfs.get(&gltf) else {
                            error!("No gltf for handle {:?}", gltf);
                            return;
                        };
                        gltf.scenes[0].clone()
                    };
                    // let material = {
                    //     let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
                    //     materials.add(Color::srgb(0.8, 0.7, 0.6))
                    // };
                    world.entity_mut(id).insert((
                        SceneRoot(scene),
                        // MeshMaterial3d(material),
                        Transform::from_translation(pos).with_scale(scale),
                    ));
                });
                Ok(id)
            }
            _ => todo!(),
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
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
                 sx: Option<f32>,
                 sy: Option<f32>,
                 sz: Option<f32>| {
                    let pos = Vec3::new(x.unwrap_or(0.0), y.unwrap_or(0.0), z.unwrap_or(0.0));
                    let scale = Vec3::new(sx.unwrap_or(1.0), sy.unwrap_or(1.0), sz.unwrap_or(1.0));
                    let _id = with_pico8(&ctx, move |pico8| pico8.mesh(n, pos, scale))?;
                    Ok(())
                },
            );
    }
}
