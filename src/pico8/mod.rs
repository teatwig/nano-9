use bevy::prelude::*;
mod pico8;
pub use pico8::*;
// pub mod cartridge;
mod cart;
pub use cart::*;
mod clear;
pub use clear::*;
pub mod audio;
mod map;
pub use map::*;
pub(crate) mod lua;
mod pal_map;
pub(crate) use pal_map::*;
mod pal;
pub(crate) use pal::*;
mod gfx;
pub(crate) mod rand;
pub(crate) use gfx::*;
mod fillp;
pub mod p8scii;
pub(crate) use fillp::*;
mod gfx_handles;
pub(crate) use gfx_handles::*;
pub(crate) mod image;
// mod gfx2;

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(pico8::plugin)
        .add_plugins(lua::plugin)
        .add_plugins(clear::plugin)
        .add_plugins(audio::plugin)
        .add_plugins(rand::plugin)
        .add_plugins(gfx::plugin)
        .add_plugins(gfx_handles::plugin)
        .add_plugins(cart::plugin);
}
