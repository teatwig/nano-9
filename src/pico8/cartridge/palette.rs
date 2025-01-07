use bevy::prelude::*;
// use gfx::palette::RGB;

use std::collections::HashMap;

#[derive(Debug)]
pub struct CartridgePalette {
    pub colors: HashMap<u32, Srgba>
}

fn to_u8(c: &Srgba) -> (u8, u8, u8, u8) {
    ((c.red * 255.0) as u8,
     (c.green * 255.0) as u8,
     (c.blue * 255.0) as u8,
     (c.alpha * 255.0) as u8)
}

impl CartridgePalette {
    pub fn empty() -> CartridgePalette {
        CartridgePalette { colors: HashMap::new() }
    }

    pub fn new(lines: &[String]) -> CartridgePalette {
        let mut colors = HashMap::new();

        for line in lines {
            let split_line = line.split(" ");
            let vec: Vec<&str> = split_line.collect();

            if vec.len() == 4 {
                let color = vec[0].parse::<u32>().unwrap();
                let r = vec[1].parse::<u8>().unwrap();
                let g = vec[2].parse::<u8>().unwrap();
                let b = vec[3].parse::<u8>().unwrap();

                colors.insert(color, Srgba::rgb_u8(r, g, b));
            }
        }

        CartridgePalette { colors: colors }
    }

    pub fn get_data(&mut self) -> String {
        let mut data = String::new();

        for (color, rgb) in &self.colors {
            let (r,g,b,_) = to_u8(rgb);
            data.push_str(&format!("{:?} {:?} {:?} {:?}\n", color, r, g, b));
        }

        data
    }

    pub fn set_colors(&mut self, colors: HashMap<u32, Srgba>) {
        self.colors.clear();
        self.colors.extend(colors);
    }

}
