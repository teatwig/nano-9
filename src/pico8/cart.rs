use bevy::{
    render::{render_asset::RenderAssetUsages,
             render_resource::{Extent3d, TextureDimension, TextureFormat}
    },
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
enum CartLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not convert to utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Unexpected header: {0}")]
    UnexpectedHeader(String),
    #[error("Unexpected hexadecimal: {0}")]
    UnexpectedHex(char),
}

#[derive(Asset, TypePath, Debug)]
struct Cart {
    lua: String,
    sprites: Option<Image>,
    map: Vec<u8>,
    flags: Vec<u8>,
}
const PALETTE: [[u8; 3]; 16] = [
[0, 0, 0], // black
[29, 43, 83], //dark-blue
[126, 37, 83], //dark-purple
[0, 135, 81], //dark-green
[171, 82, 54], //brown
[95, 87, 79], //dark-grey
[194, 195, 199], //light-grey
[255, 241, 232], //white
[255, 0, 77], //red
[255, 163, 0], //orange
[255, 236, 39], //yellow
[0, 228, 54], //green
[41, 173, 255], //blue
[131, 118, 156], //lavender
[255, 119, 168], //pink
[255, 204, 170], // light-peach
];

impl Cart {
    fn from_str(content: &str) -> Result<Self, CartLoaderError> {
        let headers = ["lua", "gfx", "gff", "label", "map", "sfx", "music"];
        let mut sections = [(None,None); 7];
        // enum State {
        //     None,
        //     Lua,
        //     Gfx,
        //     Gff,
        //     Label,
        //     Map,
        //     Sfx,
        //     Music,
        // }
        // let mut state = State::None;
        let mut even_match: Option<usize> = None;
        for (index, _) in content.match_indices("__") {
            if let Some(begin) = even_match {
                let header = &content[begin + 2..index];
                if let Some(h) = headers.iter().position(|word| *word == header) {
                    sections[h].0 = Some(index + 3);
                    if let Some(h) = h.checked_sub(1) {
                        sections[h].1 = Some(begin - 1);
                    }
                } else {
                    Err(CartLoaderError::UnexpectedHeader(String::from(header)))?;
                }
                even_match = None;
            } else {
                even_match = Some(index);
            }
        }
        // Close the last segment.
        for segment in &mut sections {
            if segment.0.is_some() && segment.1.is_none() {
                segment.1 = Some(content.len());
            }
        }
        let get_segment = |(i, j): &(Option<usize>, Option<usize>)| -> Option<&str> {
            i.zip(*j).map(|(i,j)| &content[i..j])
        };


        // lua
        let lua: String = get_segment(&sections[0]).unwrap_or("").into();
        let sprites = if let Some(content) = get_segment(&sections[1]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if columns.is_none() || columns == Some(0) {
                None
            } else {
                let columns = columns.unwrap();
            let rows = lines.count() + 1;
            let width = columns as u32 * 2;
            let height = rows as u32;
            // let mut bytes = Vec::with_capacity((dbg!(width * height)) as usize * 4);
            let mut bytes = vec![0xff; (width * height) as usize * 4];
            let mut set_color = |pixel_index: usize, palette_index: u8| {
                // PERF: We should just set the 24 or 32 bits in one go, right?
                bytes[pixel_index * 4 + 0] = PALETTE[palette_index as usize][0];
                bytes[pixel_index * 4 + 1] = PALETTE[palette_index as usize][1];
                bytes[pixel_index * 4 + 2] = PALETTE[palette_index as usize][2];
                bytes[pixel_index * 4 + 3] = 0xff;
            };
            let mut i = 0;
            for line in content.lines() {
                for c in line.as_bytes() {
                    let c = *c as char;
                    let digit: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                    let left = digit >> 4;
                    let right = digit & 0x0f;
                    set_color(i, left);
                    set_color(i + 1, left);
                    i += 2;
                }
            }
            Some(Image::new(Extent3d {
                width,
                height,
                ..default()
            },
                            TextureDimension::D2,
                            bytes,
                            TextureFormat::Rgba8UnormSrgb,
                            RenderAssetUsages::RENDER_WORLD,
            ))
            }
        } else {
            None
        };
        Ok(Cart {
            lua,
            sprites,
            map: Vec::new(),
            flags: Vec::new(),
        })
    }
}

#[derive(Default)]
struct CartLoader;

impl AssetLoader for CartLoader {
    type Asset = Cart;
    type Settings = ();
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;
        Ok(Cart::from_str(&content)?)
    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const sample_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
 cls()
	spr(1, 0, 0)
end
__gfx__
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;
    #[test]
    fn test_string_find() {
        let s = String::from("Hello World");
        assert_eq!(s.find('o'), Some(4));
        assert_eq!(s[5..].find('o'), Some(2));
    }

    #[test]
    fn test_cart_from() {
        let cart = Cart::from_str(sample_cart).unwrap();
        assert_eq!(cart.lua, r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#);
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(256));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
    }


}
