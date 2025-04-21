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

pub(crate) fn plugin(app: &mut App) {
    app
        .init_asset::<Gfx>();
}

#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx {
    pub nybbles: Vec<u8>,
    pub pixel_width: usize,
}

impl Gfx {
    pub fn new(size: UVec2) -> Self {
        Gfx {
            nybbles: vec![0; (size.x * size.y) as usize / 2],
            pixel_width: size.x as usize,
        }
    }

    pub fn get(&self, pos: UVec2) -> u8 {
        let byte = self.nybbles[pos.x as usize / 2 + pos.y  as usize * self.pixel_width / 2];
        if pos.x % 2 == 0 {
            // high nybble
            byte >> 4
        } else {
            // low nybble
            byte & 0x0f
        }
    }

    pub fn set(&mut self, pos: UVec2, color_index: u8) {
        let byte = &mut self.nybbles[pos.x as usize / 2 + pos.y as usize * self.pixel_width / 2];
        if pos.x % 2 == 0 {
            // high nybble
            *byte = color_index << 4 | *byte & 0x0f;
        } else {
            // low nybble
            *byte = (*byte & 0xf0) | (color_index & 0x0f);
        }
    }
    /// Turn a Gfx into an image.
    ///
    /// The `write_color` function writes a Srgba set of pixels to the given u8
    /// slice of four bytes.
    pub fn to_image(&self, write_color: impl Fn(u8, &mut [u8])) -> Image {
        let pixel_count = self.nybbles.len() * 2;
        let columns = self.pixel_width;
        let (rows, remainder) = (pixel_count / columns, pixel_count % columns);
        assert_eq!(remainder, 0, "Gfx expects an integer number of rows but {} bytes were left over", remainder);
        let mut pixel_bytes = vec![0x00; columns * rows * 4];
        let mut i = 0;
        for byte in &self.nybbles {
            // first nybble
            write_color(byte & 0x0f, &mut pixel_bytes[i * 4..(i + 1) * 4]);
            i += 1;
            // second nybble
            write_color(byte >> 4, &mut pixel_bytes[i * 4..(i + 1) * 4]);
            i += 1;
        }
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
