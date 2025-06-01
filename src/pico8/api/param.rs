use super::*;
use bevy::{
    audio::PlaybackMode,
    ecs::system::SystemParam,
    image::ImageSampler,
    input::gamepad::GamepadConnectionEvent,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::Anchor,
    text::TextLayoutInfo,
    prelude::*,
};
use tiny_skia::{self, FillRule, Paint, PathBuilder, Pixmap, Stroke};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        error::InteropError,
    };

use crate::{
    pico8::{
        self,
        audio::{AudioBank, AudioCommand, SfxChannels, SfxDest},
        image::pixel_art_settings,
        keyboard::KeyInput,
        mouse::MouseInput,
        rand::Rand8,
        ClearEvent, Clearable, Gfx, GfxHandles, Map, PalMap, Palette,
    },
    DrawState, FillColor, N9Canvas, N9Color, Nano9Camera, PColor, ValueExt,
};

use std::{any::TypeId, borrow::Cow, f32::consts::PI};


#[derive(SystemParam)]
#[allow(dead_code)]
pub struct Pico8<'w, 's> {
    // TODO: Turn these image operations into triggers so that the Pico8 system
    // parameter will not preclude users from accessing images in their rust
    // systems.
    pub(crate) images: ResMut<'w, Assets<Image>>,
    pub(crate) state: ResMut<'w, Pico8State>,
    pub(crate) commands: Commands<'w, 's>,
    pub(crate) canvas: Res<'w, N9Canvas>,
    pub(crate) player_inputs: Res<'w, PlayerInputs>,
    pub(crate) sfx_channels: Res<'w, SfxChannels>,
    pub(crate) time: Res<'w, Time>,
    #[cfg(feature = "level")]
    pub(crate) tiled: crate::level::tiled::Level<'w, 's>,
    pub(crate) gfxs: ResMut<'w, Assets<Gfx>>,
    pub(crate) gfx_handles: ResMut<'w, GfxHandles>,
    pub(crate) rand8: Rand8<'w>,
    pub(crate) key_input: ResMut<'w, KeyInput>,
    pub(crate) mouse_input: ResMut<'w, MouseInput>,
    pub(crate) pico8_assets: ResMut<'w, Assets<Pico8Asset>>,
    pub(crate) pico8_handle: Res<'w, Pico8Handle>,
    pub(crate) defaults: Res<'w, pico8::Defaults>,
}
