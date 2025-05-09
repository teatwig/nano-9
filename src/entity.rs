use bevy::prelude::*;

use crate::pico8::{negate_y, Clearable};
use bevy_mod_scripting::{
    core::bindings::{
        ScriptValue,
        function::{from::Val, namespace::NamespaceBuilder, script_function::FunctionCallContext},
        ThreadWorldContainer, WorldContainer,
    },
    lua::mlua::{self, prelude::LuaError, FromLua, Lua, UserData, UserDataFields, Value},
};

use std::sync::Arc;

#[derive(Debug, Clone, Copy, Reflect)]
pub enum DropPolicy {
    Nothing,
    Despawn,
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

#[derive(Clone, Reflect)]
pub struct N9Entity {
    pub entity: Entity,
    pub drop: DropPolicy,
}

pub(crate) fn plugin(app: &mut App) {
    NamespaceBuilder::<N9Entity>::new(app.world_mut())
        .register(
            "retain",
            |ctx: FunctionCallContext, this: Val<N9Entity>, z: Option<f32>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    let mut commands = world.commands();
                    commands.entity(this.entity).remove::<Clearable>();
                    if let Some(mut transform) = world.get_mut::<Transform>(this.entity) {
                        transform.translation.z = z.unwrap_or(0.0);
                    }
                })?;
                Ok(this)
            },
        )
        .register(
            "pos",
            |ctx: FunctionCallContext, this: Val<N9Entity>, x: Option<f32>, y: Option<f32>, z: Option<f32>| {
                let world = ctx.world()?;
                let pos = world.with_global_access(|world| {
                    if x.is_some() || y.is_some() || z.is_some() {
                        world.get_mut::<Transform>(this.entity).map(|mut transform| {
                            let last = transform.translation;
                            if let Some(x) = x {
                                transform.translation.x = x;
                            }
                            if let Some(y) = y {
                                transform.translation.y = negate_y(y);
                            }
                            if let Some(z) = z {
                                transform.translation.z = z;
                            }
                            last
                        })
                    } else {
                        world.get::<Transform>(this.entity).map(|transform| transform.translation)
                    }
                })?;
                if let Some(pos) = pos {
                    Ok(Some(vec![pos.x, negate_y(pos.y), pos.z]))
                } else {
                    Ok(None)
                }
            },
        )
        .register(
            "name",
            |ctx: FunctionCallContext, this: Val<N9Entity>, new_name: Option<String>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    if let Some(name) = new_name {
                        let mut commands = world.commands();
                        commands.entity(this.entity).insert(Name::new(name));
                        None
                    } else {
                        world
                            .get::<Name>(this.entity)
                            .map(|n| n.as_str().to_string())
                    }
                })
            },
        )
        .register(
            "vis",
            |ctx: FunctionCallContext, this: Val<N9Entity>, vis: Option<bool>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    if let Some(vis) = vis {
                        if let Some(mut visible) = world.get_mut::<Visibility>(this.entity) {
                            *visible = match vis {
                                // None => Visibility::Inherited,
                                true => Visibility::Visible,
                                false => Visibility::Hidden,
                            };
                        }
                        None
                    } else {
                        world
                            .get::<Visibility>(this.entity)
                            .map(|v| !matches!(v, Visibility::Hidden))
                    }
                })
            },
        )
        .register(
            "despawn",
            |ctx: FunctionCallContext, this: Val<N9Entity>| {
                let world = ctx.world()?;
                world.with_global_access(|world| {
                    let mut commands = world.commands();
                    commands.entity(this.entity).despawn_recursive();
                })?;
                Ok(())
            },
        );
}

impl UserData for N9Entity {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_ctx, this| {
            let world = ThreadWorldContainer
                .try_get_world()
                .map_err(|e| LuaError::ExternalError(Arc::new(e)))?;
            world
                .with_component(this.entity, |name: Option<&Name>| {
                    name.map(|s| s.as_str().to_owned())
                })
                .map_err(|e| LuaError::ExternalError(Arc::new(e)))
        });

        fields.add_field_method_set("name", |_ctx, this, value: String| {
            let world = ThreadWorldContainer
                .try_get_world()
                .map_err(|e| LuaError::ExternalError(Arc::new(e)))?;
            // with_or_insert_component_mut(&world, this.entity, |name: &mut Name| {
            //     name.mutate(|s| *s = value);
            // })
            // .map_err(|e| LuaError::ExternalError(Arc::new(e)))
            world
                .with_or_insert_component_mut(this.entity, |name: &mut Name| {
                    name.mutate(|s| *s = value);
                })
                .map_err(|e| LuaError::ExternalError(Arc::new(e)))
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
