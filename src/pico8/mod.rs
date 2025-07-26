use bevy::{asset::embedded_asset, prelude::*};
mod api;
pub use api::*;
mod clear;
pub use clear::*;
pub mod audio;
mod map;
pub use map::*;
mod pal_map;
pub(crate) use pal_map::*;
mod pal;
pub(crate) use pal::*;
mod gfx;
pub use gfx::*;
mod fillp;
pub mod p8scii;
pub(crate) use fillp::*;
mod gfx_handles;
pub(crate) use gfx_handles::*;
mod defaults;
pub(crate) mod image;
pub(crate) use defaults::*;
// mod gfx2;
pub const PICO8_PALETTE: &str = "embedded://nano9/pico8/pico-8-palette.png";
pub const PICO8_BORDER: &str = "embedded://nano9/pico8/rect-border.png";
pub const PICO8_FONT: &str = "embedded://nano9/pico8/pico-8-wide.ttf";

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "pico-8-palette.png");
    embedded_asset!(app, "rect-border.png");
    embedded_asset!(app, "pico-8-wide.ttf");
    app.add_plugins(api::plugin)
        .add_plugins(clear::plugin)
        .add_plugins(audio::plugin)
        .add_plugins(gfx::plugin)
        .add_plugins(gfx_handles::plugin);
}
