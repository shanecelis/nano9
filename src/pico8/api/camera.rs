use super::*;
use crate::pico8::api::canvas::PixelCanvas;
use crate::translate::Position;

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

fn change_camera_position(
    In(position): In<Vec2>,
    mut camera: Single<&mut Position, With<Nano9Camera>>,
    mut items: Query<&mut Position, (Without<Nano9Camera>, Without<PixelCanvas>)>,
) {
    let old_position = camera.0;
    camera.0 = position;
    let dp = position - old_position;
    for mut item_position in &mut items {
        item_position.0 += dp;
    }
}

bobtail::define! {
    #[doc(hidden)]
    pub __camera => fn camera(&mut self, #[tail] pos: Option<Vec2>) -> Result<Vec2, Error>;
}
pub use __camera as camera;

impl super::Pico8<'_, '_> {
    pub fn camera(&mut self, pos: Option<Vec2>) -> Result<Vec2, Error> {
        // Pico-8 `camera()` with no arguments resets to (0, 0). Either way
        // the previous position is returned.
        let pos = pos.unwrap_or(Vec2::ZERO);
        let last = std::mem::replace(&mut self.state.draw_state.camera_position, pos);
        self.commands
            .run_system_cached_with(change_camera_position, pos);
        Ok(last)
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;
    use std::collections::VecDeque;

    use bevy_mod_scripting::bindings::{
        ScriptValue, VariadicTuple,
        function::{
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world).register(
            "camera",
            |ctx: FunctionCallContext, x: Option<f32>, y: Option<f32>| {
                with_pico8(&ctx, move |pico8| {
                    let arg = x.map(|x| Vec2::new(x, y.unwrap_or(0.0)));
                    pico8.camera(arg)
                })
                .map(|last_pos| {
                    ScriptValue::Tuple(VariadicTuple(VecDeque::from([
                        ScriptValue::from(last_pos.x as f64),
                        ScriptValue::from(last_pos.y as f64),
                    ])))
                })
            },
        );
    }
}
