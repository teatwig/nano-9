use bevy::{
    prelude::*
};
use super::*;

#[derive(Clone, Asset, Debug, Reflect)]
pub struct Pico8Asset {
    #[cfg(feature = "scripting")]
    pub code: Option<Handle<bevy_mod_scripting::core::asset::ScriptAsset>>,
    pub(crate) palettes: Vec<Palette>,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Vec<SpriteSheet>,
    pub(crate) maps: Vec<Map>,
    pub(crate) font: Vec<N9Font>,
    pub(crate) audio_banks: Vec<AudioBank>,
}
