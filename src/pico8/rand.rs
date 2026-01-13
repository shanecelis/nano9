use crate::pico8::Error;
use ::rand::Rng;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_mod_scripting::bindings::InteropError;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::bindings::ScriptValue;
use bevy_prng::WyRand;
use bevy_rand::prelude::{EntropyPlugin, RngSeed, SeedSource};
use rand::RngCore;

#[derive(Debug, Component)]
struct Source;

#[derive(SystemParam)]
pub struct Rand8<'w, 's> {
    commands: Commands<'w, 's>,
    rand: Single<'w, 's, (Entity, &'static mut WyRand), With<Source>>,
}

impl Rand8<'_, '_> {
    pub fn rnd<T>(&mut self, max: T) -> T
    where
        T: ::rand::distr::uniform::SampleUniform + PartialOrd + num_traits::Zero + Copy,
    {
        let (_id, rng) = &mut *self.rand;
        rng.random_range(T::zero()..max)
    }

    #[cfg(feature = "scripting")]
    pub fn rnd_value(&mut self, value: Option<ScriptValue>) -> ScriptValue {
        let value = value.unwrap_or(ScriptValue::Unit);
        let (_id, rng) = &mut *self.rand;
        match value {
            ScriptValue::Integer(x) => {
                ScriptValue::from(
                    x as f64 * (rng.next_u64().saturating_sub(1) as f64) / (u64::MAX as f64),
                )
                // ScriptValue::from(rng.next_u64() as i64 % (x + 1)),
            }
            ScriptValue::Float(x) => {
                ScriptValue::from(x * (rng.next_u64().saturating_sub(1) as f64) / (u64::MAX as f64))
            }
            ScriptValue::Unit => {
                ScriptValue::from((rng.next_u64().saturating_sub(1) as f64) / (u64::MAX as f64))
            }
            ScriptValue::List(mut x) => {
                if x.is_empty() {
                    ScriptValue::Unit
                } else {
                    let index = rng.next_u64() as usize % x.len();
                    x.swap_remove(index)
                }
            }
            _ => ScriptValue::Error(InteropError::external(Box::new(Error::InvalidArgument(
                "rng expects integer, float, or list".into(),
            )))),
        }
    }

    // XXX: For now.
    // #[allow(deprecated)]
    pub fn srand(&mut self, new_seed: u64) {
        let (id, _rng) = &mut *self.rand;
        self.commands
            .entity(*id)
            .insert(RngSeed::<WyRand>::from_seed(new_seed.to_ne_bytes()));
        // rng.reseed(new_seed.to_ne_bytes());
        // Commands does do it
        // commands.reseed(new_seed);
        //**seed = RngSeed::<WyRand>::from_seed(new_seed.to_ne_bytes());
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(EntropyPlugin::<WyRand>::default())
        .add_systems(PreStartup, setup);
}

fn setup(mut commands: Commands) {
    commands.spawn((Source, RngSeed::<WyRand>::default()));
}
