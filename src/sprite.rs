use bevy::{
    ecs::{system::SystemState, world::Command},
    prelude::*,
    sprite::Anchor,
    transform::commands::AddChildInPlace,
};

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{
    UserData, UserDataFields, UserDataMethods,
};

use bevy_mod_scripting::api::providers::bevy_ecs::LuaEntity;
use bevy_mod_scripting::prelude::*;
// use bevy_pixel_buffer::prelude::*;
use crate::{palette::Nano9Palette, N9Color, N9Image};

pub(crate) fn plugin(_app: &mut App) {
}

#[derive(Debug, Clone, Copy)]
pub enum DropPolicy {
    Nothing,
    Despawn,
}

impl UserData for DropPolicy {}

impl FromLua<'_> for DropPolicy {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

pub struct N9Sprite {
    pub entity: Entity,
    pub drop: DropPolicy,
}

pub struct N9LocalTransform {
    entity: Entity,
}

pub struct N9GlobalTransform {
    entity: Entity,
}

impl Drop for N9Sprite {
    fn drop(&mut self) {
        if matches!(self.drop, DropPolicy::Despawn) {
            warn!("Retained sprite leaked {:?}.", self.entity);
        }
    }
}

pub(crate) trait EntityRep {
    fn entity(&self) -> Entity;
}

pub(crate) trait UserDataComponent {
    fn add_fields<'lua, S: EntityRep, F: UserDataFields<'lua, S>>(_fields: &mut F) {}

    #[allow(dead_code)]
    fn add_methods<'lua, S: EntityRep, M: UserDataMethods<'lua, S>>(_methods: &mut M) {}
}

impl<T: EntityRep> UserDataComponent for T {
    fn add_fields<'lua, S: EntityRep, F: UserDataFields<'lua, S>>(fields: &mut F) {
        fields.add_field_method_get("parent", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Parent>> = SystemState::new(&mut world);
            let parents = system_state.get(&world);
            if let Ok(p) = parents.get(this.entity()) {
                LuaEntity::new(p.get()).into_lua(ctx)
            } else {
                Ok(Value::Nil)
            }
            // .map_err(|e| LuaError::RuntimeError("No parent available".into()))
        });
        fields.add_field_method_set("parent", |ctx, this, parent: LuaEntity| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let cmd = AddChildInPlace {
                child: this.entity(),
                parent: parent.inner()?,
            };
            cmd.apply(&mut world);
            Ok(())
        });
    }
}

impl UserDataComponent for Transform {
    fn add_fields<'lua, S: EntityRep, F: UserDataFields<'lua, S>>(fields: &mut F) {
        fields.add_field_method_get("x", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Transform>> = SystemState::new(&mut world);
            let transforms = system_state.get(&world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation.x)
        });

        fields.add_field_method_set("x", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Transform>> = SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let mut transform = transforms.get_mut(this.entity()).unwrap();
            transform.translation.x = value;
            system_state.apply(&mut world);
            Ok(())
        });
        fields.add_field_method_get("y", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Transform>> = SystemState::new(&mut world);
            let transforms = system_state.get(&world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation.y)
        });

        fields.add_field_method_set("y", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Transform>> = SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let mut transform = transforms.get_mut(this.entity()).unwrap();
            transform.translation.y = value;
            Ok(())
        });

        fields.add_field_method_get("z", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Transform>> = SystemState::new(&mut world);
            let transforms = system_state.get(&world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation.z)
        });

        fields.add_field_method_set("z", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Transform>> = SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let mut transform = transforms.get_mut(this.entity()).unwrap();
            transform.translation.z = value;
            Ok(())
        });

        fields.add_field_method_set("parent", |ctx, this, parent: LuaEntity| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let cmd = AddChildInPlace {
                child: this.entity(),
                parent: parent.inner()?,
            };
            cmd.apply(&mut world);
            Ok(())
        });

        fields.add_field_method_get("global", |_ctx, this| {
            Ok(N9GlobalTransform {
                entity: this.entity(),
            })
        });
        fields.add_field_method_get("entity", |_ctx, this| Ok(LuaEntity::new(this.entity())));
    }
}

impl UserDataComponent for GlobalTransform {
    fn add_fields<'lua, S: EntityRep, F: UserDataFields<'lua, S>>(fields: &mut F) {
        fields.add_field_method_get("x", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&GlobalTransform>> =
                SystemState::new(&mut world);
            let transforms = system_state.get(&world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation().x)
        });

        fields.add_field_method_set("x", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<(&mut Transform, &GlobalTransform)>> =
                SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let (mut transform, global) = transforms.get_mut(this.entity()).unwrap();
            let m = global.compute_matrix().inverse();
            let mut p = global.translation();
            p.x = value;
            transform.translation = m.transform_vector3(p);
            Ok(())
        });

        fields.add_field_method_get("y", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&GlobalTransform>> =
                SystemState::new(&mut world);
            let transforms = system_state.get(&mut world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation().y)
        });

        fields.add_field_method_set("y", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<(&mut Transform, &GlobalTransform)>> =
                SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let (mut transform, global) = transforms.get_mut(this.entity()).unwrap();
            let m = global.compute_matrix().inverse();
            let mut p = global.translation();
            p.y = value;
            transform.translation = m.transform_vector3(p);
            system_state.apply(&mut world);
            Ok(())
        });

        fields.add_field_method_get("z", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&GlobalTransform>> =
                SystemState::new(&mut world);
            let transforms = system_state.get(&world);
            let transform = transforms.get(this.entity()).unwrap();
            Ok(transform.translation().z)
        });

        fields.add_field_method_set("z", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<(&mut Transform, &GlobalTransform)>> =
                SystemState::new(&mut world);
            let mut transforms = system_state.get_mut(&mut world);
            let (mut transform, global) = transforms.get_mut(this.entity()).unwrap();
            let m = global.compute_matrix().inverse();
            let mut p = global.translation();
            p.z = value;
            transform.translation = m.transform_vector3(p);
            system_state.apply(&mut world);
            Ok(())
        });

        fields.add_field_method_set("parent", |ctx, this, parent: LuaEntity| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let cmd = AddChildInPlace {
                child: this.entity(),
                parent: parent.inner()?,
            };
            cmd.apply(&mut world);
            Ok(())
        });

        fields.add_field_method_get("entity", |_ctx, this| Ok(LuaEntity::new(this.entity())));
        fields.add_field_method_get("loc", |_ctx, this| {
            Ok(N9LocalTransform {
                entity: this.entity(),
            })
        });
    }
}

impl EntityRep for N9Sprite {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl EntityRep for N9LocalTransform {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl UserData for N9LocalTransform {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        Transform::add_fields::<Self, _>(fields);
    }
}

impl EntityRep for N9GlobalTransform {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl UserData for N9GlobalTransform {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        GlobalTransform::add_fields::<Self, _>(fields);
    }
}

impl UserData for N9Sprite {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        Transform::add_fields::<Self, _>(fields);

        fields.add_field_method_set("color", |ctx, this, value: Option<N9Color>| {
            let world = ctx.get_world()?;
            let mut world = world.write();

            let c = value
                .map(|v| Nano9Palette::get_color_or_pen(v, &mut world))
                .unwrap_or(Color::WHITE);
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.color = c;
            Ok(())
        });

        fields.add_field_method_get("flip_x", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Sprite>> = SystemState::new(&mut world);
            let query = system_state.get(&mut world);
            let item = query.get(this.entity).unwrap();
            Ok(item.flip_x)
        });

        fields.add_field_method_set("flip_x", |ctx, this, value: bool| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.flip_x = value;
            Ok(())
        });

        fields.add_field_method_set("sx", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.custom_size.get_or_insert(Vec2::ONE).x = value;
            Ok(())
        });

        fields.add_field_method_set("sy", |ctx, this, value: f32| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.custom_size.get_or_insert(Vec2::ONE).y = value;
            Ok(())
        });

        fields.add_field_method_set("flip_y", |ctx, this, value: bool| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.flip_y = value;
            Ok(())
        });

        fields.add_field_method_get("flip_y", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&Sprite>> = SystemState::new(&mut world);
            let query = system_state.get(&mut world);
            let item = query.get(this.entity).unwrap();
            Ok(item.flip_y)
        });

        fields.add_field_method_set("index", |ctx, this, value| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            if let Some(ref mut atlas) = item.texture_atlas {
                atlas.index = value;
            } else {
                warn!("sprite has no index");
            }
            Ok(())
        });

        fields.add_field_method_set("anchor", |ctx, this, value: [f32; 2]| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Sprite>> = SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            item.anchor = Anchor::Custom(Vec2::new(value[0] / 2.0, value[1] / 2.0));
            Ok(())
        });

        fields.add_field_method_set("vis", |ctx, this, value: bool| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<Query<&mut Visibility>> =
                SystemState::new(&mut world);
            let mut query = system_state.get_mut(&mut world);
            let mut item = query.get_mut(this.entity).unwrap();
            *item = if value {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            Ok(())
        });

        fields.add_field_method_get("image", |ctx, this| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<(Query<&Sprite>,)> = SystemState::new(&mut world);
            let (query,) = system_state.get_mut(&mut world);
            let item = query.get(this.entity).unwrap();
            Ok(N9Image {
                handle: item.image.clone(),
                layout: None,
            }) //.ok_or(LuaError::RuntimeError("No such image".into()))
        });
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("despawn", |ctx, this, _: ()| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            world.despawn(this.entity);
            Ok(())
        });
        // methods.add_method_mut("set_anchor", |ctx, this, _: ()| {
        // fields.add_field_method_set("anchor", |ctx, this, value: (f32, f32)| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     let mut system_state: SystemState<Query<&mut Sprite>> =
        //         SystemState::new(&mut world);
        //     let mut query = system_state.get_mut(&mut world);
        //     let mut item = query.get_mut(this.entity).unwrap();
        //     item.anchor = Anchor::Custom(value.0, value.1);
        //     Ok(())
        // });

        // methods.add_meta_method(MetaMethod::Add, |_, this, value: i32| {
        //     Ok(this.entity + value)
        // });
    }
}
