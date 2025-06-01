

use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        error::InteropError,
    };

use crate::pico8::{
        Gfx,
    };

use std::any::TypeId;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn ovalfill(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta_ivec2(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta_ivec2(lower_right);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE)
            .try_into()
            .unwrap();
        // // let size = UVec2::new((a.x - b.x).abs() + 1,
        // //                       (a.y - b.y).abs() + 1);
        // let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
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
        let id = self
            .commands
            .spawn((
                Name::new("ovalfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    negate_y(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn oval(
        &mut self,
        upper_left: IVec2,
        lower_right: IVec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = self.state.draw_state.apply_camera_delta_ivec2(upper_left);
        let lower_right = self.state.draw_state.apply_camera_delta_ivec2(lower_right);
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size: UVec2 = ((lower_right - upper_left) + IVec2::ONE)
            .try_into()
            .unwrap();
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
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
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("oval"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(
                    upper_left.x as f32,
                    negate_y(upper_left.y as f32),
                    clearable.suggest_z(),
                ),
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
    use crate::pico8::lua::with_pico8;

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
    // callbacks can receive any `ToLuaMulti` arguments, here '()' and
    // return any `FromLuaMulti` arguments, here a `usize`
    // check the Rlua documentation for more details
    let world = app.world_mut();

    NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
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

        ;
}

}
