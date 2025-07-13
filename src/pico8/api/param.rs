use super::*;
use bevy::ecs::system::SystemParam;

use crate::{
    pico8::{
        self, audio::SfxChannels, keyboard::KeyInput, mouse::MouseInput, rand::Rand8, Gfx,
    api::canvas::N9Canvas,
        Palettes,
    },
};

#[derive(SystemParam)]
#[allow(dead_code)]
pub struct Pico8<'w, 's> {
    // TODO: Turn these image operations into triggers so that the Pico8 system
    // parameter will not preclude users from accessing images in their rust
    // systems.
    pub(crate) images: ResMut<'w, Assets<Image>>,
    pub(crate) state: ResMut<'w, Pico8State>,
    pub(crate) commands: Commands<'w, 's>,
    pub(crate) canvas: Res<'w, N9Canvas>,
    pub(crate) player_inputs: Res<'w, PlayerInputs>,
    pub(crate) sfx_channels: Res<'w, SfxChannels>,
    // TODO: read in what we need for time so we don't lock up standard resources.
    pub(crate) time: Res<'w, Time>,
    #[cfg(feature = "level")]
    pub(crate) tiled: crate::level::tiled::Level<'w, 's>,
    pub(crate) gfxs: ResMut<'w, Assets<Gfx>>,
    pub(crate) palettes: ResMut<'w, Palettes>,
    pub(crate) rand8: Rand8<'w>,
    pub(crate) key_input: ResMut<'w, KeyInput>,
    pub(crate) mouse_input: ResMut<'w, MouseInput>,
    pub(crate) pico8_assets: ResMut<'w, Assets<Pico8Asset>>,
    pub(crate) pico8_handle: Res<'w, Pico8Handle>,
    pub(crate) defaults: Res<'w, pico8::Defaults>,
    pub(crate) clear_cache: ResMut<'w, ClearCache>,
    pub(crate) gfx_sprites: Query<'w, 's, &'static mut GfxSprite>,
    pub(crate) gfx_materials: ResMut<'w, Assets<GfxMaterial>>,
    pub(crate) sprite_sheets: ResMut<'w, Assets<SpriteSheet>>,
    pub(crate) p8_maps: ResMut<'w, Assets<P8Map>>,
}

impl Pico8<'_, '_> {

    /// Returns the current GfxMaterial. It is cached.
    pub(crate) fn gfx_material(&mut self) -> Handle<GfxMaterial> {
        self.state.gfx_material(&mut self.gfx_materials)
    }

    /// Resurrects a hidden entity with the same essential attributes.
    pub fn resurrect(&mut self, hash: u64, position: Vec2) -> Option<Entity> {
        // See if there's already an entity available.
        if let Some(id) = self.clear_cache.take(&hash) {
            let ttl = self.defaults.time_to_live;
            self.commands.queue(move |world: &mut World| {
                let maybe_z = world.get_mut::<Clearable>(id).map(|mut clearable| {
                    // We've extracted it from the cache, so it's no longer cached.
                    clearable.cached = false;
                    clearable.resurrect(ttl); // Make this a parameter.
                    clearable.suggest_z()
                });
                assert!(maybe_z.is_some());
                if let Some(mut visibility) = world.get_mut::<Visibility>(id) {
                    *visibility = Visibility::Inherited;
                }
                if let Some(mut transform) = world.get_mut::<Transform>(id) {
                    let z = maybe_z.unwrap_or(transform.translation.z);
                    transform.translation = position.extend(z);
                }
            });
            Some(id)
        } else {
            None
        }
    }
}
