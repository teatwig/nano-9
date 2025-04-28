use bevy::{
    prelude::*,
    image::ImageSampler,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bitvec::prelude::*;

#[derive(Debug, Deref, DerefMut, Hash, PartialEq, Eq, Clone, Copy, Reflect, Default)]
pub struct FillPat {
    #[reflect(ignore)]
    pub data: BitArray<[u8; 2], Lsb0>,
}

impl FillPat {

    /// Get a fill.
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.data.get((x % 4) + (y % 4) * 4).as_deref().map(|x| *x).unwrap_or(false)
    }

    /// Set fill.
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.data.set((x % 4) + (y % 4) * 4, value);
    }

    /// Create an image.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn to_image<E>(&self, width: usize, height: usize, mut write_color: impl FnMut(bool, usize, &mut [u8]) -> Result<(), E>) -> Result<Image, E> {
        let mut pixel_bytes = vec![0x00; width * height * 4];
        let mut i = 0;
        for y in 0..height {
            for x in 0..width {
                let pat_bit = self.get(x, y);
                write_color(pat_bit, i, &mut pixel_bytes[i * 4..(i + 1) * 4])?;
                i += 1;
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                ..default()
            },
            TextureDimension::D2,
            pixel_bytes,
            TextureFormat::Rgba8UnormSrgb,
            // Must have main world, not sure why.
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        Ok(image)
    }

}
