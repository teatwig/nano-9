use bevy::{ecs::system::SystemState, prelude::*};

use bevy_mod_scripting::prelude::*;
// use bevy_pixel_buffer::prelude::*;
use crate::{assets::ImageHandles, DrawState, N9Color};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup);
}

fn setup(mut commands: Commands, image_handles: Res<ImageHandles>) {
    commands.insert_resource(Nano9Palette(
        image_handles
            .get(ImageHandles::PICO8_PALETTE)
            .unwrap()
            .clone(),
    ));
}

#[derive(Resource)]
pub struct Nano9Palette(pub Handle<Image>);

impl Nano9Palette {
    pub fn get_color_or_pen(c: impl Into<N9Color>, world: &mut World) -> Color {
        let mut system_state: SystemState<(Res<Nano9Palette>, Res<Assets<Image>>, Res<DrawState>)> =
            SystemState::new(world);
        let (palette, images, draw_state) = system_state.get(world);
        match c.into() {
            N9Color::Pen => draw_state.pen,
            N9Color::Palette(n) => images
                .get(&palette.0)
                .and_then(|pal| pal.get_color_at_1d(n as u32).ok())
                .unwrap(),
            N9Color::Color(c) => c.into(),
        }
    }

    pub fn get_color(index: usize, world: &mut World) -> Result<Color, LuaError> {
        let mut system_state: SystemState<(Res<Nano9Palette>, Res<Assets<Image>>, Res<DrawState>)> =
            SystemState::new(world);
        let (palette, images, _draw_state) = system_state.get(world);

        images
            .get(&palette.0)
            .ok_or_else(|| LuaError::RuntimeError(format!("no such palette {:?}", &palette.0)))
            .and_then(|pal| {
                pal.get_color_at_1d(index as u32)
                    .map_err(|_| LuaError::RuntimeError(format!("no such pixel index {:?}", index)))
            })
    }
}
