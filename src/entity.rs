use bevy::prelude::*;

use crate::pico8::{negate_y, Clearable, Pico8State};
use bevy_mod_scripting::{
    core::bindings::{
        function::{from::Val, namespace::NamespaceBuilder, script_function::FunctionCallContext},
    },
    lua::mlua::{self, FromLua, Lua, UserData, Value},
};


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
            |ctx: FunctionCallContext,
             this: Val<N9Entity>,
             x: Option<f32>,
             y: Option<f32>,
             z: Option<f32>| {
                let world = ctx.world()?;
                let pos = world.with_global_access(|world| {
                    let camera_position_delta = world
                        .get_resource::<Pico8State>()
                        .and_then(|state| state.draw_state.camera_position_delta);
                    if x.is_some() || y.is_some() || z.is_some() {
                        world
                            .get_mut::<Transform>(this.entity)
                            .map(|mut transform| {
                                let last = transform.translation;
                                if let Some(x) = x {
                                    transform.translation.x =
                                        camera_position_delta.map(|d| x + d.x).unwrap_or(x);
                                }
                                if let Some(y) = y {
                                    transform.translation.y = negate_y(
                                        camera_position_delta.map(|d| y + d.y).unwrap_or(y),
                                    );
                                    // transform.translation.y = camera_position_delta.map(|d| negate_y(y) + d.y).unwrap_or_else(|| negate_y(y));
                                }
                                if let Some(z) = z {
                                    transform.translation.z = z;
                                }
                                last
                            })
                    } else {
                        world
                            .get::<Transform>(this.entity)
                            .map(|transform| transform.translation)
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
