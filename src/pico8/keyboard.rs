use crate::pico8::Error;
use bevy::{
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
};
use std::{borrow::Cow, collections::VecDeque};

#[derive(Clone, Debug, Reflect, Default, Resource)]
pub struct KeyInput {
    pub enabled: bool,
    pub buffer: VecDeque<Key>,
}

impl KeyInput {
    pub fn pop(&mut self) -> Result<Option<Cow<'static, str>>, Error> {
        if let Some(key) = self.buffer.pop_front() {
            use bevy::input::keyboard::Key::*;
            match key {
                Character(smol_str) => Ok(Some(smol_str.to_string().into())),
                Enter => Ok(Some("\r".into())),
                Backspace => Ok(Some("\x08".into())),
                Space => Ok(Some(" ".into())),
                Tab => Ok(Some("\t".into())),
                Escape => Ok(Some("\x1b".into())),
                _ => Err(Error::Unsupported(format!("stat 31 key {:?}", &key).into())),
            }
        } else {
            Ok(None)
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<KeyInput>()
        .add_systems(PreUpdate, fill_buffer)
        .add_systems(Last, clear_buffer);
}

fn fill_buffer(mut key_input: ResMut<KeyInput>, mut char_input_events: MessageReader<KeyboardInput>) {
    if !key_input.enabled {
        return;
    }
    for event in char_input_events.read() {
        // Only check for characters when the key is pressed.
        if !event.state.is_pressed() {
            continue;
        }
        key_input.buffer.push_back(event.logical_key.clone());
    }
}

fn clear_buffer(mut key_input: ResMut<KeyInput>) {
    key_input.buffer.clear();
}
