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
        .register(
            "rectfill",
            |ctx: FunctionCallContext,
             x0: f32,
             y0: f32,
             x1: f32,
             y1: f32,
             color: Option<FillColor>| {
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
        // sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] [sheet_index] )
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
        // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y,] [turns])
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
                let id = with_pico8(&ctx, move |pico8| pico8.spr(n, pos, size, flip, turns))?;

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
                // Ok(())
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
             text: Option<ScriptValue>,
             x: Option<f32>,
             y: Option<f32>,
             c: Option<N9Color>,
             font_size: Option<f32>,
             font_index: Option<usize>| {
                let pos = with_pico8(&ctx, move |pico8| {
                    Ok(x.map(|x| Vec2::new(x, y.unwrap_or(pico8.state.draw_state.print_cursor.y))))
                })?;

                let text: Cow<'_, str> = match text.unwrap_or(ScriptValue::Unit) {
                    ScriptValue::String(s) => s,
                    ScriptValue::Float(f) => format!("{f:.4}").into(),
                    ScriptValue::Integer(x) => format!("{x}").into(),
                    // If we print a zero-length string, nothing is printed.
                    // This ensures there will be a newline.
                    _ => " ".into(),
                };

                let world_guard = ctx.world()?;
                let raid = ReflectAccessId::for_global();
                if world_guard.claim_global_access() {
                    let world = world_guard.as_unsafe_world_cell()?;
                    let world = unsafe { world.world_mut() };
                    let r = Pico8::print_world(
                        world,
                        None,
                        text.to_string(),
                        pos,
                        c,
                        font_size,
                        font_index,
                    );
                    unsafe { world_guard.release_global_access() };
                    r.map_err(|e| InteropError::external_error(Box::new(e)))
                } else {
                    Err(InteropError::cannot_claim_access(
                        raid,
                        world_guard.get_access_location(raid),
                        "print",
                    ))
                }
            },
        )
        .register(
            "sfx",
            |ctx: FunctionCallContext,
             // TODO: Need to be able to specify which audio bank.
             n: i8,
             channel: Option<u8>,
             offset: Option<u8>,
             length: Option<u8>,
             bank: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.sfx(
                        match n {
                            -2 => Ok(SfxCommand::Release),
                            -1 => Ok(SfxCommand::Stop),
                            n if n >= 0 => Ok(SfxCommand::Play(n as u8)),
                            x => Err(Error::InvalidArgument(
                                format!("sfx: expected n to be -2, -1 or >= 0 but was {x}").into(),
                            )),
                        }?,
                        channel,
                        offset,
                        length,
                        bank,
                    )
                })
            },
        )
        .register(
            "music",
            |ctx: FunctionCallContext,
             // TODO: Need to be able to specify which audio bank.
             n: i8,
             fade_ms: Option<u32>,
             channel_mask: Option<u8>,
             bank: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.music(
                        match n {
                            -2 => Ok(SfxCommand::Release),
                            -1 => Ok(SfxCommand::Stop),
                            n if n >= 0 => Ok(SfxCommand::Play(n as u8)),
                            x => Err(Error::InvalidArgument(
                                format!("sfx: expected n to be -2, -1 or >= 0 but was {x}").into(),
                            )),
                        }?,
                        fade_ms,
                        channel_mask,
                        bank,
                    )
                })
            },
        )
        .register(
            "fget",
            |ctx: FunctionCallContext, n: Option<usize>, f: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    let v = pico8.fget(n, f)?;
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
        .register("exit", |ctx: FunctionCallContext, error: Option<u8>| {
            with_pico8(&ctx, move |pico8| {
                pico8.exit(error);
                Ok(())
            })
        })
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
            "circfill",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             r: Option<u32>,
             c: Option<N9Color>| {
                let id = with_pico8(&ctx, move |pico8| {
                    pico8.circfill(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        UVec2::splat(r.unwrap_or(4)),
                        c,
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
                    if old.is_some() && new.is_none() && mode.is_none() {
                        // Set the palette.
                        pico8.state.palette = old.unwrap();
                    } else {
                        pico8.pal_map(
                            old.zip(new),
                            mode.map(|i| match i {
                                0 => PalModify::Following,
                                1 => PalModify::Present,
                                2 => PalModify::Secondary,
                                x => panic!("No such palette modify mode {x}"),
                            }),
                        );
                    }
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
        .register("shl", |_ctx: FunctionCallContext, a: f32, b: u8| {
            Pico8::shl(a, b)
        })
        .register("shr", |_ctx: FunctionCallContext, a: f32, b: u8| {
            Pico8::shr(a, b)
        })
        .register("lshr", |_ctx: FunctionCallContext, a: f32, b: u8| {
            Pico8::lshr(a, b)
        })
        .register("rotl", |_ctx: FunctionCallContext, a: f32, b: u8| {
            Pico8::rotl(a, b)
        })
        .register("rotr", |_ctx: FunctionCallContext, a: f32, b: u8| {
            Pico8::rotr(a, b)
        });

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
            |ctx: FunctionCallContext,
             x: u32,
             y: u32,
             color: Option<N9Color>,
             sprite_index: Option<usize>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.sset(UVec2::new(x, y), color, sprite_index)
                })
            },
        )
        .register(
            "sget",
            |ctx: FunctionCallContext, x: u32, y: u32, sprite_index: Option<usize>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.sget(UVec2::new(x, y), sprite_index)
                })
            },
        )
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
