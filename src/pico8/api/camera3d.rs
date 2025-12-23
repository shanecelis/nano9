use super::*;

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera3d;
// {
//     translation: Option<Vec3>,
//     orientation: Option<Quat>,
//     command: Option<Look>,
// }

#[allow(dead_code)]
#[derive(Event, Debug)]
pub enum Camera3dCommand {
    Set { active: bool },
    Goto { position: Vec3 },
    LookAt { position: Vec3 },
    LookTo { direction: Vec3 },
    // StareAt { position: Vec3 },
    // StareTo { direction: Vec3 },
}
// #[derive(Event, Debug, Reflect)]
// pub enum Look {
//     To(Vec3),
//     At(Vec3)
// }
//
fn update_camera3d(
    mut events: EventReader<Camera3dCommand>,
    camera: Option<Single<(&mut Transform, &mut Camera), With<Nano9Camera3d>>>,
    mut commands: Commands,
) {
    if events.is_empty() {
        return;
    }
    if let Some(camera) = camera {
        let (mut transform, mut camera) = camera.into_inner();
        for event in events.read() {
            match event {
                Camera3dCommand::Set { active } => {
                    camera.is_active = *active;
                }
                Camera3dCommand::Goto { position } => {
                    let mut pos = *position;
                    // pos.y = negate_y(pos.y);
                    transform.translation = pos;
                }
                Camera3dCommand::LookAt { position } => {
                    transform.look_at(*position, Dir3::Y);
                }
                Camera3dCommand::LookTo { direction } => {
                    transform.look_to(*direction, Dir3::Y);
                }
            }
        }
    } else {
        // Make a camera if there isn't one. Handle events next frame.
        commands.spawn((
            Nano9Camera3d,
            Name::new("camera3d"),
            Camera3d::default(),
            Camera {
                order: 1,
                ..default()
            },
        ));
    }
}

// fn update_camera3d(mut query: Query<(&mut Transform, &Nano9Camera3d),
//                                     Changed<Nano9Camera3d>>) {
//     for (mut transform, camera3d) in &mut query {
//         if let Some(mut pos) = camera3d.translation {
//             pos.y = negate_y(pos.y);
//             transform.translation = pos;
//         }
//         if let Some(ori) = camera3d.orientation  {
//             transform.rotation = ori;
//         }
//     }
// }

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Nano9Camera3d>()
        .add_event::<Camera3dCommand>()
        .add_systems(Update, update_camera3d);
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    // camera3d([x,] [y,] [z,] [lx,] [ly,] [lz,]
    pub fn camera3d(&mut self, position: Option<Vec3>, look: Option<Vec3>) {
        if let Some(position) = position {
            self.commands.send_event(Camera3dCommand::Goto { position });
        }
        if let Some(position) = look {
            self.commands
                .send_event(Camera3dCommand::LookAt { position });
        }
    }

    pub fn camera3d_active(&mut self, active: bool) {
        self.commands.send_event(Camera3dCommand::Set { active });
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

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world).register(
            "camera3d",
            |ctx: FunctionCallContext,
             x: Option<f32>,
             y: Option<f32>,
             z: Option<f32>,
             lx: Option<f32>,
             ly: Option<f32>,
             lz: Option<f32>| {
                with_pico8(&ctx, move |pico8| {
                    let pos = if x.is_some() || y.is_some() || z.is_some() {
                        Some(Vec3::new(
                            x.unwrap_or(0.0),
                            y.unwrap_or(0.0),
                            z.unwrap_or(0.0),
                        ))
                    } else {
                        None
                    };
                    let ori = if lx.is_some() || ly.is_some() || lz.is_some() {
                        Some(Vec3::new(
                            lx.unwrap_or(0.0),
                            ly.unwrap_or(0.0),
                            lz.unwrap_or(0.0),
                        ))
                    } else {
                        None
                    };
                    let _: () = pico8.camera3d(pos, ori);
                    Ok(())
                })
            },
        );
    }
}
