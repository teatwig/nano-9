use bevy::prelude::*;
use std::{any::TypeId, sync::Arc};

use crate::{ValueExt, pico8::{Error, PalMap}};
use bevy_mod_scripting::{
    core::docgen::typed_through::{ThroughTypeInfo, TypedThrough},
    core::{
        bindings::{
            function::from::FromScript, script_value::ScriptValue, IntoScript, WorldAccessGuard,
        },
        error::InteropError,
    },
    lua::mlua::{
        self, prelude::LuaError, FromLua, Lua, UserData, UserDataFields, UserDataMethods, Value,
    },
    GetTypeDependencies,
};

#[derive(Debug, Clone, Copy, Reflect, GetTypeDependencies)]
pub enum PColor {
    Palette(usize),
    Color(LinearRgba),
}

impl PColor {
    fn write_color(&self, palette: &[[u8; 4]], pal_map: &PalMap, pixel_bytes: &mut [u8; 4]) -> Result<(), Error> {
        match self {
            PColor::Palette(i) => {
                pal_map.write_color(palette, *i as u8, pixel_bytes)
            }
            PColor::Color(c) => {
                let arr = c.to_u8_array();
                pixel_bytes.copy_from_slice(&arr);
                Ok(())
            }
        }
    }
}

impl TypedThrough for PColor {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<PColor as bevy::reflect::Typed>::type_info())
    }
}

impl FromScript for PColor {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => Ok(PColor::Palette(n as usize)),
            ScriptValue::Float(n) => Ok(PColor::Palette(n as usize)),
            _ => Err(InteropError::impossible_conversion(TypeId::of::<PColor>())),
        }
    }
}

impl IntoScript for PColor {
    fn into_script(self, _world: WorldAccessGuard<'_>) -> Result<ScriptValue, InteropError> {
        match self {
            PColor::Palette(n) => Ok(ScriptValue::Integer(n as i64)),
            PColor::Color(n) => {
                let a = n.to_u8_array();
                Ok(ScriptValue::List(
                    a.into_iter()
                        .map(|x| ScriptValue::Integer(x as i64))
                        .collect(),
                ))
            }
        }
    }
}

impl From<Color> for PColor {
    fn from(c: Color) -> Self {
        PColor::Color(c.into())
    }
}

impl FromLua for PColor {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        fn bad_arg(s: &str) -> LuaError {
            LuaError::WithContext {
                context: format!("unable to convert {s:?} field to f32."),
                cause: Arc::new(LuaError::UserDataTypeMismatch),
            }
        }
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            Value::Integer(n) => Ok(PColor::Palette(n as usize)),
            Value::Number(n) => Ok(PColor::Palette(n as usize)),
            Value::Table(t) => {
                let l = t.len().unwrap_or(0);
                if t.contains_key("r")? && t.contains_key("g")? && t.contains_key("b")? {
                    Ok(PColor::Color(LinearRgba::new(
                        t.get("r")
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("r")))?,
                        t.get("g")
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("g")))?,
                        t.get("b")
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("b")))?,
                        t.get("a").map(|x: Value| x.as_f32().unwrap_or(1.0))?,
                    )))
                } else if l >= 3 {
                    Ok(PColor::Color(LinearRgba::new(
                        t.get(1)
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("r")))?,
                        t.get(2)
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("g")))?,
                        t.get(3)
                            .and_then(|x: Value| x.to_f32().ok_or(bad_arg("b")))?,
                        t.get(4).map(|x: Value| x.as_f32().unwrap_or(1.0))?,
                    )))
                } else {
                    Err(LuaError::UserDataTypeMismatch)
                }
            }
            _ => Err(LuaError::UserDataTypeMismatch),
        }
    }
}
