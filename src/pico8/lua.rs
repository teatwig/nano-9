use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};

use bevy_mod_scripting::core::{
        bindings::{
            access_map::ReflectAccessId,
            function::{
                from::{FromScript, Val},
                into_ref::IntoScriptRef,
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            ReflectReference,
        },
        error::InteropError,
    };

use crate::{
    pico8::{
        Error, Pico8, PropBy, SfxCommand, Spr, PalModify,
    }, DropPolicy, N9Color, N9Entity,
};

#[cfg(feature = "level")]
use std::collections::HashMap;

pub(crate) fn with_system_param<S: SystemParam + 'static,X, E: std::error::Error + Send + Sync + 'static>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut S::Item<'_, '_>) -> Result<X, E>,
) -> Result<X, InteropError>
{
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

fn with_pico8<X>(
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
        .register(
            "btnp",
            |ctx: FunctionCallContext, b: Option<u8>, p: Option<u8>| {
                with_pico8(&ctx, |pico8| pico8.btnp(b, p))
            },
        )
        .register(
            "btn",
            |ctx: FunctionCallContext, b: Option<u8>, p: Option<u8>| {
                with_pico8(&ctx, |pico8| pico8.btn(b, p))
            },
        )
        .register("cls", |ctx: FunctionCallContext, c: Option<N9Color>| {
            with_pico8(&ctx, |pico8| pico8.cls(c))
        })
        .register(
            "pset",
            |ctx: FunctionCallContext, x: u32, y: u32, color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.pset(UVec2::new(x, y), color);
                    Ok(())
                })
            },
        )
        .register(
            "rectfill",
            |ctx: FunctionCallContext,
             x0: f32,
             y0: f32,
             x1: f32,
             y1: f32,
             color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rectfill(Vec2::new(x0, y0), Vec2::new(x1, y1), color);
                    Ok(())
                })
            },
        )
        .register(
            "rect",
            |ctx: FunctionCallContext,
             x0: f32,
             y0: f32,
             x1: f32,
             y1: f32,
             color: Option<N9Color>| {
                with_pico8(&ctx, |pico8| {
                    // We want to ignore out of bounds errors specifically but possibly not others.
                    // Ok(pico8.pset(x, y, color)?)
                    let _ = pico8.rect(Vec2::new(x0, y0), Vec2::new(x1, y1), color);
                    Ok(())
                })
            },
        )
        // sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y], [sheet_index] )
        .register(
            "sspr",
            |ctx: FunctionCallContext,
             sx: f32,
             sy: f32,
             sw: f32,
             sh: f32,
             dx: f32,
             dy: f32,
             dw: Option<f32>,
             dh: Option<f32>,
             flip_x: Option<bool>,
             flip_y: Option<bool>,
             sheet_index: Option<usize>| {
                let sprite_rect = Rect::new(sx, sy, sx + sw, sy + sh);
                let pos = Vec2::new(dx, dy);
                let size = dw
                    .or(dh)
                    .is_some()
                    .then(|| Vec2::new(dw.unwrap_or(sw), dh.unwrap_or(sh)));
                let flip = (flip_x.is_some() || flip_y.is_some())
                    .then(|| BVec2::new(flip_x.unwrap_or(false), flip_y.unwrap_or(false)));
                // We get back an entity. Not doing anything with it here yet.
                let _id = with_pico8(&ctx, move |pico8| {
                    pico8.sspr(sprite_rect, pos, size, flip, sheet_index)
                })?;
                Ok(())
            },
        )
        // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
        .register(
            "spr",
            |ctx: FunctionCallContext,
             n: ScriptValue,
             x: Option<f32>,
             y: Option<f32>,
             w: Option<f32>,
             h: Option<f32>,
             flip_x: Option<bool>,
             flip_y: Option<bool>,
             turns: Option<f32>| {
                let pos = Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0));
                let flip = (flip_x.is_some() || flip_y.is_some())
                    .then(|| BVec2::new(flip_x.unwrap_or(false), flip_y.unwrap_or(false)));
                let size = w
                    .or(h)
                    .is_some()
                    .then(|| Vec2::new(w.unwrap_or(1.0), h.unwrap_or(1.0)));

                // We get back an entity. Not doing anything with it here yet.
                let n = Spr::from_script(n, ctx.world()?)?;
                let _id = with_pico8(&ctx, move |pico8| pico8.spr(n, pos, size, flip, turns))?;
                Ok(())
            },
        )
        // map( celx, cely, sx, sy, celw, celh, [layer] )
        .register(
            "map",
            |ctx: FunctionCallContext,
             celx: Option<u32>,
             cely: Option<u32>,
             sx: Option<f32>,
             sy: Option<f32>,
             celw: Option<u32>,
             celh: Option<u32>,
             layer: Option<u8>,
             map_index: Option<usize>| {
                let id = with_pico8(&ctx, move |pico8| {
                    pico8.map(
                        UVec2::new(celx.unwrap_or(0), cely.unwrap_or(0)),
                        Vec2::new(sx.unwrap_or(0.0), sy.unwrap_or(0.0)),
                        UVec2::new(celw.unwrap_or(16), celh.unwrap_or(16)),
                        layer,
                        map_index,
                    )
                })?;

                let entity = N9Entity {
                    entity: id,
                    drop: DropPolicy::Nothing,
                };
                let world = ctx.world()?;
                let reference = {
                    let allocator = world.allocator();
                    let mut allocator = allocator.write();
                    ReflectReference::new_allocated(entity, &mut allocator)
                };
                ReflectReference::into_script_ref(reference, world)
            },
        )
        .register(
            "print",
            |ctx: FunctionCallContext,
             text: Option<String>,
             x: Option<f32>,
             y: Option<f32>,
             c: Option<N9Color>| {
                with_pico8(&ctx, move |pico8| {
                    let pos =
                        x.map(|x| Vec2::new(x, y.unwrap_or(pico8.state.draw_state.print_cursor.y)));
                    pico8.print(text.as_deref().unwrap_or(""), pos, c)
                })
            },
        )
        .register(
            "sfx",
            |ctx: FunctionCallContext,
             n: i8,
             channel: Option<u8>,
             offset: Option<u8>,
             length: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.sfx(
                        match n {
                            -2 => Ok(SfxCommand::Release),
                            -1 => Ok(SfxCommand::Stop),
                            n if n >= 0 => Ok(SfxCommand::Play(n as u8)),
                            x => {
                                // Maybe we should let Lua errors pass through.
                                // Err(LuaError::BadArgument {
                                //     to: Some("sfx".into()),
                                //     pos: 0,
                                //     name: Some("n".into()),
                                //     cause: std::sync::Arc::new(
                                // })
                                Err(Error::InvalidArgument(
                                    format!("sfx: expected n to be -2, -1 or >= 0 but was {x}")
                                        .into(),
                                ))
                            }
                        }?,
                        channel,
                        offset,
                        length,
                        None,
                    )
                })
            },
        )
        .register(
            "fget",
            |ctx: FunctionCallContext, n: Option<usize>, f: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    let v = pico8.fget(n, f);
                    Ok(if f.is_some() {
                        ScriptValue::Bool(v == 1)
                    } else {
                        ScriptValue::Integer(v as i64)
                    })
                })
            },
        )
        .register(
            "fset",
            |ctx: FunctionCallContext, n: usize, f_or_v: u8, v: Option<u8>| {
                let (f, v) = v.map(|v| (Some(f_or_v), v)).unwrap_or((None, f_or_v));
                with_pico8(&ctx, move |pico8| {
                    pico8.fset(n, f, v);
                    Ok(())
                })
            },
        )
        .register(
            "mget",
            |ctx: FunctionCallContext,
             x: f32,
             y: f32,
             map_index: Option<usize>,
             layer_index: Option<usize>| {
                with_pico8(&ctx, move |pico8| {
                    Ok(pico8.mget(Vec2::new(x, y), map_index, layer_index))
                })
            },
        )
        .register(
            "mset",
            |ctx: FunctionCallContext,
             x: f32,
             y: f32,
             v: usize,
             map_index: Option<usize>,
             layer_index: Option<usize>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.mset(Vec2::new(x, y), v, map_index, layer_index)
                })
            },
        )
        .register("sub", |s: String, start: isize, end: Option<isize>| {
            Pico8::sub(&s, start, end)
        })
        .register("time", |ctx: FunctionCallContext| {
            with_pico8(&ctx, move |pico8| Ok(pico8.time()))
        })
        .register("rnd", |ctx: FunctionCallContext, value: ScriptValue| {
            with_pico8(&ctx, move |pico8| Ok(pico8.rnd(value)))
        })
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
            "circfill",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             r: Option<u32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.circfill(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        UVec2::splat(r.unwrap_or(4)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "circ",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             r: Option<u32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.circ(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        UVec2::splat(r.unwrap_or(4)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "ovalfill",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.ovalfill(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "oval",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<N9Color>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.oval(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        )
        .register(
            "pal",
            |ctx: FunctionCallContext, old: Option<usize>, new: Option<usize>, mode: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.pal(old.zip(new), mode.map(|i| match i {
                        0 => PalModify::Following,
                        1 => PalModify::Present,
                        2 => PalModify::Secondary,
                        x => panic!("No such palette modify mode {x}"),
                    }));
                    Ok(())
                })
            },
        )
        .register(
            "palt",
            |ctx: FunctionCallContext, color: Option<usize>, transparency: Option<bool>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.palt(color, transparency);
                    Ok(())
                })
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
        .register(
            "sset",
            |ctx: FunctionCallContext, id: i64, sprite_index: usize| {
                let id = Entity::from_bits(id as u64);
                with_pico8(&ctx, move |pico8| {
                    pico8.sset(id, sprite_index);
                    Ok(())
                })
            },
        )
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
        })
        ;
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
