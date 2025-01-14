use bevy::{
    ecs::{
        system::SystemState,
        world::{Command},
    },
    prelude::*,
};

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{
    UserData, UserDataFields, UserDataMethods,
};
use bevy_mod_scripting::prelude::*;

pub(crate) fn plugin(_app: &mut App) {
}

#[derive(Clone)]
pub struct N9TextLoader;
impl FromLua<'_> for N9TextLoader {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

#[allow(dead_code)]
struct Print(Entity, String, TextFont);

impl Command for Print {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.0).insert((Text::new(self.1), self.2));
    }
}

impl UserData for N9TextLoader {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("default", |_ctx, _this| Ok(N9TextStyle::default()));
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("load", |ctx, _this, path: String| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            let mut system_state: SystemState<(Res<AssetServer>,)> = SystemState::new(&mut world);
            let (server,) = system_state.get(&world);
            let font: Handle<Font> = server.load(path);
            Ok(N9TextStyle(TextFont::from_font(font)))
        });

        methods.add_method_mut("print", |ctx, _this, str: String| {
            let world = ctx.get_world()?;
            let mut world = world.write();
            world.spawn(Text::new(str));
            Ok(())
        });
    }
}

#[derive(Clone, Default)]
pub struct N9TextStyle(TextFont);
impl FromLua<'_> for N9TextStyle {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

impl UserData for N9TextStyle {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "print",
            |ctx, this, (str, x, y, z): (String, Option<f32>, Option<f32>, Option<f32>)| {
                let world = ctx.get_world()?;
                let mut world = world.write();
                let x = x.unwrap_or(0.0);
                let y = y.unwrap_or(0.0);
                let z = z.unwrap_or(0.0);
                world
                    .spawn((Text::new(str), this.0.clone(), Transform::from_xyz(x, y, z)));
                Ok(())
            },
        );
    }
}
