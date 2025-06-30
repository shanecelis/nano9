use super::*;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::event::Recipients;

#[derive(Resource, Debug, Reflect, Deref)]
pub struct Pico8Handle {
    #[deref]
    pub handle: Handle<Pico8Asset>,
    #[reflect(ignore)]
    #[cfg(feature = "scripting")]
    pub main_script: Option<Recipients>,
}

impl From<Handle<Pico8Asset>> for Pico8Handle {
    fn from(handle: Handle<Pico8Asset>) -> Self {
        Self {
            handle,
            #[cfg(feature = "scripting")]
            main_script: None,
        }
    }
}
