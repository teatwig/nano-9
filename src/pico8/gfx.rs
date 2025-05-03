use crate::pico8::*;
use bevy::{
    image::ImageSampler,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bitvec::{prelude::*, view::BitView};

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<Gfx>();
}

/// An indexed image using `N`-bit palette with color index `T`.
#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx<const N: usize = 4, T: TypePath + Send + Sync + BitStore = u8> {
    #[reflect(ignore)]
    pub data: BitVec<T, Lsb0>,
    pub width: usize,
    pub height: usize,
}

impl<T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy> Gfx<1, T> {
    pub fn mirror_horizontal(mut self) -> Self {
        for elem in self.data.chunks_mut(self.width) {
            elem.reverse();
        }
        self
    }
}

impl<
        const N: usize,
        T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy,
    > Gfx<N, T>
{
    /// Create an indexed image.
    pub fn new(width: usize, height: usize) -> Self {
        Gfx {
            data: BitVec::<T, Lsb0>::repeat(false, width * height * N),
            width,
            height,
        }
    }

    pub fn from_vec(width: usize, height: usize, vec: Vec<T>) -> Self {
        let gfx = Gfx {
            data: BitVec::<T, Lsb0>::from_vec(vec),
            width,
            height,
        };
        assert!(width * height * N <= gfx.data.len());
        gfx
    }

    /// Get a color index.
    pub fn get(&self, x: usize, y: usize) -> T {
        let start = x * N + y * N * self.width;
        let slice = &self.data[start..start + N];
        let mut result = T::default();
        let bits = result.view_bits_mut::<Lsb0>();
        bits[0..N].copy_from_bitslice(slice);
        result
    }

    /// Set a color index.
    pub fn set(&mut self, x: usize, y: usize, color_index: T) {
        let bits = color_index.view_bits::<Lsb0>();
        let start = x * N + y * N * self.width;
        self.data[start..start + N].copy_from_bitslice(&bits[0..N]);
    }

    /// Create an image.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn try_to_image<E>(
        &self,
        mut write_color: impl FnMut(T, usize, &mut [u8]) -> Result<(), E>,
    ) -> Result<Image, E> {
        let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        for (i, pixel) in self.data.chunks_exact(N).enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..N].copy_from_bitslice(pixel);
            write_color(color_index, i, &mut pixel_bytes[i * 4..(i + 1) * 4])?;
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

    pub fn to_image(&self, mut write_color: impl FnMut(T, usize, &mut [u8])) -> Image {
        self.try_to_image::<Error>(move |color_index, pixel_index, pixel_bytes| {
            write_color(color_index, pixel_index, pixel_bytes);
            Ok(())
        })
        .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const BIT1_PALETTE: [[u8; 4]; 2] = [[0x00, 0x00, 0x00, 0xff], [0xff, 0xff, 0xff, 0xff]];

    #[test]
    fn ex0() {
        let mut a = Gfx::<4>::new(8, 8);
        assert_eq!(0, a.get(0, 0));
        a.set(0, 0, 15);
        assert_eq!(15, a.get(0, 0));
    }

    #[test]
    fn create_image() {
        let mut a = Gfx::<4>::new(8, 8);
        assert_eq!(0, a.get(0, 0));
        a.set(0, 0, 15);
        let _ = a.to_image(|_, _, _| {});
    }

    #[test]
    fn create_1bit_image() {
        let a = Gfx::<1>::from_vec(
            8,
            8,
            vec![
                0b00000001, 0b00000010, 0b00000100, 0b00001000, 0b00010000, 0b00100000, 0b01000000,
                0b10000000,
            ],
        );
        let image = a.to_image(|i, _, pixel_bytes| {
            pixel_bytes.copy_from_slice(&BIT1_PALETTE[i as usize]);
        });
        let color: Srgba = image.get_color_at(0, 0).unwrap().into();
        assert_eq!(color, Srgba::BLACK);
        let color: Srgba = image.get_color_at(0, 7).unwrap().into();
        assert_eq!(color, Srgba::WHITE);
    }
}
