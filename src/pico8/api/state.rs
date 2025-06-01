use bevy::prelude::*;
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
}

// XXX: Dump this after refactor.
impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let defaults = world.resource::<pico8::Defaults>();
        Pico8State {
            palette: 0,
            pal_map: PalMap::default(),
            draw_state: {
                let mut draw_state = DrawState::default();
                draw_state.pen = PColor::Palette(defaults.pen_color);
                draw_state
            },
        }
    }
}
