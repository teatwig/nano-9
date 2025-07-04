use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn rectfill(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<impl Into<FillColor>>,
    ) -> Result<Entity, Error> {
        let upper_left = pixel_snap(self.state.draw_state.apply_camera_delta(upper_left));
        let lower_right = pixel_snap(self.state.draw_state.apply_camera_delta(lower_right));
        let size = (lower_right - upper_left) + Vec2::ONE;
        let clearable = Clearable::default();
        let color = color.map(|x| x.into());
        let id = self
            .commands
            .spawn((
                Name::new("rectfill"),
                if let Some(fill_pat) = &self.state.draw_state.fill_pat {
                    let image =
                            // {
                            //     let a = Gfx::<1>::from_vec(8,8,
                            //                                vec![
                            //                                    0b10000000,
                            //                                    0b01000000,
                            //                                    0b00100000,
                            //                                    0b00010000,
                            //                                    0b00001000,
                            //                                    0b00000100,
                            //                                    0b00000010,
                            //                                    0b00000001,
                            //                                ]);
                            //     a.mirror_horizontal().to_image(|i, _, pixel_bytes| {
                            //         pixel_bytes.copy_from_slice(&PALETTE[i as usize]);
                            //     })
                            // }
                            fill_pat.to_image(8, 8, |bit, _pixel_index, pixel_bytes| {
                                let c: Option<PColor> = if bit {
                                    color.and_then(|x| x.on())
                                } else {
                                    color.map(|x| x.off()).or(Some(self.state.draw_state.pen))
                                };
                                if let Some(c) = c {
                                    // c.map(&self.state.pal_map).write_color(&PALETTE, pixel_bytes);
                                    let _ = c.write_color(
                                        &self.pico8_asset()?.palettes[self.state.palette].data,
                                        &self.state.pal_map,
                                        pixel_bytes,
                                    );
                                }
                                Ok::<(), Error>(())
                            })?;
                    Sprite {
                        anchor: Anchor::TopLeft,
                        // NOTE: Technically we only need a 4x4 image. However, this generates a warning.
                        //
                        // ```
                        // WARN bevy_sprite::texture_slice: One of your tiled
                        // textures has generated 1089 slices. You might want to
                        // use higher stretch values to avoid a great
                        // performance cost
                        // ```
                        //
                        // So we generate an 8x8 to avoid that.
                        image: self.images.add(image),
                        custom_size: Some(size),
                        image_mode: SpriteImageMode::Tiled {
                            tile_x: true,
                            tile_y: true,
                            stretch_value: 1.0,
                        },
                        ..default()
                    }
                } else {
                    let c =
                        self.get_color(color.map(|x| x.off().into()).unwrap_or(N9Color::Pen))?;
                    Sprite {
                        color: c,
                        anchor: Anchor::TopLeft,
                        custom_size: Some(size),
                        ..default()
                    }
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn rect(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = pixel_snap(self.state.draw_state.apply_camera_delta(upper_left));
        let lower_right = pixel_snap(self.state.draw_state.apply_camera_delta(lower_right));
        let c = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size = (lower_right - upper_left) + Vec2::ONE;
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("rect"),
                Sprite {
                    image: self.pico8_asset()?.border.clone(),
                    color: c,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(size),
                    image_mode: SpriteImageMode::Sliced(TextureSlicer {
                        border: BorderRect::square(1.0),
                        center_scale_mode: SliceScaleMode::Stretch,
                        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
                        ..default()
                    }),
                    ..default()
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn fillp(&mut self, pattern: Option<u16>) -> u16 {
        let last: u16 = self
            .state
            .draw_state
            .fill_pat
            .map(|x| x.into())
            .unwrap_or(0);
        if let Some(pattern) = pattern {
            if pattern == 0 {
                self.state.draw_state.fill_pat = None;
            } else {
                self.state.draw_state.fill_pat = Some(pattern.into());
            }
        }
        last
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::core::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
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
            .register("fillp", |ctx: FunctionCallContext, pattern: Option<u16>| {
                with_pico8(&ctx, move |pico8| Ok(pico8.fillp(pattern)))
            })
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
            );
    }
}
