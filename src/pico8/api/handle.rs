use bevy::prelude::*;
use super::*;

#[derive(Resource, Debug, Reflect, Deref)]
pub struct Pico8Handle {
    #[deref]
    pub handle: Handle<Pico8Asset>,
    pub script_component: Option<Entity>,
}

impl From<Handle<Pico8Asset>> for Pico8Handle {
    fn from(handle: Handle<Pico8Asset>) -> Self {
        Self {
            handle,
            script_component: None,
        }
    }
}
