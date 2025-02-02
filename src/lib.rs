#![allow(clippy::type_complexity)]
use bevy::prelude::*;
// pub mod api;
// mod assets;
// mod audio;
// mod camera;
mod color;
pub mod error;
mod ext;
// mod image;
// mod palette;
// mod pixel;
mod entity;
#[cfg(feature = "level")]
pub mod level;
// pub(crate) mod macros;
// mod nano9;
pub mod pico8;
mod plugin;
pub mod screens;
// mod sprite;
// mod text;
pub mod minibuffer;
mod var;

// pub use audio::*;
// pub use camera::*;
pub use color::*;
pub use entity::*;
pub use ext::*;
// pub use image::*;
// pub use nano9::*;
// pub use palette::*;
pub use plugin::*;
// pub use sprite::*;
// pub use text::*;
pub use var::*;
pub mod config;
pub mod cursor;

#[derive(thiserror::Error, Debug)]
pub enum N9Error {
    #[error("palette unavailable")]
    PaletteUnavailable,
}

pub(crate) fn plugin(app: &mut App) {
    // Add other plugins.
    app.add_plugins((
        // demo::plugin,
        screens::plugin,
        entity::plugin,
        // assets::plugin,
        // api::plugin,
        // sprite::plugin,
        // palette::plugin,
        error::plugin,
        // text::plugin,
        var::plugin,
        // audio::plugin,
        // level::plugin,
    ));
    if app.is_plugin_added::<WindowPlugin>() {
        #[cfg(feature = "level")]
        app.add_plugins(level::plugin);
    }

    // Enable dev tools for dev builds.
    // #[cfg(feature = "dev")]
    // app.add_plugins(dev_tools::plugin);
}
