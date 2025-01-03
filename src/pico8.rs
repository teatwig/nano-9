use bevy::{
    ecs::{system::SystemState, world::Command},
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
    sprite::Anchor,
    transform::commands::AddChildInPlace,
};

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{
    MetaMethod, UserData, UserDataFields, UserDataMethods,
};

use bevy_mod_scripting::api::{common::bevy::ScriptWorld, providers::bevy_ecs::LuaEntity};
use bevy_mod_scripting::prelude::*;
// use bevy_pixel_buffer::prelude::*;
use crate::{
    api::N9Args, despawn_list, palette::Nano9Palette, DropPolicy, N9AudioLoader, N9Color, N9Image,
    N9ImageLoader, N9TextLoader, Nano9Screen, OneFrame,
};

use std::sync::{Mutex, OnceLock};

pub const PICO8_PALETTE: &'static str = "images/pico-8-palette.png";
pub const PICO8_SPRITES: &'static str = "images/kenney-pico-8-city.png";

/// Pico8's state.
#[derive(Clone)]
pub struct Pico8 {
    palette: Handle<Image>,
    sprites: Handle<Image>,
}

impl FromWorld for Pico8 {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };

        Pico8 {
            palette: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
            sprites: asset_server.load_with_settings(PICO8_SPRITES, pixel_art_settings),
        }
    }
}

impl UserData for Pico8 {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        // fields.add_field("audio", N9AudioLoader);
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {}
}

pub struct Pico8API(Option<Pico8>);

impl FromWorld for Pico8API {
    fn from_world(world: &mut World) -> Self {
        Pico8API(Some(Pico8::from_world(world)))
    }
}

pub fn plugin(app: &mut App) {
    let pico8_api = Pico8API::from_world(app.world_mut());
    app.add_api_provider::<LuaScriptHost<N9Args>>(Box::new(pico8_api));
}

impl APIProvider for Pico8API {
    type APITarget = Mutex<Lua>;
    type ScriptContext = Mutex<Lua>;
    type DocTarget = LuaDocFragment;

    fn attach_api(&mut self, ctx: &mut Self::APITarget) -> Result<(), ScriptError> {
        // callbacks can receive any `ToLuaMulti` arguments, here '()' and
        // return any `FromLuaMulti` arguments, here a `usize`
        // check the Rlua documentation for more details

        let ctx = ctx.get_mut().unwrap();
        ctx.globals()
            .set("pico8", self.0.take().unwrap())
            .map_err(ScriptError::new_other)?;

        crate::macros::define_globals! {
        // XXX: This should be demoted in favor of a general `input` solution.
        fn btnp(ctx, b: u8) {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Res<ButtonInput<KeyCode>>> =
                SystemState::new(&mut world);
            let input = system_state.get(&world);
            Ok(input.just_pressed(match b {
                0 => KeyCode::ArrowLeft,
                1 => KeyCode::ArrowRight,
                2 => KeyCode::ArrowUp,
                3 => KeyCode::ArrowDown,
                4 => KeyCode::KeyZ,
                5 => KeyCode::KeyX,
                x => todo!("key {x:?}"),
            }))
        }

        fn btn(ctx, b: u8) {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Res<ButtonInput<KeyCode>>> =
                SystemState::new(&mut world);
            let input = system_state.get(&world);
            Ok(input.pressed(match b {
                0 => KeyCode::ArrowLeft,
                1 => KeyCode::ArrowRight,
                2 => KeyCode::ArrowUp,
                3 => KeyCode::ArrowDown,
                4 => KeyCode::KeyZ,
                5 => KeyCode::KeyX,
                x => todo!("key {x:?}"),
            }))
        }

        fn cls(ctx, value: N9Color) {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let c = Nano9Palette::get_color_or_pen(value, &mut world);
            let mut system_state: SystemState<(Res<Nano9Screen>, ResMut<Assets<Image>>)> =
                SystemState::new(&mut world);
            let (screen, mut images) = system_state.get_mut(&mut world);
            let image = images.get_mut(&screen.0).unwrap();
            for i in 0..image.width() {
                for j in 0..image.height() {
                    image.set_color_at(i, j, c).map_err(|_| LuaError::RuntimeError("Could not set pixel color".into()))?;
                }
            }
            system_state.apply(&mut world);
            Ok(())
        }

        fn pset(ctx, (x, y, c): (f32, f32, N9Color)) {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let color = Nano9Palette::get_color_or_pen(c, &mut world);
            let mut system_state: SystemState<(Res<Nano9Screen>, ResMut<Assets<Image>>)> =
                SystemState::new(&mut world);
            let (screen, mut images) = system_state.get_mut(&mut world);
            let image = images.get_mut(&screen.0).unwrap();
            let _ = image.set_color_at(x as u32, y as u32, color);
            system_state.apply(&mut world);
            Ok(())
        }
        }

        Ok(())
    }

    fn register_with_app(&self, app: &mut App) {
        // app.register_type::<Settings>();
    }
}
