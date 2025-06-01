use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};

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

use crate::{
    color::FillColor,
    pico8::{Error, PalModify, Pico8, SfxCommand, Spr},
    DropPolicy, N9Color, N9Entity, PColor,
};

use std::borrow::Cow;
#[cfg(feature = "level")]
use std::collections::HashMap;

pub(crate) fn with_system_param<
    S: SystemParam + 'static,
    X,
    E: std::error::Error + Send + Sync + 'static,
>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut S::Item<'_, '_>) -> Result<X, E>,
) -> Result<X, InteropError> {
    let world_guard = ctx.world()?;
    let raid = ReflectAccessId::for_global();
    if world_guard.claim_global_access() {
        let world = world_guard.as_unsafe_world_cell()?;
        let world = unsafe { world.world_mut() };
        let mut system_state: SystemState<S> = SystemState::new(world);
        let r = {
            let mut pico8 = system_state.get_mut(world);
            f(&mut pico8)
        };
        system_state.apply(world);
        unsafe { world_guard.release_global_access() };
        r.map_err(|e| InteropError::external_error(Box::new(e)))
    } else {
        Err(InteropError::cannot_claim_access(
            raid,
            world_guard.get_access_location(raid),
            "with_system_param",
        ))
    }
}

pub(crate) fn with_pico8<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Pico8) -> Result<X, Error>,
) -> Result<X, InteropError> {
    with_system_param::<Pico8, X, Error>(ctx, f)
}

pub(crate) fn plugin(app: &mut App) {
    // callbacks can receive any `ToLuaMulti` arguments, here '()' and
    // return any `FromLuaMulti` arguments, here a `usize`
    // check the Rlua documentation for more details
    let world = app.world_mut();

    NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
        .register("cls", |ctx: FunctionCallContext, c: Option<PColor>| {
            with_pico8(&ctx, |pico8| pico8.cls(c))
        })
        .register(
            "pset",
            |ctx: FunctionCallContext, x: u32, y: u32, color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.pset(UVec2::new(x, y), color.unwrap_or(N9Color::Pen));
                    Ok(())
                })
            },
        )
        .register("exit", |ctx: FunctionCallContext, error: Option<u8>| {
            with_pico8(&ctx, move |pico8| {
                pico8.exit(error);
                Ok(())
            })
        })
        .register("sub", |s: String, start: isize, end: Option<isize>| {
            Pico8::sub(&s, start, end)
        })
        .register("time", |ctx: FunctionCallContext| {
            with_pico8(&ctx, move |pico8| Ok(pico8.time()))
        })
        .register(
            "rnd",
            |ctx: FunctionCallContext, value: Option<ScriptValue>| {
                with_pico8(&ctx, move |pico8| Ok(pico8.rnd(value)))
            },
        )
        .register("srand", |ctx: FunctionCallContext, value: u64| {
            with_pico8(&ctx, move |pico8| {
                pico8.srand(value);
                Ok(())
            })
        })
        .register(
            "_camera",
            |ctx: FunctionCallContext, x: Option<f32>, y: Option<f32>| {
                with_pico8(&ctx, move |pico8| {
                    let arg = x.map(|x| Vec2::new(x, y.unwrap_or(0.0)));
                    Ok(pico8.camera(arg))
                })
                .map(|last_pos| (last_pos.x, last_pos.y))
            },
        )
        .register(
            "line",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.line(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "color",
            |ctx: FunctionCallContext, color: Option<PColor>| {
                with_pico8(&ctx, move |pico8| pico8.color(color))
            },
        )
        .register("fillp", |ctx: FunctionCallContext, pattern: Option<u16>| {
            with_pico8(&ctx, move |pico8| Ok(pico8.fillp(pattern)))
        })
        .register(
            "_cursor",
            |ctx: FunctionCallContext, x: Option<f32>, y: Option<f32>, color: Option<PColor>| {
                let (last_pos, last_color) = with_pico8(&ctx, move |pico8| {
                    let pos = if x.is_some() || y.is_some() {
                        Some(Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0)))
                    } else {
                        None
                    };

                    Ok(pico8.cursor(pos, color))
                })?;
                Ok(ScriptValue::List(vec![
                    ScriptValue::Float(last_pos.x as f64),
                    ScriptValue::Float(last_pos.y as f64),
                    last_color.into_script(ctx.world()?)?,
                ]))
            },
        )
        .register("peek", |ctx: FunctionCallContext, addr: usize| {
            with_pico8(&ctx, move |pico8| pico8.peek(addr))
        })
        .register(
            "poke",
            |ctx: FunctionCallContext, addr: usize, value: u8| {
                with_pico8(&ctx, move |pico8| pico8.poke(addr, value))
            },
        )
        .register(
            "stat",
            |ctx: FunctionCallContext, n: u8, value: Option<u8>| {
                with_pico8(&ctx, move |pico8| pico8.stat(n, value))
            },
        )
        ;

    #[cfg(feature = "level")]
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
        })
        // .register(
        //     "sset",
        //     |ctx: FunctionCallContext, id: i64, sprite_index: usize| {
        //         let id = Entity::from_bits(id as u64);
        //         with_pico8(&ctx, move |pico8| {
        //             pico8.sset(id, sprite_index);
        //             Ok(())
        //         })
        //     },
        // )
        .register("ent", |_ctx: FunctionCallContext, id: i64| {
            let id = Entity::from_bits(id as u64);
            // let entity = N9Entity {
            //     entity: id,
            //     drop: DropPolicy::Nothing,
            // };
            // let world = ctx.world()?;
            // let reference = {
            //     let allocator = world.allocator();
            //     let mut allocator = allocator.write();
            //     ReflectReference::new_allocated(entity, &mut allocator)
            // };
            // ReflectReference::into_script_ref(reference, world)
            // Ok(Val::new(0.0))
            // Ok(0.0)
            Val(id)
        })
        .register("print_ent", |_ctx: FunctionCallContext, id: Val<Entity>| {
            info!("print id {}", &id.0);
        });
}

#[cfg(feature = "level")]
fn from_properties(properties: &tiled::Properties) -> ScriptValue {
    let map: HashMap<String, ScriptValue> = properties
        .iter()
        .flat_map(|(name, value)| from_property(value).map(|v| (name.to_owned(), v)))
        .collect();
    ScriptValue::Map(map)
}

#[cfg(feature = "level")]
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
