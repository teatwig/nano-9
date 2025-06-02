use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn cursor(&mut self, pos: Option<Vec2>, color: Option<PColor>) -> (Vec2, PColor) {
        let last_pos = self.state.draw_state.print_cursor;
        let last_color = self.state.draw_state.pen;
        if let Some(pos) = pos.map(|p| self.state.draw_state.apply_camera_delta(p)) {
            self.state.draw_state.print_cursor = pos;
        }
        if let Some(color) = color {
            self.state.draw_state.pen = color;
        }
        (last_pos, last_color)
    }
    /// print(text, [x,] [y,] [color,] [font_size])
    ///
    /// Print the given text. The Lua `print()` function will return the new x
    /// value. This function only returns the entity. To recover the new x
    /// value, one can call the `cursor().x` function.
    pub fn print(
        &mut self,
        text: impl Into<String>,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let text = text.into();
        let id = self.commands.spawn_empty().id();
        self.commands.queue(move |world: &mut World| {
            if let Err(e) =
                Self::print_world(world, Some(id), text, pos, color, font_size, font_index)
            {
                warn!("print error {e}");
            }
        });
        Ok(id)
    }

    pub(crate) fn print_world(
        world: &mut World,
        dest: Option<Entity>,
        text: String,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<f32, Error> {
        let (id, add_newline) =
            Self::pre_print_world(world, dest, text, pos, color, font_size, font_index)?;
        world
            .run_system_cached(bevy::text::update_text2d_layout)
            .expect("update_text2d_layout");
        world
            .run_system_cached_with(Self::post_print_world, (id, add_newline))
            .expect("post_print_world")
    }

    fn post_print_world(
        In((id, add_newline)): In<(Entity, bool)>,
        query: Query<(&Transform, &TextLayoutInfo)>,
        mut state: ResMut<Pico8State>,
    ) -> Result<f32, Error> {
        let (transform, text_layout) = query
            .get(id)
            .map_err(|_| Error::NoSuch("text layout".into()))?;
        let pos = &transform.translation;
        if add_newline {
            state.draw_state.print_cursor.x = pos.x;
            state.draw_state.print_cursor.y = negate_y(pos.y) + text_layout.size.y;
        } else {
            state.draw_state.print_cursor.x = pos.x + text_layout.size.x;
        }
        state.draw_state.mark_drawn();
        Ok(pos.x + text_layout.size.x)
    }

    fn pre_print_world(
        world: &mut World,
        entity: Option<Entity>,
        mut text: String,
        pos: Option<Vec2>,
        color: Option<N9Color>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<(Entity, bool), Error> {
        let assets = world
            .get_resource::<Assets<Pico8Asset>>()
            .expect("Pico8Assets");
        let state = world.get_resource::<Pico8State>().expect("Pico8State");
        let pico8_handle = world.get_resource::<Pico8Handle>().expect("Pico8Handle");
        let pico8_asset = assets
            .get(&pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))?;
        let font = pico8_asset
            .font
            .get(font_index.unwrap_or(0))
            .ok_or(Error::NoSuch("font".into()))?
            .handle
            .clone();

        let c = pico8_asset.get_color(
            color
                .unwrap_or(N9Color::Pen)
                .into_pcolor(&state.draw_state.pen),
            state.palette,
        )?;
        // XXX: Should the camera delta apply to the print cursor position?
        let pos = pos
            .map(|p| state.draw_state.apply_camera_delta(p))
            .unwrap_or_else(|| {
                Vec2::new(
                    state.draw_state.print_cursor.x,
                    state.draw_state.print_cursor.y,
                )
            });
        // pos =
        let clearable = Clearable::default();
        let add_newline = if text.ends_with('\0') {
            text.pop();
            false
        } else {
            true
        };
        let font_size = font_size.unwrap_or(5.0);
        let z = clearable.suggest_z();
        let id = entity.unwrap_or_else(|| world.spawn_empty().id());
        world.entity_mut(id).insert((
            Name::new("print"),
            Transform::from_xyz(pos.x, negate_y(pos.y), z),
            Text2d::new(text),
            Visibility::default(),
            TextColor(c),
            TextFont {
                font,
                font_smoothing: bevy::text::FontSmoothing::None,
                font_size,
            },
            Anchor::TopLeft,
            clearable,
        ));
        Ok((id, add_newline))
    }

    pub fn sub(string: &str, start: isize, end: Option<isize>) -> String {
        let count = string.chars().count() as isize;
        let start = if start < 0 {
            (count - start - 1) as usize
        } else {
            (start - 1) as usize
        };
        match end {
            Some(end) => {
                let end = if end < 0 {
                    (count - end) as usize
                } else {
                    end as usize
                };
                if start <= end {
                    string.chars().skip(start).take(end - start).collect()
                    // BUG: This cuts unicode boundaries.
                    // Ok(string[start..end].to_string())
                } else {
                    String::new()
                }
            }
            None => string.chars().skip(start).collect(),
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::core::{
        bindings::{
            access_map::ReflectAccessId,
            function::{
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            IntoScript,
        },
        error::InteropError,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
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
                        Ok(x.map(|x| {
                            Vec2::new(x, y.unwrap_or(pico8.state.draw_state.print_cursor.y))
                        }))
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
                "_cursor",
                |ctx: FunctionCallContext,
                 x: Option<f32>,
                 y: Option<f32>,
                 color: Option<PColor>| {
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
            .register("sub", |s: String, start: isize, end: Option<isize>| {
                Pico8::sub(&s, start, end)
            });
    }
}
