use crate::pico8::api::Nano9Camera;
use bevy::prelude::*;

#[derive(Clone, Debug, Reflect, Default, Resource)]
pub struct MouseInput {
    // pub enabled: bool,
    pub position: Vec2,
    pub buttons: u8,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<MouseInput>()
        .add_systems(PreUpdate, (fill_mouse_position, fill_buttons));
}
fn fill_mouse_position(
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Nano9Camera>>,
    mut mouse_input: ResMut<MouseInput>,
) -> Result<(), BevyError> {
    let Ok(window) = windows.single() else {
        return Ok(());
    };
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return Ok(());
    };

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_transform, cursor).ok())
    {
        mouse_input.position.x = world_position.x;
        mouse_input.position.y = world_position.y;
    }
    Ok(())
}

fn fill_buttons(
    mut mouse_input: ResMut<MouseInput>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    mouse_input.buttons = 0;
    if mouse_button_input.pressed(MouseButton::Left) {
        mouse_input.buttons |= 1;
    }
    if mouse_button_input.pressed(MouseButton::Right) {
        mouse_input.buttons |= 2;
    }
    if mouse_button_input.pressed(MouseButton::Middle) {
        mouse_input.buttons |= 4;
    }
}
