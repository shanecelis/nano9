use super::*;

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera;

pub(crate) fn plugin(app: &mut App) {
    app
        .add_observer(
            |trigger: Trigger<UpdateCameraPos>,
             camera: Single<&mut Transform, With<Nano9Camera>>,
             mut state: ResMut<Pico8State>| {
                 let mut pos = trigger.event().0;
                 let mut camera = camera.into_inner();
                 pos.y = negate_y(pos.y);
                 trace!("UpdateCameraPos({:.2}, {:.2})", pos.x, pos.y);
                 camera.translation.x = pos.x;
                 camera.translation.y = pos.y;
                 state.draw_state.camera_position_delta = None;
            },
        );
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[derive(Event, Debug)]
pub(crate) struct UpdateCameraPos(pub(crate) Vec2);

impl super::Pico8<'_, '_> {
    pub fn camera(&mut self, pos: Option<Vec2>) -> Vec2 {
        if let Some(pos) = pos.map(pixel_snap) {
            let last = std::mem::replace(&mut self.state.draw_state.camera_position, pos);
            if let Some(ref mut delta) = &mut self.state.draw_state.camera_position_delta {
                // Do not move the camera. Something has already been drawn.
                // Accumulate the delta.
                *delta += last - pos;
            }
            if self.state.draw_state.is_clear {
                trace!("camera(): trigger update camera position");
                // We haven't drawn anything yet. Move the actual camera.
                self.commands.trigger(UpdateCameraPos(pos));
            }
            last
        } else {
            self.state.draw_state.camera_position
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

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world).register(
            "_camera",
            |ctx: FunctionCallContext, x: Option<f32>, y: Option<f32>| {
                with_pico8(&ctx, move |pico8| {
                    let arg = x.map(|x| Vec2::new(x, y.unwrap_or(0.0)));
                    Ok(pico8.camera(arg))
                })
                .map(|last_pos| (last_pos.x, last_pos.y))
            },
        );
    }
}
