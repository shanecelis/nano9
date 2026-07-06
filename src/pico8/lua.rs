use bevy::ecs::system::{SystemParam, SystemState};

use bevy_mod_scripting::bindings::InteropError;
use bevy_mod_scripting::bindings::function::script_function::FunctionCallContext;

use crate::pico8::{Error, Pico8};

pub(crate) fn with_system_param<
    S: SystemParam + 'static,
    X,
    E: std::error::Error + Send + Sync + 'static,
>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut S::Item<'_, '_>) -> Result<X, E>,
) -> Result<X, InteropError> {
    let world_guard = ctx.world()?;
    world_guard
        .with_world_mut_access_and_then(|world| {
            let mut system_state: SystemState<S> = SystemState::new(world);
            let r = {
                let mut param = system_state
                    .get_mut(world)
                    .map_err(|e| InteropError::external(Box::new(e)))?;
                f(&mut param)
            };
            system_state.apply(world);
            r.map_err(|e| InteropError::external(Box::new(e)))
        })
        .map_err(Into::into)
}

pub(crate) fn with_pico8<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Pico8) -> Result<X, Error>,
) -> Result<X, InteropError> {
    with_system_param::<Pico8, X, Error>(ctx, f)
}
