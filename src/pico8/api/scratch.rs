use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
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
        .register("ent", |_ctx: FunctionCallContext, id: i64| {
            let id = Entity::from_bits(id as u64);
            // let entity = N9Entity {
            //     entity: id,
            //     drop: DropPolicy::Nothing,
            // };
            // let world = ctx.world()?;
            // let reference = {
            //     let allocator = world.allocator();
            //     let mut allocator = allocator.write();
            //     ReflectReference::new_allocated(entity, &mut allocator)
            // };
            // ReflectReference::into_script_ref(reference, world)
            // Ok(Val::new(0.0))
            // Ok(0.0)
            Val(id)
        })
        .register("print_ent", |_ctx: FunctionCallContext, id: Val<Entity>| {
            info!("print id {}", &id.0);
        });
    }
}
