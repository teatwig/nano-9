#![allow(clippy::type_complexity)]
use bevy::prelude::*;
mod color;
pub mod error;
mod ext;
mod entity;
#[cfg(feature = "level")]
pub mod level;
pub mod pico8;
mod plugin;
pub mod minibuffer;
mod var;

pub use color::*;
pub use entity::*;
pub use ext::*;
pub use plugin::*;
pub use var::*;
pub mod config;
pub mod cursor;
pub mod conversions;

pub(crate) fn plugin(app: &mut App) {
    // Add other plugins.
    app.add_plugins((
        config::plugin,
        entity::plugin,
        error::plugin,
        var::plugin,
    ));
    if app.is_plugin_added::<WindowPlugin>() {
        #[cfg(feature = "level")]
        app.add_plugins(level::plugin);
    }

    // Enable dev tools for dev builds.
    // #[cfg(feature = "dev")]
    // app.add_plugins(dev_tools::plugin);
}
