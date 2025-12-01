use super::canvas;
use crate::{
    PColor,
    pico8::{Gfx, GfxDirty, GfxSprite, Pico8State, Pico8Handle, Pico8Asset},
};
use bevy::prelude::*;
use mashmap::MashMap;

mod counter;
use counter::DrawCounter;

static DRAW_COUNTER: DrawCounter = DrawCounter::new(1);
const MAX_EXPECTED_CLEARABLES: f32 = 1000.0;

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Clearable>()
        .add_event::<ClearEvent>()
        .init_resource::<ClearCache>()
        .add_systems(Last, (handle_overflow).chain())
        .add_observer(handle_clear_event);
}

#[derive(Debug, Event, Clone, Copy)]
pub struct ClearEvent {
    color: PColor,
}

impl ClearEvent {
    pub fn new(color: PColor) -> Self {
        Self { color }
    }
}

// We're relying on the hash to do all our dirty work without any Eq protection
// from collisions.

// pub enum ClearKey {
//     Map { map_pos: UVec2,
//           size: UVec2,
//           mask: Option<u8>,
//           map_index: Option<usize>,
//     }
// }

#[derive(Resource, Default)]
pub(crate) struct ClearCache(MashMap<u64, Entity>);

impl ClearCache {
    pub fn insert(&mut self, clearable: &Clearable, id: Entity) -> bool {
        // info!("CACHE INSERT {id}");
        assert!(matches!(clearable.state, ClearState::Visible));
        match clearable.hash {
            Some(hash) => {
                // info!("CACHE INSERTED {id}");
                self.0.insert(hash, id);
                true
            }
            None => false,
        }
    }

    /// Must mark clearable.cached = false on returned entity.
    pub fn take(&mut self, hash: &u64) -> Option<Entity> {
        // info!("CACHE TAKEN {:?}", &result);
        self.0.remove_one(hash)
    }

    pub fn remove(&mut self, clearable: &Clearable, id: Entity) -> bool {
        // We're trusting clearable.cached here. Should we? No.
        // if clearable.cached {
        if let Some(hash) = clearable.hash {
            let result = self.0.drain_key_if(&hash, |v| *v == id).next().is_some();
            // info!("CACHE REMOVE {:?}", &result);
            if matches!(clearable.state, ClearState::Visible) {
                // warn!("Clearable {id} requested to be removed that is in state visible, was {}removed.", if result { "" } else {"not " })
            }
            result
        } else {
            false
        }
    }
}

#[derive(Debug, Component, Clone, Copy, Reflect)]
// #[component(on_add = on_insert_hook)]
// #[component(on_insert = on_insert_hook)]
#[component(on_remove = on_remove_hook)]
pub struct Clearable {
    pub(crate) draw_count: usize,
    pub time_to_live: u8,
    pub hash: Option<u64>,
    pub state: ClearState,
}

#[derive(Debug, Component, Clone, Copy, Reflect)]
pub enum ClearState {
    Visible,
    Hidden { time_to_live: u8 },
}

impl ClearState {
    /// Decrements the time to live and returns its value or None.
    pub fn decrement_ttl(&mut self) -> Option<u8> {
        match self {
            ClearState::Visible => None,
            ClearState::Hidden { time_to_live } => {
                let last_ttl = *time_to_live;
                *time_to_live = time_to_live.saturating_sub(1);
                Some(last_ttl)
            }
        }
    }
}

// fn on_insert_hook(mut world: DeferredWorld, id: Entity, _comp_id: ComponentId) {
//     let Some(hash) = world.get::<Clearable>(id).and_then(|clearable| clearable.hash) else { return; };
//     let Some(mut cache) = world.get_resource_mut::<ClearCache>() else { return; };
//     cache.insert(hash, id);
// }

fn on_remove_hook(
    mut world: bevy::ecs::world::DeferredWorld,
    hook: bevy::ecs::component::HookContext,
) {
    let id = hook.entity;
    let Some(clearable) = world.get::<Clearable>(id).copied() else {
        return;
    };
    let Some(mut cache) = world.get_resource_mut::<ClearCache>() else {
        return;
    };
    // info!("Removing clearable {id} from cache.");

    // clearable.cached = true; // We want to ensure it's removed.
    cache.remove(&clearable, id);
    // let _ = cache.0.remove(&hash);
}

impl Default for Clearable {
    fn default() -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live: 0,
            hash: None,
            state: ClearState::Visible,
        }
    }
}

impl Clearable {
    pub fn new(time_to_live: u8) -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live,
            hash: None,
            state: ClearState::Visible,
        }
    }

    pub fn is_cached(&self) -> bool {
        !matches!(self.state, ClearState::Visible)
    }

    pub fn mark_cached(&mut self) -> bool {
        if !self.is_cached() {
            self.state = ClearState::Hidden {
                time_to_live: self.time_to_live,
            };
            true
        } else {
            false
        }
    }

    pub fn with_hash(mut self, hash: u64) -> Self {
        // That's _some_ hash!
        self.hash = Some(hash);
        self
    }

    /// Suggest a z value based on the draw count.
    pub fn suggest_z(&self) -> f32 {
        1.0 + self.draw_count as f32 / MAX_EXPECTED_CLEARABLES
    }

    /// Update the draw count and time-to-live.
    pub fn resurrect(&mut self) {
        self.state = ClearState::Visible;
        self.draw_count = DRAW_COUNTER.increment();
    }
}

fn handle_overflow(mut query: Query<&mut Clearable>) {
    if DRAW_COUNTER.overflowed() {
        for mut clearable in &mut query {
            // It will normally never be zero.
            clearable.draw_count = 0;
        }
        DRAW_COUNTER.reset_overflowed()
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_clear_event(
    trigger: Trigger<ClearEvent>,
    mut query: Query<(Entity, &mut Clearable, &mut Visibility)>,
    mut commands: Commands,
    mut state: ResMut<Pico8State>,
    mut cache: ResMut<ClearCache>,
    mut gfxs: ResMut<Assets<Gfx>>,
    one_color: Single<&mut Sprite, With<canvas::OneColorBackground>>,
    background: Single<(Entity, &GfxSprite, &mut GfxDirty), With<canvas::Background>>,
    pico8_handle: Res<Pico8Handle>,
    pico8_assets: Res<Assets<Pico8Asset>>,
    // palettes: Res<Palettes>,
) {
    state.draw_state.clear_screen();
    // Clear the 1x1 background.
    let mut sprite = one_color.into_inner();

    let Some(pico8_asset) = pico8_assets
        .get(&pico8_handle.handle) else {
            warn!("No pico8 asset setup during clear event.");
            return;
        };
        // .ok_or_else(|| Error::NoAsset("pico8".into()))
    match pico8_asset.palettes.get_color(trigger.color, state.palette) {
        Ok(color) => {
            sprite.color = color;
        }
        Err(e) => {
            error!("Could not clear to color: {e}");
            sprite.color = Srgba::rgb(1.0, 0.0, 1.0).into(); // Ugly pink
        }
    }
    let (_background_id, gfx_sprite, mut gfx_dirty) = background.into_inner();

    // Clear the background if needed.
    if gfx_dirty.0 {
        if let Some(gfx) = gfxs.get_mut(&gfx_sprite.image) {
            trace!("Clearing Background pixels.");
            gfx.data.set_elements(0x00);
        }
        gfx_dirty.0 = false;
    }

    for (id, mut clearable, mut visibility) in &mut query {
        match clearable.state.decrement_ttl() {
            Some(0) => {
                commands.entity(id).despawn();
            }
            Some(_ttl) => {
                // It still has time.
            }
            None => {
                if clearable.time_to_live == 0 || clearable.hash.is_none() {
                    commands.entity(id).despawn();
                } else {
                    *visibility = Visibility::Hidden;
                    if cache.insert(&clearable, id) {
                        assert!(clearable.mark_cached());
                    } else {
                        panic!();
                    }
                }
            }
        }
    }

    // Shouldn't we move the camera?
    // if let Some(delta) = state.draw_state.camera_position_delta.take() {
    //     commands.trigger(UpdateCameraPos(state.draw_state.camera_position));
    // }
    DRAW_COUNTER.set(1);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test0() {
        static COUNTER: DrawCounter = DrawCounter::new(0);
        assert_eq!(COUNTER.increment(), 0);
        assert_eq!(COUNTER.increment(), 1);
        assert_eq!(COUNTER.get(), 2);
    }
}
