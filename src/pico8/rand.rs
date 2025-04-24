use crate::pico8::Error;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_mod_scripting::core::{bindings::ScriptValue, error::InteropError};
use bevy_prng::WyRand;
use bevy_rand::prelude::{Entropy, EntropyPlugin, RngSeed, SeedSource};
use rand::RngCore;

#[derive(Debug, Component)]
struct Source;

#[derive(SystemParam)]
pub struct Rand8<'w> {
    rand: Single<'w, (&'static mut Entropy<WyRand>, &'static mut RngSeed<WyRand>), With<Source>>,
}

impl Rand8<'_> {
    pub fn rnd(&mut self, value: Option<ScriptValue>) -> ScriptValue {
        let value = value.unwrap_or(ScriptValue::Unit);
        let (ref mut rng, ref mut _seed) = *self.rand;
        match value {
            ScriptValue::Integer(x) => ScriptValue::from(rng.next_u64() as i64 % (x + 1)),
            ScriptValue::Float(x) => {
                ScriptValue::from(x * (rng.next_u64() as f64) / (u64::MAX as f64))
            }
            ScriptValue::Unit => ScriptValue::from((rng.next_u64() as f64) / (u64::MAX as f64)),
            ScriptValue::List(mut x) => {
                if x.is_empty() {
                    ScriptValue::Unit
                } else {
                    let index = rng.next_u64() as usize % x.len();
                    x.swap_remove(index)
                }
            }
            _ => ScriptValue::Error(InteropError::external_error(Box::new(
                Error::InvalidArgument("rng expects integer, float, or list".into()),
            ))),
        }
    }

    pub fn srand(&mut self, new_seed: u64) {
        let (ref mut rng, ref mut seed) = *self.rand;
        rng.reseed(new_seed.to_ne_bytes());
        **seed = RngSeed::<WyRand>::from_seed(new_seed.to_ne_bytes());
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(EntropyPlugin::<WyRand>::default())
        .add_systems(PreStartup, setup);
}

fn setup(mut commands: Commands) {
    commands.spawn((Source, RngSeed::<WyRand>::default()));
}
