use bevy::{
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};

pub(crate) fn pixel_art_settings(settings: &mut ImageLoaderSettings) {
    // Use `nearest` image sampling to preserve the pixel art style.
    settings.sampler = ImageSampler::nearest();
}
