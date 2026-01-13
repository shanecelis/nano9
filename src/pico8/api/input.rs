use super::*;
use bitvec::prelude::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[derive(Default, Debug, Clone)]
pub struct Buttons {
    from: Option<Entity>,
    curr: BitArray<[u8; 1]>,
    last: BitArray<[u8; 1]>,
}

impl Buttons {
    pub fn btnp(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => {
                let curr = self
                    .curr
                    .get(b as usize)
                    .map(|x| *x.as_ref())
                    .ok_or(Error::NoSuchButton(b))?;
                let last = self
                    .last
                    .get(b as usize)
                    .map(|x| *x.as_ref())
                    .ok_or(Error::NoSuchButton(b))?;
                Ok(curr && !last)
            }
            None => Ok((self.curr & (self.curr & !self.last)).any()),
        }
    }

    pub fn btn(&self, b: Option<u8>) -> Result<bool, Error> {
        match b {
            Some(b) => self
                .curr
                .get(b as usize)
                .map(|x| *x.as_ref())
                .ok_or(Error::NoSuchButton(b)),
            None => Ok(self.curr.any()),
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct PlayerInputs(Vec<Buttons>);

impl Default for PlayerInputs {
    fn default() -> Self {
        PlayerInputs(vec![Buttons::default(); 2])
    }
}

impl super::Pico8<'_, '_> {
    pub fn btnp(&self, b: Option<u8>, player: Option<u8>) -> Result<bool, Error> {
        let Some(buttons) = self.player_inputs.get(player.unwrap_or(0) as usize) else {
            return Err(Error::NoSuch("player".into()));
        };
        buttons.btnp(b)
    }

    pub fn btn(&self, b: Option<u8>, player: Option<u8>) -> Result<bool, Error> {
        let Some(buttons) = self.player_inputs.get(player.unwrap_or(0) as usize) else {
            return Err(Error::NoSuch("player".into()));
        };
        buttons.btn(b)
    }
}

pub(crate) fn fill_input(
    mut connection_events: MessageReader<GamepadConnectionEvent>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut player_inputs: ResMut<PlayerInputs>,
) {
    for event in connection_events.read() {
        info!("{event:?}");
        if event.connected() {
            match player_inputs
                .iter_mut()
                .find(|buttons| buttons.from.is_none())
            {
                Some(buttons) => buttons.from = Some(event.gamepad),
                None => player_inputs.push(Buttons {
                    from: Some(event.gamepad),
                    ..default()
                }),
            }
        } else {
            // disconnected
            match player_inputs
                .iter_mut()
                .find(|buttons| buttons.from == Some(event.gamepad))
            {
                Some(buttons) => buttons.from = None,
                None => {
                    warn!("Gamepad disconnected but not present in player inputs.");
                }
            }
        }
    }
    for (i, buttons) in player_inputs.iter_mut().enumerate() {
        buttons.last = buttons.curr;
        buttons.curr.fill(false);

        // buttons.curr.set(0, keys.pressed(KeyCode::ArrowLeft)
        for b in 0..=5 {
            let key_pressed = match i {
                0 => match b {
                    0 => keys.pressed(KeyCode::ArrowLeft),
                    1 => keys.pressed(KeyCode::ArrowRight),
                    2 => keys.pressed(KeyCode::ArrowUp),
                    3 => keys.pressed(KeyCode::ArrowDown),
                    4 => keys.any_pressed([
                        KeyCode::KeyZ,
                        KeyCode::KeyC,
                        KeyCode::KeyN,
                        KeyCode::NumpadSubtract,
                    ]),
                    5 => keys.any_pressed([
                        KeyCode::KeyX,
                        KeyCode::KeyV,
                        KeyCode::KeyM,
                        KeyCode::Numpad8,
                    ]),
                    _ => unreachable!(),
                },
                1 => match b {
                    0 => keys.pressed(KeyCode::KeyS),
                    1 => keys.pressed(KeyCode::KeyF),
                    2 => keys.pressed(KeyCode::KeyE),
                    3 => keys.pressed(KeyCode::KeyD),
                    4 => keys.any_pressed([KeyCode::ShiftLeft, KeyCode::Tab]),
                    5 => keys.any_pressed([KeyCode::KeyA, KeyCode::KeyQ]),
                    _ => unreachable!(),
                },
                _ => false,
            };
            let (button, dir_maybe) = match b {
                0 => (GamepadButton::DPadLeft, Some(Vec2::NEG_X)),
                1 => (GamepadButton::DPadRight, Some(Vec2::X)),
                2 => (GamepadButton::DPadUp, Some(Vec2::Y)),
                3 => (GamepadButton::DPadDown, Some(Vec2::NEG_Y)),
                4 => (GamepadButton::South, None),
                5 => (GamepadButton::East, None),
                _ => unreachable!(),
            };
            let button_pressed = buttons
                .from
                .and_then(|id| {
                    // We have a gamepad.
                    gamepads.get(id).ok().map(|gamepad| {
                        gamepad.pressed(button)
                            || dir_maybe
                                .map(|dir| gamepad.left_stick().dot(dir) > ANALOG_STICK_THRESHOLD)
                                .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            buttons.curr.set(b, key_pressed || button_pressed);
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
            .register(
                "btnp",
                |ctx: FunctionCallContext, b: Option<u8>, p: Option<u8>| {
                    with_pico8(&ctx, |pico8| pico8.btnp(b, p))
                },
            )
            .register(
                "btn",
                |ctx: FunctionCallContext, b: Option<u8>, p: Option<u8>| {
                    with_pico8(&ctx, |pico8| pico8.btn(b, p))
                },
            );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_buttons() {
        let mut b = Buttons::default();
        assert!(!b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
        b.curr.set(0, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.last.set(1, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.curr.set(1, true);
        assert!(b.btn(None).unwrap());
        assert!(b.btnp(None).unwrap());
        b.last = b.curr;
        assert!(b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
        b.curr.set(0, false);
        b.curr.set(1, false);
        b.last.set(1, false);
        assert!(!b.btn(None).unwrap());
        assert!(!b.btnp(None).unwrap());
    }
}
