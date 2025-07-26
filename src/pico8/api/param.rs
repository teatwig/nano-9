use super::*;
use bevy::ecs::system::SystemParam;

use crate::{
    pico8::{self, audio::SfxChannels, Gfx, GfxHandles},
    N9Canvas,
};

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
    pub(crate) sfx_channels: Res<'w, SfxChannels>,
    #[cfg(feature = "level")]
    pub(crate) tiled: crate::level::tiled::Level<'w, 's>,
    pub(crate) gfxs: ResMut<'w, Assets<Gfx>>,
    pub(crate) gfx_handles: ResMut<'w, GfxHandles>,
    pub(crate) pico8_assets: ResMut<'w, Assets<Pico8Asset>>,
    pub(crate) pico8_handle: Res<'w, Pico8Handle>,
    pub(crate) defaults: Res<'w, pico8::Defaults>,
    pub(crate) clear_cache: Res<'w, ClearCache>,
}
