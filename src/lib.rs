#![allow(clippy::type_complexity)]
use bevy::prelude::*;
mod color;
mod entity;
pub mod error;
mod ext;
#[cfg(feature = "level")]
pub mod level;
pub mod minibuffer;
pub mod pico8;
mod plugin;
mod var;

pub use color::*;
pub use entity::*;
pub use ext::*;
pub use plugin::*;
pub use var::*;
pub mod config;
pub mod conversions;
pub mod cursor;
pub mod raycast;

pub(crate) fn plugin(app: &mut App) {
    // Add other plugins.
    app.add_plugins((config::plugin, entity::plugin, error::plugin, var::plugin, pico8::plugin));
    if app.is_plugin_added::<WindowPlugin>() {
        #[cfg(feature = "level")]
        app.add_plugins(level::plugin);
    }

    // Enable dev tools for dev builds.
    // #[cfg(feature = "dev")]
    // app.add_plugins(dev_tools::plugin);
}
