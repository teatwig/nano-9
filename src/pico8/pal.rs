use crate::pico8::Error;
use bevy::prelude::*;

#[derive(Debug, Clone, Reflect)]
pub struct Palette {
    pub data: Vec<[u8; 4]>,
}

impl Palette {
    pub fn from_image(image: &Image, row: Option<u32>) -> Self {
        let size = image.size();
        let mut data = Vec::new();
        if let Some(row) = row {
            for i in 0..size.x {
                let color: Srgba = image.get_color_at(i, row).unwrap().into();
                data.push(color.to_u8_array());
            }
        } else {
            for j in 0..size.y {
                for i in 0..size.x {
                    let color: Srgba = image.get_color_at(i, j).unwrap().into();
                    data.push(color.to_u8_array());
                }
            }
        }
        Palette { data }
    }

    pub fn from_slice(slice: &[[u8; 4]]) -> Self {
        Palette {
            data: Vec::from(slice),
        }
    }

    pub fn write_color(&self, index: usize, pixel_bytes: &mut [u8]) -> Result<(), Error> {
        let data = self
            .data
            .get(index)
            .ok_or(Error::NoSuch(format!("palette color {index}").into()))?;
        pixel_bytes.copy_from_slice(&data[0..pixel_bytes.len()]);
        Ok(())
    }

    pub fn get_color(&self, index: usize) -> Result<Srgba, Error> {
        self.data
            .get(index)
            .ok_or(Error::NoSuch(format!("palette color {index}").into()))
            .map(|a| Srgba::rgba_u8(a[0], a[1], a[2], a[3]))
    }
}
