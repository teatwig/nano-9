use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
    bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
    error::InteropError,
};

use crate::pico8::Gfx;

use std::any::TypeId;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[derive(Debug, Clone, Reflect)]
pub enum PropBy {
    Pos(Vec2),
    Rect(Rect),
    Name(Cow<'static, str>),
}

impl From<Vec2> for PropBy {
    fn from(v: Vec2) -> Self {
        PropBy::Pos(v)
    }
}

impl From<String> for PropBy {
    fn from(v: String) -> Self {
        PropBy::Name(v.into())
    }
}

// impl Default for PropBy {
//     fn default() -> Self {
//         PropBy::Pos(Vec2::ZERO)
//     }
// }

#[cfg(feature = "scripting")]
impl TypedThrough for PropBy {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<PropBy as bevy::reflect::Typed>::type_info())
    }
}

#[cfg(feature = "scripting")]
impl FromScript for PropBy {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::String(n) => Ok(PropBy::Name(n)),
            ScriptValue::List(l) => {
                let x = l.first().and_then(ValueExt::to_f32).unwrap_or(0.0);
                let y = l.get(1).and_then(ValueExt::to_f32).unwrap_or(0.0);
                Ok(PropBy::Pos(Vec2::new(x, y)))
            }
            ScriptValue::Map(v) => {
                let x = v.get("x").and_then(ValueExt::to_f32).unwrap_or(0.0);
                let y = v.get("y").and_then(ValueExt::to_f32).unwrap_or(0.0);
                let w = v.get("width").and_then(ValueExt::to_f32);
                let h = v.get("height").and_then(ValueExt::to_f32);
                if w.is_some() && h.is_some() {
                    Ok(PropBy::Rect(Rect::from_corners(
                        Vec2::new(x, y),
                        Vec2::new(x + w.unwrap(), y + h.unwrap()),
                    )))
                } else {
                    Ok(PropBy::Pos(Vec2::new(x, y)))
                }
            }
            _ => Err(InteropError::impossible_conversion(TypeId::of::<PropBy>())),
        }
    }
}

impl super::Pico8<'_, '_> {
    /// Get properties
    pub fn props(&self, id: Entity) -> Result<tiled::Properties, Error> {
        self.tiled.props(id)
    }

    /// Get properties
    pub fn mgetp(
        &self,
        prop_by: PropBy,
        map_index: Option<usize>,
        layer_index: Option<usize>,
    ) -> Option<tiled::Properties> {
        let map: &Map = self.sprite_map(map_index).ok()?;
        match *map {
            Map::P8(ref _map) => None,

            #[cfg(feature = "level")]
            Map::Level(ref map) => self.tiled.mgetp(map, prop_by, map_index, layer_index),
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{pico8::lua::with_pico8, DropPolicy, N9Entity};

    use bevy_mod_scripting::core::{
        bindings::{
            access_map::ReflectAccessId,
            function::{
                from::FromScript,
                into_ref::IntoScriptRef,
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            IntoScript, ReflectReference,
        },
        error::InteropError,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "mgetp",
                |ctx: FunctionCallContext,
                 prop_by: ScriptValue,
                 map_index: Option<usize>,
                 layer_index: Option<usize>| {
                    let prop_by = PropBy::from_script(prop_by, ctx.world()?)?;
                    with_pico8(&ctx, move |pico8| {
                        Ok(pico8
                            .mgetp(prop_by, map_index, layer_index)
                            .map(|p| from_properties(&p)))
                    })
                },
            )
            .register("props", |ctx: FunctionCallContext, id: i64| {
                let id = Entity::from_bits(id as u64);
                with_pico8(&ctx, move |pico8| {
                    pico8.props(id).map(|p| from_properties(&p))
                })
            });
    }

    fn from_properties(properties: &tiled::Properties) -> ScriptValue {
        let map: HashMap<String, ScriptValue> = properties
            .iter()
            .flat_map(|(name, value)| from_property(value).map(|v| (name.to_owned(), v)))
            .collect();
        ScriptValue::Map(map)
    }

    fn from_property(v: &tiled::PropertyValue) -> Option<ScriptValue> {
        use tiled::PropertyValue;
        match v {
            PropertyValue::BoolValue(v) => Some(ScriptValue::Bool(*v)),
            PropertyValue::FloatValue(f) => Some(ScriptValue::Float(*f as f64)),
            PropertyValue::IntValue(i) => Some(ScriptValue::Integer(*i as i64)),
            PropertyValue::ColorValue(_color) => None,
            PropertyValue::StringValue(s) => Some(ScriptValue::String(s.to_owned().into())),
            PropertyValue::FileValue(f) => Some(ScriptValue::String(f.to_owned().into())),
            PropertyValue::ObjectValue(_number) => None,
            PropertyValue::ClassValue {
                property_type,
                properties,
            } => Some(ScriptValue::Map(
                [(property_type.to_owned(), from_properties(properties))]
                    .into_iter()
                    .collect(),
            )),
        }
    }
}
