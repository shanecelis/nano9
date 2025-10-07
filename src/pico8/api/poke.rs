use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::bindings::script_value::ScriptValue;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn poke(&mut self, addr: usize, value: u8) -> Result<(), Error> {
        match addr {
            0x5f2d => {
                self.key_input.enabled = value != 0;
            }
            _ => Err(Error::UnsupportedPoke(addr))?,
        }
        Ok(())
    }

    pub fn peek(&mut self, addr: usize) -> Result<u8, Error> {
        Err(Error::UnsupportedPeek(addr))
    }

    #[cfg(feature = "scripting")]
    pub fn stat(&mut self, n: u8, _value: Option<u8>) -> Result<ScriptValue, Error> {
        match n {
            8 => Ok(ScriptValue::Float(1.0 / self.delta_time() as f64)), // This should be the target frame rate
            9 => Ok(ScriptValue::Float(1.0 / self.delta_time() as f64)),
            30 => Ok(ScriptValue::Bool(!self.key_input.buffer.is_empty())),
            31 => self.key_input.pop().map(|string_maybe| {
                string_maybe
                    .map(ScriptValue::String)
                    .unwrap_or(ScriptValue::Unit)
            }),
            32 => Ok(ScriptValue::Float(self.mouse_input.position.x as f64)),
            33 => Ok(ScriptValue::Float(
                negate_y(self.mouse_input.position.y) as f64
            )),
            34 => Ok(ScriptValue::Integer(self.mouse_input.buttons as i64)),
            _ => Err(Error::UnsupportedStat(n))?,
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
            .register("peek", |ctx: FunctionCallContext, addr: usize| {
                with_pico8(&ctx, move |pico8| pico8.peek(addr))
            })
            .register(
                "poke",
                |ctx: FunctionCallContext, addr: usize, value: u8| {
                    with_pico8(&ctx, move |pico8| pico8.poke(addr, value))
                },
            )
            .register(
                "stat",
                |ctx: FunctionCallContext, n: u8, value: Option<u8>| {
                    with_pico8(&ctx, move |pico8| pico8.stat(n, value))
                },
            );
    }
}
