mod error;
pub use error::*;
mod asset;
use super::*;
pub use asset::*;
mod spr;
pub use spr::*;
mod state;
pub use state::*;
mod handle;
pub use handle::*;
pub mod input;
use input::*;
mod camera;
use camera::*;
mod param;
pub use param::*;
mod sfx;
pub use sfx::*;
mod circ;
mod map;
mod oval;
mod pal;
mod print;
mod rect;
pub use pal::*;
mod bit_ops;
mod canvas;
#[cfg(feature = "level")]
mod level;
mod line;
mod poke;
mod time;
#[cfg(feature = "level")]
pub use level::*;

use bevy::{
    image::ImageSampler,
    input::gamepad::GamepadConnectionEvent,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
    text::TextLayoutInfo,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

use crate::{
    pico8::{
        self, audio::AudioBank, image::pixel_art_settings, ClearEvent, Clearable, Map, PalMap,
        Palette,
    },
    DrawState, FillColor, N9Color, Nano9Camera, PColor,
};

use std::{borrow::Cow, f32::consts::PI};

pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

const ANALOG_STICK_THRESHOLD: f32 = 0.1;

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Pico8Asset>()
        .register_type::<Pico8State>()
        .register_type::<N9Font>()
        .register_type::<Palette>()
        .register_type::<SpriteSheet>()
        .init_asset::<Pico8Asset>()
        .init_resource::<Pico8State>()
        .init_resource::<PlayerInputs>()
        .add_observer(
            |trigger: Trigger<UpdateCameraPos>,
             camera: Single<&mut Transform, With<Nano9Camera>>| {
                let pos = trigger.event();
                let mut camera = camera.into_inner();
                camera.translation.x = pos.0.x;
                camera.translation.y = negate_y(pos.0.y);
            },
        )
        .add_plugins((
            sfx::plugin,
            spr::plugin,
            map::plugin,
            input::plugin,
            print::plugin,
            rect::plugin,
            circ::plugin,
            oval::plugin,
            pal::plugin,
            bit_ops::plugin,
            line::plugin,
            poke::plugin,
            canvas::plugin,
            camera::plugin,
            time::plugin,
            #[cfg(feature = "level")]
            level::plugin,
        ));
}

/// Negates y IF the feature "negate-y" is enabled.
#[inline]
pub fn negate_y(y: f32) -> f32 {
    if cfg!(feature = "negate-y") {
        -y
    } else {
        y
    }
}

impl Pico8<'_, '_> {
    pub fn exit(&mut self, error: Option<u8>) {
        self.commands.send_event(match error {
            Some(n) => std::num::NonZero::new(n)
                .map(AppExit::Error)
                .unwrap_or(AppExit::Success),
            None => AppExit::Success,
        });
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

    /// Return the size of the canvas
    ///
    /// This is not the window dimensions, which are physical pixels. Instead it
    /// is the number of "logical" pixels, which may be comprised of many
    /// physical pixels.
    pub fn canvas_size(&self) -> UVec2 {
        self.canvas.size
    }
}

#[cfg(test)]
mod test {

    #[test]
    fn test_suffix_match() {
        let s = "a\\0";
        assert_eq!(s.len(), 3);
        assert!(s.ends_with("\\0"));
    }
}
