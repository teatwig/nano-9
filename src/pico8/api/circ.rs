use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn circfill(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let pos = self.state.draw_state.apply_camera_delta_ivec2(pos);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("circ rect");
        let path = PathBuilder::from_oval(oval).expect("circ path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let offset = 0.5;
        let id = self
            .commands
            .spawn((
                Name::new("circfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::Custom(Vec2::new(
                        -offset / size.x as f32,
                        offset / size.y as f32,
                    )),
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, negate_y(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn circ(
        &mut self,
        pos: IVec2,
        r: impl Into<UVec2>,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let pos = self.state.draw_state.apply_camera_delta_ivec2(pos);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let r: UVec2 = r.into();
        let size: UVec2 = r * UVec2::splat(2) + UVec2::ONE;
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("circ rect");
        let path = PathBuilder::from_oval(oval).expect("circ path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        let mut stroke = Stroke::default();
        stroke.width = 0.0;
        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();

        let offset = 0.5;
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("circ"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::Custom(Vec2::new(
                        -offset / size.x as f32,
                        offset / size.y as f32,
                    )),
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(pos.x as f32, negate_y(pos.y as f32), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{pico8::lua::with_pico8, DropPolicy, N9Entity};

    use bevy_mod_scripting::core::bindings::{
        function::{
            into_ref::IntoScriptRef,
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        ReflectReference,
    };
    pub(crate) fn plugin(app: &mut App) {
        // callbacks can receive any `ToLuaMulti` arguments, here '()' and
        // return any `FromLuaMulti` arguments, here a `usize`
        // check the Rlua documentation for more details
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
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
            );
    }
}
