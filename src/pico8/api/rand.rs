use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::bindings::script_value::ScriptValue;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {

    pub fn rnd<T>(&mut self, max: T) -> T
                   where T: ::rand::distributions::uniform::SampleUniform + PartialOrd + num_traits::Zero + Copy {
        self.rand8.rnd(max)
    }
    #[cfg(feature = "scripting")]
    pub fn rnd_value(&mut self, value: Option<ScriptValue>) -> ScriptValue {
        self.rand8.rnd_value(value)
    }

    pub fn srand(&mut self, seed: u64) {
        self.rand8.srand(seed)
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::bindings::{
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
                    with_pico8(&ctx, move |pico8| Ok(pico8.rnd_value(value)))
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
