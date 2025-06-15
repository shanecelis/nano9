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
        }
    }
}
