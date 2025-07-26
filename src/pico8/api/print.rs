use super::*;

impl super::Pico8<'_, '_> {
    pub fn cursor(&mut self, pos: Option<Vec2>, color: Option<PColor>) -> (Vec2, PColor) {
        let last_pos = self.state.draw_state.print_cursor;
        let last_color = self.state.draw_state.pen;
        if let Some(pos) = pos.map(|p| pixel_snap(self.state.draw_state.apply_camera_delta(p))) {
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
            .map(|p| pixel_snap(state.draw_state.apply_camera_delta(p)))
            .unwrap_or_else(|| {
                pixel_snap(Vec2::new(
                    state.draw_state.print_cursor.x,
                    state.draw_state.print_cursor.y,
                ))
            });
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
