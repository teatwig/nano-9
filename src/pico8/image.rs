use bevy::image::{ImageLoaderSettings, ImageSampler};

pub(crate) fn pixel_art_settings(settings: &mut ImageLoaderSettings) {
    // Use `nearest` image sampling to preserve the pixel art style.
    if let Some(image_sampler) = image_sampler() {
        settings.sampler = image_sampler;
    }
}

#[inline]
pub(crate) fn image_sampler() -> Option<ImageSampler> {
    Some(ImageSampler::nearest())
}
