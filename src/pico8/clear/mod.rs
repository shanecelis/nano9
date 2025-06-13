use crate::pico8::Pico8State;
use bevy::prelude::*;
// use bevy::utils::HashMap;
use mashmap::MashMap;

mod counter;
use counter::DrawCounter;

static DRAW_COUNTER: DrawCounter = DrawCounter::new(1);
const MAX_EXPECTED_CLEARABLES: f32 = 1000.0;

pub(crate) fn plugin(app: &mut App) {
    app
        .register_type::<Clearable>()
        .add_event::<ClearEvent>()
        .init_resource::<ClearCache>()
        .add_systems(Last, (handle_overflow, handle_clear_event).chain());
}

#[derive(Debug, Event, Clone, Copy)]
pub struct ClearEvent {
    draw_ceiling: usize,
}

impl Default for ClearEvent {
    fn default() -> Self {
        ClearEvent {
            draw_ceiling: DRAW_COUNTER.get(),
        }
    }
}

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
    pub fn insert(&mut self, clearable: &mut Clearable, id: Entity) -> bool {
        assert!(!clearable.cached);
        match clearable.hash {
            Some(hash) => {
                self.0.insert(hash, id);
                clearable.cached = true;
                true
            }
            None => false,
        }
    }

    /// Must mark clearable.cached = false on returned entity.
    pub fn take(&mut self, hash: &u64) -> Option<Entity> {
        self.0.remove_one(hash)
    }

    pub fn remove(&mut self, clearable: &Clearable, id: Entity) -> bool {
        if clearable.cached {
            self.0
                .drain_key_if(&clearable.hash.unwrap(), |v| *v == id)
                .next()
                .is_some()
        } else {
            false
        }
    }
}

#[derive(Debug, Component, Clone, Copy, Reflect)]
// #[component(on_add = on_insert_hook)]
// #[component(on_insert = on_insert_hook)]
// #[component(on_remove = on_remove_hook)]
pub struct Clearable {
    draw_count: usize,
    pub time_to_live: u8,
    pub hash: Option<u64>,
    pub cached: bool,
}

// fn on_insert_hook(mut world: DeferredWorld, id: Entity, _comp_id: ComponentId) {
//     let Some(hash) = world.get::<Clearable>(id).and_then(|clearable| clearable.hash) else { return; };
//     let Some(mut cache) = world.get_resource_mut::<ClearCache>() else { return; };
//     cache.insert(hash, id);
// }

// fn on_remove_hook(mut world: DeferredWorld, id: Entity, _comp_id: ComponentId) {
//     let Some(hash) = world.get::<Clearable>(id).and_then(|clearable| clearable.hash) else { return; };
//     let Some(mut cache) = world.get_resource_mut::<ClearCache>() else { return; };
//     cache.remove(&hash);
// }

impl Default for Clearable {
    fn default() -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live: 0,
            hash: None,
            cached: false,
        }
    }
}

impl Clearable {
    pub fn new(time_to_live: u8) -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live,
            hash: None,
            cached: false,
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
    pub fn resurrect(&mut self, new_time_to_live: u8) {
        self.time_to_live = new_time_to_live;
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

fn handle_clear_event(
    mut events: EventReader<ClearEvent>,
    mut query: Query<(Entity, &mut Clearable, &mut Transform, &mut Visibility)>,
    mut commands: Commands,
    mut state: ResMut<Pico8State>,
    mut cache: ResMut<ClearCache>,
) {
    if let Some(ceiling) = events.read().map(|e| e.draw_ceiling).max() {
        let (less_than, mut greater_than): (Vec<_>, Vec<_>) = query
            .iter_mut()
            .partition(|(_, clearable, _, _)| clearable.draw_count < ceiling);
        for (id, mut clearable, _, mut visibility) in less_than {
            if clearable.time_to_live == 0 {
                // These should be removed from the cache.
                commands.entity(id).despawn_recursive();
                // Remove from cache if necessary.
                let _removed = cache.remove(&clearable, id);
            } else {
                // These should go into the cache.
                clearable.time_to_live -= 1;
                *visibility = Visibility::Hidden;
                if !clearable.cached && clearable.hash.is_some() {
                    cache.insert(&mut clearable, id);
                }
            }
        }

        let mut i = 1;
        greater_than.sort_by(|(_, _, a, _), (_, _, b, _)| {
            a.translation
                .z
                .partial_cmp(&b.translation.z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (_id, mut clearable, mut transform, _) in greater_than {
            clearable.draw_count = 0;
            transform.translation.z = i as f32 / MAX_EXPECTED_CLEARABLES;
            i += 1;
        }

        if i == 1 {
            // If there aren't any more clearables, we can let the camera
            // move.
            state.draw_state.camera_position_delta = None;
        }
        DRAW_COUNTER.set(1);
    }
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
