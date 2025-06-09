use bevy::ecs::system::{SystemParam, SystemState};

use bevy_mod_scripting::core::{
    bindings::{access_map::ReflectAccessId, function::script_function::FunctionCallContext},
    error::InteropError,
};

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
    let raid = ReflectAccessId::for_global();
    if world_guard.claim_global_access() {
        let world = world_guard.as_unsafe_world_cell()?;
        let world = unsafe { world.world_mut() };
        let mut system_state: SystemState<S> = SystemState::new(world);
        let r = {
            let mut pico8 = system_state.get_mut(world);
            f(&mut pico8)
        };
        system_state.apply(world);
        unsafe { world_guard.release_global_access() };
        r.map_err(|e| InteropError::external_error(Box::new(e)))
    } else {
        Err(InteropError::cannot_claim_access(
            raid,
            world_guard.get_access_location(raid),
            "with_system_param",
        ))
    }
}

pub(crate) fn with_pico8<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Pico8) -> Result<X, Error>,
) -> Result<X, InteropError> {
    with_system_param::<Pico8, X, Error>(ctx, f)
}
