#![doc(html_root_url = "https://docs.rs/nano9/0.1.0-alpha.2")]
#![doc = include_str!("../README.md")]
#![allow(clippy::type_complexity)]
pub use bevy;
use bevy::prelude::*;
mod color;
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

pub use color::*;
pub use ext::*;
pub use plugin::*;
pub mod config;
pub mod cursor;
pub mod raycast;
pub use plugins::*;

pub(crate) fn plugin(app: &mut App) {
    // Add other plugins.
    app.add_plugins((config::plugin, error::plugin, pico8::plugin));
    if app.is_plugin_added::<WindowPlugin>() {
        #[cfg(feature = "level")]
        app.add_plugins(level::plugin);
    }
}
