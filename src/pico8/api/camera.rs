use super::*;
use crate::translate::Position;

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

fn change_camera_position(In(position): In<Vec2>,
                          mut camera: Single<&mut Position, With<Nano9Camera>>,
                          mut items: Query<&mut Position, Without<Nano9Camera>>) {
    let old_position = camera.0;
    camera.0 = position;
    let dp = position - old_position;
    for mut item_position in &mut items {
        item_position.0 += dp;
    }
}

bobtail::define! {
    #[macro_export]
    fn camera(&mut self, #[tail] pos: Option<Vec2>) -> Vec2;
}

impl super::Pico8<'_, '_> {
    pub fn camera(&mut self, pos: Option<Vec2>) -> Vec2 {
        if let Some(pos) = pos {
            let last = std::mem::replace(&mut self.state.draw_state.camera_position, pos);
            self.commands.run_system_cached_with(change_camera_position, pos);
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
