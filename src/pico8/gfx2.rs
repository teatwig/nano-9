use crate::{
    pico8::{audio::*, *}, DrawState,
    error::RunState,
};
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    image::{ImageLoaderSettings, ImageSampler},
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_mod_scripting::core::asset::ScriptAsset;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
};
use bitvec::prelude::*;

pub(crate) fn plugin(app: &mut App) {
    app
        .init_asset::<Gfx<4>>();
}

#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx<const N: usize> {
    #[reflect(ignore)]
    pub nybbles: BitVec<u8, Lsb0>,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

impl<const N: usize> Gfx<N> {
    pub fn new(size: UVec2) -> Self {
        Gfx {
            // (x * y * N).div_ceil(8)
            nybbles: bitvec![u8, Lsb0; 0; ((size.x * size.y) as usize * N).div_ceil(8)],
            pixel_width: size.x as usize,
            pixel_height: size.y as usize,
        }
    }

    pub fn get(&self, pos: UVec2) -> u8 {
        // x / 2 + y * w / 2
        // x = 8u
        // v = N * w
        // 8u / 2 +
        let start = pos.x as usize * N + pos.y as usize * N * self.pixel_width;
        let slice = &self.nybbles[start..start+N];
        let mut result = bitvec![u8, Lsb0; 0; N];
        result.copy_from_slice(slice);
        result.into_vec()[0]

        // let byte = self.nybbles[pos.x as usize / 2 + pos.y  as usize * self.pixel_width / 2];
        // if pos.x % 2 == 0 {
        //     // high nybble
        //     byte >> 4
        // } else {
        //     // low nybble
        //     byte & 0x0f
        // }
    }

    pub fn set(&mut self, pos: UVec2, color_index: u8) {

        let value = BitVec::<u8, Lsb0>::from_vec(vec![color_index]);
        let start = pos.x as usize * N + pos.y as usize * N * self.pixel_width;
        self.nybbles[start..start+N].copy_from_bitslice(&value[0..N]);

        // let byte = &mut self.nybbles[pos.x as usize / 2 + pos.y as usize * self.pixel_width / 2];
        // if pos.x % 2 == 0 {
        //     // high nybble
        //     *byte = color_index << 4 | *byte & 0x0f;
        // } else {
        //     // low nybble
        //     *byte = (*byte & 0xf0) | (color_index & 0x0f);
        // }
    }
    /// Turn a Gfx<const N: usize> into an image.
    ///
    /// The `write_color` function writes a Srgba set of pixels to the given u8
    /// slice of four bytes.
    pub fn to_image(&self, write_color: impl Fn(u8, &mut [u8])) -> Image {
        let pixel_count = self.nybbles.len() * 2;
        let columns = self.pixel_width;
        let (rows, remainder) = (pixel_count / columns, pixel_count % columns);
        assert_eq!(remainder, 0, "Gfx<const N: usize> expects an integer number of rows but {} bytes were left over", remainder);
        let mut pixel_bytes = vec![0x00; columns * rows * 4];
        let mut i = 0;
        let c = N;
        let mut color_index = bitarr!(u8, Lsb0; 0; 8); // How to provide N here?
        for pixel in self.nybbles.chunks_exact(N) {
            color_index.copy_from_slice(pixel);
            write_color(color_index.data[0], &mut pixel_bytes[i * 4..(i + 1) * 4]);
        }
        // for byte in &self.nybbles {
        //     // first nybble
        //     write_color(byte & 0x0f, );
        //     i += 1;
        //     // second nybble
        //     write_color(byte >> 4, &mut pixel_bytes[i * 4..(i + 1) * 4]);
        //     i += 1;
        // }
        let mut image = Image::new(
            Extent3d {
                width: columns as u32,
                height: rows as u32,
                ..default()
            },
            TextureDimension::D2,
            pixel_bytes,
            TextureFormat::Rgba8UnormSrgb,
            // Must have main world, not sure why.
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        image
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ex0() {
        let mut a = Gfx::<4>::new(UVec2::new(8, 8));
        assert_eq!(0, a.get(UVec2::new(0,0)));
        a.set(UVec2::new(0, 0), 15);
        assert_eq!(15, a.get(UVec2::new(0,0)));
    }

}
