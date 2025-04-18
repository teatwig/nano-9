use bevy::prelude::*;

use bitvec::prelude::*;
use rand::Rng;

use crate::pico8::{
        Error,
        cart::PALETTE,
    };

use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Eq)]
pub struct Pal {
    palette: Vec<[u8; 4]>,
    remap: Vec<u8>,
    pub transparency: BitVec<u8, Lsb0>,
}

impl Hash for Pal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.remap.hash(state);
        self.transparency.hash(state);
    }
}
impl PartialEq for Pal {
    fn eq(&self, other: &Self) -> bool {
        self.remap == other.remap && self.transparency == other.transparency
    }
}


impl Default for Pal {
    fn default() -> Self {
        let mut pal = Pal::with_capacity(16);
        for c in &PALETTE {
            pal.palette.push([c[0], c[1], c[2], 0xff]);
        }
        pal.transparency.set(0, true);
        pal
    }
}

impl Pal {
    pub fn with_capacity(count: usize) -> Self {
        let palette = Vec::with_capacity(count);
        let remap = (0..count).map(|x| x as u8).collect();
        let transparency = BitVec::repeat(false, count);
        Self {
            palette,
            remap,
            transparency,
        }
    }

    pub fn remap(&mut self, original_index: usize, new_index: usize) {
        self.remap[original_index] = new_index as u8;
    }

    pub fn reset(&mut self) {
        let n = self.remap.len() as u8;
        self.remap.clear();
        self.remap.extend(0..n);
        self.reset_transparency();
    }

    pub fn reset_transparency(&mut self) {
        self.transparency.fill(false);
        self.transparency.set(0, true);
    }

    pub fn from_image(image: &Image) -> Self {
        let size = image.size();
        let count = (size.x * size.y) as usize;
        let mut palette = Vec::with_capacity(count);
        for i in 0..size.x {
            for j in 0..size.y {
                let color = image.get_color_at(i, j).unwrap().to_srgba();
                let rgb = color.to_u8_array();
                palette.push(rgb);
            }
        }
        let remap = (0..count).map(|x| x as u8).collect();
        let transparency = BitVec::repeat(false, count);
        Self {
            palette,
            remap,
            transparency,
        }
    }

    pub fn write_color(&self, palette_index: u8, pixel_bytes: &mut [u8]) -> Result<(), Error> {
        let pi = *self.remap.get(palette_index as usize).ok_or(Error::NoSuch("palette index".into()))? as usize;
        // PERF: We should just set the 24 or 32 bits in one go, right?
        if *self.transparency.get(pi).ok_or(Error::NoSuch("transparency bit".into()))? {
            pixel_bytes[0..=2].copy_from_slice(&self.palette[pi][0..=2]);
            pixel_bytes[3] = 0x00;
        } else {
            pixel_bytes[0..=3].copy_from_slice(&self.palette[pi]);
        }
        Ok(())
    }

}
