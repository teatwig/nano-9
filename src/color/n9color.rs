use super::PColor;
use bevy::prelude::*;
use std::{any::TypeId, sync::Arc};

use crate::ValueExt;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    core::docgen::typed_through::{ThroughTypeInfo, TypedThrough},
    core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        error::InteropError,
    },
    lua::mlua::{
        self, prelude::LuaError, FromLua, Lua, UserData, UserDataFields, UserDataMethods, Value,
    },
    GetTypeDependencies,
};

#[derive(Debug, Clone, Copy, Reflect)]
#[cfg_attr(feature = "scripting", derive(GetTypeDependencies))]
pub enum N9Color {
    Pen,
    PColor(PColor)
}

impl N9Color {
    pub fn into_pcolor(&self, pen_color: &PColor) -> PColor {
        match self {
            N9Color::Pen => *pen_color,
            N9Color::PColor(p) => *p
        }
    }
}

impl From<PColor> for N9Color {
    fn from(c: PColor) -> Self {
        N9Color::PColor(c)
    }
}

impl From<usize> for N9Color {
    fn from(n: usize) -> Self {
        N9Color::PColor(n.into())
    }
}

#[cfg(feature = "scripting")]
impl TypedThrough for N9Color {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<N9Color as bevy::reflect::Typed>::type_info())
    }
}

#[cfg(feature = "scripting")]
impl FromScript for N9Color {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => Ok(N9Color::PColor((n as usize).into())),
            ScriptValue::Unit => Ok(N9Color::Pen),
            _ => Err(InteropError::impossible_conversion(TypeId::of::<N9Color>())),
        }
    }
}

impl From<Option<usize>> for N9Color {
    fn from(c: Option<usize>) -> Self {
        match c {
            Some(index) => N9Color::PColor(index.into()),
            None => N9Color::Pen,
        }
    }
}

impl From<Color> for N9Color {
    fn from(c: Color) -> Self {
        N9Color::PColor(c.into())
    }
}
