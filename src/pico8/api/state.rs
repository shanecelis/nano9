use bevy::utils::HashMap;

use bevy::utils::hashbrown::hash_map::DefaultHashBuilder;
use std::{
    hash::{BuildHasher, Hash, Hasher},
};
use super::*;

/// Pico8State's state.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    #[reflect(ignore)]
    pub(crate) pal_map: PalMap,
    /// Current palette
    pub(crate) palette: usize,
    pub(crate) draw_state: DrawState,
    pub(crate) sprite_sheets: Vec<Option<SpriteSheet>>,
    pub(crate) gfx_material: Option<Handle<GfxMaterial>>,
    pub(crate) gfx_materials: HashMap<u64, Handle<GfxMaterial>>,
}

// XXX: Dump this after refactor.
impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let defaults = world.resource::<pico8::Defaults>();
        let mut pal_map = PalMap::default();
        if let Some(trans) = defaults.initial_transparent_color {
            pal_map.transparency.set(trans, true);
        }
        Pico8State {
            palette: 0,
            pal_map,
            draw_state: DrawState {
                pen: PColor::Palette(defaults.initial_pen_color),
                ..default()
            },
            sprite_sheets: vec![],
            gfx_material: None,
            gfx_materials: default(),
        }
    }
}

impl Pico8State {
    pub fn mark_palette_dirty(&mut self) {
        self.gfx_material = None;
    }

    pub(crate) fn gfx_material(&mut self, gfx_materials: &mut Assets<GfxMaterial>) -> Handle<GfxMaterial> {
        self.gfx_material.get_or_insert_with(|| {
            let hash = {
                let mut hasher = DefaultHashBuilder::default().build_hasher();
                self.palette.hash(&mut hasher);
                self.pal_map.hash(&mut hasher);
                hasher.finish()
            };
            self.gfx_materials.entry(hash)
                         .or_insert_with(|| {
                             let gfx_material = GfxMaterial {
                                 palette: self.palette,
                                 pal_map: self.pal_map.clone()
                             };
                             gfx_materials.add(gfx_material)
                         }).clone()
        }).clone()
    }
}
