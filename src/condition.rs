use bevy::prelude::*;

pub fn on_just_pressed(key: KeyCode) -> impl Fn(Res<ButtonInput<KeyCode>>) -> bool {
    move |input: Res<ButtonInput<KeyCode>>| input.just_pressed(key)
}

pub fn on_just_pressed_with(
    key: KeyCode,
    modifiers: Vec<KeyCode>,
) -> impl Fn(Res<ButtonInput<KeyCode>>) -> bool {
    move |input: Res<ButtonInput<KeyCode>>| {
        {
            input.just_pressed(key) && input.any_pressed(modifiers.iter().copied())
        }
    }
}
