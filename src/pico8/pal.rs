use bevy::prelude::*;

#[derive(Debug, Clone, Reflect)]
pub struct Palette {
    data: Vec<[u8; 4]>,
}

impl Palette {

    pub fn from_image(image: &Image) -> Self {
        let size = image.size();
        let mut data = Vec::new();
        for j in 0..size.y {
            for i in 0..size.x {
                let color: Srgba = image.get_color_at(i, j).unwrap().into();
                data.push(color.to_u8_array());
            }
        }
        Palette {
            data,
        }
    }

    pub fn from_slice(slice: &[[u8; 4]]) -> Self {
        Palette {
            data: Vec::from(slice),
        }
    }

    pub fn write_color(&self, index: usize, pixel_bytes: &mut [u8]) {
        pixel_bytes.copy_from_slice(&self.data[index][0..pixel_bytes.len()]);
    }

    pub fn get_color(&self, index: usize) -> Srgba {
        let a = &self.data[index];
        Srgba::rgba_u8(a[0], a[1], a[2], a[3])
    }
}
