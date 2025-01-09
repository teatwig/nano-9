use bevy::prelude::*;
mod pico8;
pub use pico8::*;
// pub mod cartridge;
mod cart;
pub use cart::*;

pub fn plugin(app: &mut App) {
    app
        .add_plugins(pico8::plugin)
        .add_plugins(cart::plugin);
}
