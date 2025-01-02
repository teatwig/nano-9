use bevy::{
    ecs::{system::SystemState, world::Command},
    prelude::*,
    sprite::Anchor,
    transform::commands::AddChildInPlace,
};

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{
    UserData, UserDataFields, UserDataMethods, MetaMethod
};

use bevy_mod_scripting::api::{
    providers::bevy_ecs::LuaEntity,
    common::bevy::ScriptWorld,
};
use bevy_mod_scripting::prelude::*;
// use bevy_pixel_buffer::prelude::*;
use crate::{palette::Nano9Palette, N9Color, N9Image, DropPolicy, despawn_list};
use std::sync::OnceLock;

#[derive(Clone)]
pub struct N9Entity {
    pub entity: Entity,
    pub drop: DropPolicy,
}

impl Drop for N9Entity {
    fn drop(&mut self) {
        if matches!(self.drop, DropPolicy::Despawn) {
            if let Some(list) = despawn_list() {
                list.push(self.entity);
            } else {
                warn!("Unable to despawn {:?}.", self.entity);
            }
        }
    }
}

impl UserData for N9Entity {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        // Transform::add_fields::<'lua, Self, _>(fields);
        //
        //
        fields.add_field_method_get("name", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Name>> = SystemState::new(&mut world);
            let items = system_state.get(&mut world);
            Ok(items.get(this.entity).map(|name| name.as_str().to_owned()).ok())
        });
        fields.add_field_method_set("name", |ctx, this, value: String| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut commands = world.commands();
            commands.entity(this.entity).insert(Name::new(value));
            Ok(())
        });

        fields.add_field_method_get("image", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Sprite>> =
                SystemState::new(&mut world);
            let query = system_state.get(&mut world);
            let item = query.get(this.entity).map_err(|_| LuaError::RuntimeError("No sprite to get image".into()))?;
            Ok(N9Image {
                handle: item.image.clone(),
                layout: None,
            }) //.ok_or(LuaError::RuntimeError("No such image".into()))
        });

        // fields.add_field_method_get("sprite", |ctx, this| {
        //     let world = ctx.get_world()?;
        //     let world = ScriptWorld::new(world);
        //     // let mut world = world.write();
        //     let t = world.get_type_by_name("Sprite").unwrap();
        //     world.get_component(this.entity, t)
        //         .map_err(|e| LuaError::RuntimeError(e.to_string()))
        // });

        // fields.add_field_method_get("transform", |ctx, this| {
        //     let world = ctx.get_world()?;
        //     let world = ScriptWorld::new(world);
        //     // let mut world = world.write();
        //     let t = world.get_type_by_name("Transform").unwrap();
        //     world.get_component(this.entity, t)
        //         .map_err(|e| LuaError::RuntimeError(e.to_string()))
        // });
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |ctx, this, index: String| {

            let world = ctx.get_world()?;
            let world = ScriptWorld::new(world);
            // let mut world = world.write();
            let t = world.get_type_by_name(&index).ok_or_else(|| LuaError::RuntimeError(format!("No such type {:?}", &index)))?;
            world.get_component(this.entity, t)
                .map_err(|e| LuaError::RuntimeError(e.to_string()))
        });

        // methods.add_meta_method(MetaMethod::NewIndex, |ctx, this, index: String| {

        //     let world = ctx.get_world()?;
        //     let world = ScriptWorld::new(world);
        //     // let mut world = world.write();
        //     let t = world.get_type_by_name(&index).unwrap();
        //     world.get_component(this.entity, t)
        //         .map_err(|e| LuaError::RuntimeError(e.to_string()))
        // });
    }
}
