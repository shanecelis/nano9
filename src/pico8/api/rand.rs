use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::bindings::script_value::ScriptValue;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    #[cfg(feature = "scripting")]
    pub fn rnd(&mut self, value: Option<ScriptValue>) -> ScriptValue {
        self.rand8.rnd(value)
    }

    pub fn srand(&mut self, seed: u64) {
        self.rand8.srand(seed)
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::core::bindings::{
        function::{
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        script_value::ScriptValue,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "rnd",
                |ctx: FunctionCallContext, value: Option<ScriptValue>| {
                    with_pico8(&ctx, move |pico8| Ok(pico8.rnd(value)))
                },
            )
            .register("srand", |ctx: FunctionCallContext, value: u64| {
                with_pico8(&ctx, move |pico8| {
                    pico8.srand(value);
                    Ok(())
                })
            });
    }
}
