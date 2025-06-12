use bevy::prelude::*;

pub fn on_just_pressed(key: KeyCode) -> impl Fn(Res<ButtonInput<KeyCode>>) -> bool {
    move | input: Res<ButtonInput<KeyCode>>| {
        input.just_pressed(key)
    }
}
