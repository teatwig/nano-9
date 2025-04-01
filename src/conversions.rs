use bevy_mod_scripting::{
    core::{
        asset::{AssetPathToLanguageMapper, Language, ScriptAsset, ScriptAssetSettings},
        bindings::{
            access_map::ReflectAccessId,
            function::{
                from::FromScript,
                into_ref::IntoScriptRef,
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            ReflectReference, WorldAccessGuard,
        },
        error::InteropError,
    },
    lua::mlua::prelude::LuaError,
};

use bevy::prelude::*;
use std::{any::TypeId, borrow::Borrow, collections::HashMap, fmt::Display, hash::Hash, sync::Arc};

fn script_value_to_f32(value: &ScriptValue) -> Option<f32> {
    match value {
        ScriptValue::Float(f) => Some(*f as f32),
        ScriptValue::Integer(i) => Some(*i as f32),
        _ => None,
    }
}

pub struct f32Value;
impl FromScript for f32Value {
    type This<'w> = f32;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Float(f) => Ok(f as f32),
            ScriptValue::Integer(i) => Ok(i as f32),
            x => Err(InteropError::value_mismatch(TypeId::of::<f32>(), x)),
        }
    }
}

pub struct Vec2Value;
impl FromScript for Vec2Value {
    type This<'w> = Vec2;
    fn from_script(
        value: ScriptValue,
        world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::List(l) => {
                let n = l.len();
                let mut i = l.into_iter();
                if n == 2 {
                    let x = f32Value::from_script(i.next().unwrap(), world.clone())?;
                    let y = f32Value::from_script(i.next().unwrap(), world)?;
                    Ok(Vec2::new(x, y))
                } else {
                    Err(InteropError::length_mismatch(2, n))
                }
            }
            ScriptValue::Map(mut v) => {
                let x = f32Value::from_script(remover(&mut v, "x")?, world.clone())?;
                let y = f32Value::from_script(remover(&mut v, "y")?, world)?;
                Ok(Vec2::new(x, y))
            }
            _ => Err(InteropError::impossible_conversion(TypeId::of::<Vec2>())),
        }
    }
}

fn getr<'a, K, V, Q>(map: &'a HashMap<K, V>, k: &Q) -> Result<&'a V, InteropError>
where
    K: Borrow<Q> + Hash + Eq,
    Q: Hash + Eq + ?Sized + Display,
{
    map.get(k)
        .ok_or_else(|| InteropError::string_type_mismatch(k.to_string(), None))
}

fn remover<'a, K, V, Q>(map: &'a mut HashMap<K, V>, k: &Q) -> Result<V, InteropError>
where
    K: Borrow<Q> + Hash + Eq,
    Q: Hash + Eq + ?Sized + Display,
{
    map.remove(k)
        .ok_or_else(|| InteropError::string_type_mismatch(k.to_string(), None))
}

pub struct RectValue;
impl FromScript for RectValue {
    type This<'w> = Rect;
    fn from_script(
        value: ScriptValue,
        world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::List(l) => {
                let n = l.len();
                let mut i = l.into_iter();
                if n == 2 {
                    let a = Vec2Value::from_script(i.next().unwrap(), world.clone())?;
                    let b = Vec2Value::from_script(i.next().unwrap(), world)?;
                    Ok(Rect::from_corners(a, b))
                } else if n == 4 {
                    let x0 = f32Value::from_script(i.next().unwrap(), world.clone())?;
                    let y0 = f32Value::from_script(i.next().unwrap(), world.clone())?;
                    let x1 = f32Value::from_script(i.next().unwrap(), world.clone())?;
                    let y1 = f32Value::from_script(i.next().unwrap(), world)?;
                    Ok(Rect::from_corners(Vec2::new(x0, y0), Vec2::new(x1, y1)))
                } else {
                    Err(InteropError::impossible_conversion(TypeId::of::<Rect>()))
                }
            }
            ScriptValue::Map(mut v) => {
                let min = Vec2Value::from_script(remover(&mut v, "min")?, world.clone())?;
                let max = Vec2Value::from_script(remover(&mut v, "max")?, world)?;
                Ok(Rect { min, max })
            }
            _ => Err(InteropError::impossible_conversion(TypeId::of::<Rect>())),
        }
    }
}
