use crate::config;
use bevy::prelude::*;

#[derive(Debug, Resource)]
pub struct Defaults {
    pub pen_color: usize,
    pub font_size: f32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            pen_color: 1,
            font_size: 5.0,
        }
    }
}

impl Defaults {
    pub fn from_config(config_defaults: &config::Defaults) -> Self {
        Self {
            pen_color: config_defaults.pen_color.unwrap_or(1),
            font_size: config_defaults.font_size.unwrap_or(5.0),
        }
    }
}
