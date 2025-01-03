use bevy::{
    ecs::{system::SystemState, world::Command},
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
    despawn_list, palette::Nano9Palette, DropPolicy, N9AudioLoader, N9Color, N9Image,
    N9ImageLoader, N9TextLoader, OneFrame,
};

use std::sync::OnceLock;

#[derive(Clone)]
pub struct Nano9;

impl UserData for Nano9 {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        #[cfg(feature = "level")]
        fields.add_field("level", crate::N9LevelLoader);
        fields.add_field("audio", N9AudioLoader);
        fields.add_field("image", N9ImageLoader);
        fields.add_field("text", N9TextLoader);
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_function("time", |ctx, _: ()| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Res<Time>> = SystemState::new(&mut world);
            let time = system_state.get(&world);
            Ok(time.elapsed_secs())
        });

        methods.add_function("delta_time", |ctx, _: ()| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Res<Time>> = SystemState::new(&mut world);
            let time = system_state.get(&world);
            Ok(time.delta_secs())
        });

        // methods.add_function("setpal", |ctx, img: N9Image| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     world.insert_resource(Nano9Palette(img.handle.clone()));
        //     Ok(())
        // })

        // methods.add_function("_set_global", |ctx, (name, value): (String, Value)| {
        //     ctx.globals().set(name, value)
        // });
    }
}
