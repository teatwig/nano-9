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
mod canvas;
#[cfg(feature = "level")]
mod level;
mod line;
#[cfg(feature = "level")]
pub use level::*;

use bevy::{
    image::ImageSampler,
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

pub const MAP_COLUMNS: u32 = 128;
pub const PICO8_SPRITE_SIZE: UVec2 = UVec2::new(8, 8);
pub const PICO8_TILE_COUNT: UVec2 = UVec2::new(16, 16);

pub(crate) const PALETTE: [[u8; 4]; 16] = [
    [0x00, 0x00, 0x00, 0xff], //black
    [0x1d, 0x2b, 0x53, 0xff], //dark-blue
    [0x7e, 0x25, 0x53, 0xff], //dark-purple
    [0x00, 0x87, 0x51, 0xff], //dark-green
    [0xab, 0x52, 0x36, 0xff], //brown
    [0x5f, 0x57, 0x4f, 0xff], //dark-grey
    [0xc2, 0xc3, 0xc7, 0xff], //light-grey
    [0xff, 0xf1, 0xe8, 0xff], //white
    [0xff, 0x00, 0x4d, 0xff], //red
    [0xff, 0xa3, 0x00, 0xff], //orange
    [0xff, 0xec, 0x27, 0xff], //yellow
    [0x00, 0xe4, 0x36, 0xff], //green
    [0x29, 0xad, 0xff, 0xff], //blue
    [0x83, 0x76, 0x9c, 0xff], //lavender
    [0xff, 0x77, 0xa8, 0xff], //pink
    [0xff, 0xcc, 0xaa, 0xff], //light-peach
];

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<Pico8Asset>()
        .register_type::<Pico8State>()
        .register_type::<N9Font>()
        .register_type::<Palette>()
        .register_type::<SpriteSheet>()
        .init_asset::<Pico8Asset>()
        .init_resource::<Pico8State>()
        .add_observer(
            |trigger: Trigger<UpdateCameraPos>,
             camera: Single<&mut Transform, With<Nano9Camera>>| {
                let pos = trigger.event();
                let mut camera = camera.into_inner();
                camera.translation.x = pos.0.x;
                camera.translation.y = negate_y(pos.0.y);
            },
        );
}

pub(crate) fn to_nybble(a: u8) -> Option<u8> {
    let b = a as char;
    b.to_digit(16).map(|x| x as u8)
}

pub(crate) fn to_byte(a: u8, b: u8) -> Option<u8> {
    let a = to_nybble(a)?;
    let b = to_nybble(b)?;
    Some((a << 4) | b)
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

/// Snap to pixel IF the feature "pixel-snap" is enabled.
#[inline]
pub fn pixel_snap(v: Vec2) -> Vec2 {
    if cfg!(feature = "pixel-snap") {
        v.floor()
    } else {
        v
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
