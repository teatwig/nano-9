#![doc(html_root_url = "https://docs.rs/nano9/0.1.0-alpha.2")]
#![doc = include_str!("../README.md")]
#![allow(clippy::type_complexity)]
pub use bevy;
use bevy::prelude::*;
mod color;
#[cfg(feature = "scripting")]
mod entity;
pub mod error;
mod ext;
#[cfg(feature = "level")]
pub mod level;
#[cfg(feature = "minibuffer")]
pub mod minibuffer;
pub mod pico8;
mod plugin;
mod plugins;
pub mod prelude;
#[cfg(feature = "scripting")]
mod var;

pub use color::*;
#[cfg(feature = "scripting")]
pub use entity::*;
pub use ext::*;
pub use plugin::*;
#[cfg(feature = "scripting")]
pub use var::*;
pub mod config;
#[cfg(feature = "scripting")]
pub mod conversions;
pub mod cursor;
pub mod raycast;
pub use plugins::*;

pub(crate) fn plugin(app: &mut App) {
    // Add other plugins.
    app.add_plugins((config::plugin, error::plugin, pico8::plugin));
    #[cfg(feature = "scripting")]
    app.add_plugins((entity::plugin, var::plugin));
    if app.is_plugin_added::<WindowPlugin>() {
        #[cfg(feature = "level")]
        app.add_plugins(level::plugin);
    }
}
