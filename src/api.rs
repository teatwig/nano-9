#![allow(deprecated)]
use std::sync::{Arc, Mutex};

use bevy::{ecs::system::SystemState, prelude::*, reflect::Reflect};
use bevy_mod_scripting::api::providers::bevy_ecs::LuaEntity;
use bevy_mod_scripting::prelude::*;

use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{self, UserData, Variadic};
// use bevy_pixel_buffer::prelude::*;
#[cfg(feature = "level")]
use crate::N9LevelLoader;
use crate::{
    DropPolicy, N9AudioLoader, N9Camera, N9Color, N9Entity, N9Image, N9ImageLoader, N9Sound,
    N9Sprite, N9TextLoader, N9Var, Nano9, Nano9Palette, Nano9Screen,
};

#[derive(Clone)]
pub struct MyHandle<T: Asset + Clone>(pub Handle<T>);

// We can implement `FromLua` trait for our `Vec2` to return a copy
impl<T: Asset + Clone> FromLua<'_> for MyHandle<T> {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

impl<T: Asset + Clone> UserData for MyHandle<T> {}

pub type N9Args = Variadic<N9Arg>;

impl<'lua> IntoLua<'lua> for N9Arg {
    fn into_lua(self, lua: &'lua Lua) -> mlua::Result<Value<'lua>> {
        use N9Arg::*;
        match self {
            String(x) => x.into_lua(lua),
            Image(x) => x.into_lua(lua),
            Camera(x) => x.into_lua(lua),
            Sprite(x) => x.into_lua(lua),
            Sound(x) => x.into_lua(lua),
            Entity(x) => LuaEntity::new(x).into_lua(lua),
            DropPolicy(x) => x.into_lua(lua),
            N9Entity(x) => x.into_lua(lua),
        }
    }
}

#[derive(Clone)]
pub enum N9Arg {
    String(String),
    Image(N9Image),
    Camera(N9Camera),
    Sprite(Arc<Mutex<N9Sprite>>),
    Sound(Arc<Mutex<N9Sound>>),
    Entity(Entity),
    N9Entity(Arc<N9Entity>),
    DropPolicy(DropPolicy),
}

impl FromLua<'_> for N9Arg {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(ScriptingPlugin)
        .add_systems(
            FixedUpdate,
            script_event_handler::<LuaScriptHost<N9Args>, 0, 1>,
        )
        // .register_foreign_lua_type::<Handle<Image>>()
        .add_script_host::<LuaScriptHost<N9Args>>(PostUpdate)
        .add_api_provider::<LuaScriptHost<N9Args>>(Box::new(LuaCoreBevyAPIProvider))
        .add_api_provider::<LuaScriptHost<N9Args>>(Box::new(Nano9API))
        .add_script_handler::<LuaScriptHost<N9Args>, 0, 0>(PostUpdate);
}

#[derive(Default)]
pub struct Nano9API;

impl APIProvider for Nano9API {
    type APITarget = Mutex<Lua>;
    type ScriptContext = Mutex<Lua>;
    type DocTarget = LuaDocFragment;

    fn attach_api(&mut self, ctx: &mut Self::APITarget) -> Result<(), ScriptError> {
        // callbacks can receive any `ToLuaMulti` arguments, here '()' and
        // return any `FromLuaMulti` arguments, here a `usize`
        // check the Rlua documentation for more details
        let ctx = ctx.get_mut().unwrap();
        macro_rules! define_global {
            (fn $name:ident($($arg_name:ident : $arg_type:ty),*) $body:block) => {
                fn $name($($arg_name: $arg_type),*) {
                    println!("Executing function: {}", stringify!($name));
                    $body
                }
            };
        }
        macro_rules! define_function {
            ($ctx:ident, $name:ident, $body:expr) => {
                $ctx.globals()
                    .set(
                        stringify!($name),
                        $ctx.create_function($body)
                            .map_err(ScriptError::new_other)?,
                    )
                    .map_err(ScriptError::new_other)?;
            };
        }
        ctx.globals()
            .set("nano9", Nano9)
            .map_err(ScriptError::new_other)?;

        crate::macros::define_globals! {

            fn _set_global(ctx, (name, value): (String, Value)) {
                    ctx.globals().set(name, value)
            }

            fn _set_sprite(ctx, (name, id, drop): (String, LuaEntity, DropPolicy)) {
                    let sprite = N9Sprite {
                        entity: id.inner()?,
                        drop,
                    };
                    ctx.globals().set(name, sprite)
            }
        }

        // define_function!(ctx, _set_global, |ctx, (name, value): (String, Value)| {
        //             ctx.globals().set(name, value)
        //         });

        // define_function!(ctx, _set_sprite, |ctx, (name, id, drop): (String, LuaEntity, DropPolicy)| {
        //             let sprite = N9Sprite {
        //                 entity: id.inner()?,
        //                 drop,
        //             };
        //             ctx.globals().set(name, sprite)
        //         });

        // ctx.globals()
        //     .set(
        //         "_set_global",
        //         ctx.create_function(|ctx, (name, value): (String, Value)| {
        //             ctx.globals().set(name, value)
        //         })
        //         .map_err(ScriptError::new_other)?,
        //     )
        //     .map_err(ScriptError::new_other)?;

        // ctx.globals()
        //     .set(
        //         "_set_sprite",
        //         ctx.create_function(|ctx, (name, id, drop): (String, LuaEntity, DropPolicy)| {
        //             let sprite = N9Sprite {
        //                 entity: id.inner()?,
        //                 drop,
        //             };
        //             ctx.globals().set(name, sprite)
        //         })
        //         .map_err(ScriptError::new_other)?,
        //     )
        //     .map_err(ScriptError::new_other)?;

        // // A Lua binding for a set palette function.
        // ctx.globals()
        //     .set(
        //         "setpal",
        //         ctx.create_function(|ctx, img: N9Image| {
        //             let world = ctx.get_world()?;
        //             let mut world = world.write();
        //             world.insert_resource(Nano9Palette(img.handle.clone()));
        //             Ok(())
        //         })
        //         .map_err(ScriptError::new_other)?,
        //     )
        //     .map_err(ScriptError::new_other)?;

        // // Macro #1, better.
        // define_function!(ctx, setpal, |ctx, img: N9Image| {
        //     let world = ctx.get_world()?;
        //     let mut world = world.write();
        //     world.insert_resource(Nano9Palette(img.handle.clone()));
        //     Ok(())
        // });

        // Macro #2, best!
        crate::macros::define_globals! {
            fn setpal(ctx, img: N9Image) {
                let world = ctx.get_world()?;
                let mut world = world.write();
                world.insert_resource(Nano9Palette(img.handle.clone()));
                Ok(())
            }
            // ...
        }

        Ok(())
    }

    fn register_with_app(&self, app: &mut App) {
        // app.register_type::<Settings>();
    }
}
