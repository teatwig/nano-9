use bevy::prelude::*;

use crate::pico8::{Error, PalMap};

#[derive(Debug, Clone, Copy, Reflect)]
pub enum PColor {
    Palette(usize),
    Color(LinearRgba),
}

impl PColor {
    /// Map the palette
    pub fn map_pal(&self, f: impl FnOnce(usize) -> usize) -> PColor {
        match self {
            PColor::Palette(i) => PColor::Palette(f(*i)),
            x => *x,
        }
    }

    pub fn write_color(
        &self,
        palette: &[[u8; 4]],
        pal_map: &PalMap,
        pixel_bytes: &mut [u8],
    ) -> Result<(), Error> {
        match self {
            PColor::Palette(i) => pal_map.write_color(palette, *i as u8, pixel_bytes),
            PColor::Color(c) => {
                let arr = c.to_u8_array();
                pixel_bytes.copy_from_slice(&arr);
                Ok(())
            }
        }
    }
}

impl From<Color> for PColor {
    fn from(c: Color) -> Self {
        PColor::Color(c.into())
    }
}

impl From<LinearRgba> for PColor {
    fn from(c: LinearRgba) -> Self {
        PColor::Color(c)
    }
}

impl From<usize> for PColor {
    fn from(n: usize) -> Self {
        PColor::Palette(n)
    }
}

impl From<i32> for PColor {
    fn from(n: i32) -> Self {
        PColor::Palette(n as usize)
    }
}
