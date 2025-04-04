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
mod lua;

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(pico8::plugin)
        .add_plugins(lua::plugin)
        .add_plugins(clear::plugin)
        .add_plugins(audio::plugin)
        .add_plugins(cart::plugin);
}
