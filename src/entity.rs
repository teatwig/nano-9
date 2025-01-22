use bevy::prelude::*;

use bevy_mod_scripting::{
    core::bindings::{ThreadWorldContainer, WorldContainer},
    lua::mlua::{
        self, FromLua, Lua, UserData, UserDataFields, Value,
    },
};

use std::any::TypeId;

#[derive(Debug, Clone, Copy, Reflect)]
pub enum DropPolicy {
    Nothing,
    Despawn,
}

#[derive(Clone, Reflect)]
pub struct N9Entity {
    pub entity: Entity,
    pub drop: DropPolicy,
}

impl UserData for DropPolicy {}

impl FromLua for DropPolicy {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl Drop for N9Entity {
    fn drop(&mut self) {
        if matches!(self.drop, DropPolicy::Despawn) {
            warn!("Retained entity leaked {:?}.", self.entity);
        }
    }
}

// pub(crate) fn register_script_functions(app: &mut App) {
//     app.world_mut();
//     NamespaceBuilder::<N9Entity>::new_unregistered(world)
//         .register("name", |this: CallerContext, world: WorldCallbackAccess| {
//             world.get_component(

//         }

// }

impl UserData for N9Entity {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |ctx, this| {
            let world = ThreadWorldContainer.try_get_world()?;
            let name = world
                .get_component_id(TypeId::of::<Name>())?
                .expect("Name component id");
            if let Ok(maybe_name) = world.get_component(this.entity, name) {
                if let Some(name_reflect) = maybe_name {
                    if let Ok(name) = name_reflect.downcast::<Name>(world) {
                        return Ok(Some(name.as_str().to_owned()));
                    }
                }
            }
            Ok(None)
            // let world = ctx.get_world()?;
            // let mut world = world.write();
            // let mut system_state: SystemState<Query<&Name>> = SystemState::new(&mut world);
            // let items = system_state.get(&mut world);
            // Ok(items
            //     .get(this.entity)
            //     .map(|name| name.as_str().to_owned())
            //     .ok())
        });

        // TODO: Try to do this one later.
        // fields.add_field_method_set("name", |ctx, this, value: String| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     let mut commands = world.commands();
        //     commands.entity(this.entity).insert(Name::new(value));
        //     Ok(())
        // });

        // fields.add_field_method_get("image", |ctx, this| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     let mut system_state: SystemState<Query<&Sprite>> = SystemState::new(&mut world);
        //     let query = system_state.get(&mut world);
        //     let item = query
        //         .get(this.entity)
        //         .map_err(|_| LuaError::RuntimeError("No sprite to get image".into()))?;
        //     // XXX: Is layout actually none?
        //     Ok(N9Image {
        //         handle: item.image.clone(),
        //         layout: None,
        //     }) //.ok_or(LuaError::RuntimeError("No such image".into()))
        // });

        // fields.add_field_method_set("one_frame", |ctx, this, value: bool| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     let mut commands = world.commands();
        //     if value {
        //         commands.entity(this.entity).insert(OneFrame::default());
        //     } else {
        //         commands.entity(this.entity).remove::<OneFrame>();
        //     }
        //     Ok(())
        // });

        // fields.add_field_method_get("one_frame", |ctx, this| {
        //     let world = ctx.get_world()?;
        //     let world = world.write();
        //     Ok(world.entity(this.entity).contains::<OneFrame>())
        // });

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

    // fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
    //     methods.add_meta_method(MetaMethod::Index, |ctx, this, index: String| {

    //         let world = ThreadWorldContainer.try_get_world()?;
    //         // let name = world.get_component_id(TypeId::of::<Name>()).expect("Name component id");
    //         // let world = ctx.get_world()?;
    //         // let world = ScriptWorld::new(world);
    //         // let mut world = world.write();

    //         if let Some(t) = world.get_type_by_name(index) {
    //             // .ok_or_else(|| LuaError::RuntimeError(format!("No such type {:?}", &index)))?;
    //             if let Some(comp_id) = world.get_component_id(t.type_id()) {
    //                 return Ok(world.get_component(this.entity, comp_id)?);
    //             }
    //         }
    //         Ok(None::<ReflectReference>)
    //             // .map_err(|e| LuaError::RuntimeError(e.to_string()))
    //     });

    //     // methods.add_meta_method(MetaMethod::NewIndex, |ctx, this, index: String| {

    //     //     let world = ctx.get_world()?;
    //     //     let world = ScriptWorld::new(world);
    //     //     // let mut world = world.write();
    //     //     let t = world.get_type_by_name(&index).unwrap();
    //     //     world.get_component(this.entity, t)
    //     //         .map_err(|e| LuaError::RuntimeError(e.to_string()))
    //     // });
    // }
}
