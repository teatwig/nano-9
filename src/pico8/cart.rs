use bevy::{
    render::{render_asset::RenderAssetUsages,
             render_resource::{Extent3d, TextureDimension, TextureFormat}
    },
    image::{ImageLoaderSettings, ImageSampler},
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use bevy_mod_scripting::{core::{script::{Script, ScriptComponent}, asset::ScriptAsset}, lua::mlua::{prelude::LuaError}};
use serde::{Deserialize, Serialize};
use crate::{DrawState, pico8::{*, audio::*}};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_asset::<Cart>()
        .init_asset_loader::<CartLoader>()
        .add_systems(PreUpdate, load_cart);
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CartLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load asset: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not convert to utf-8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Unexpected header: {0}")]
    UnexpectedHeader(String),
    #[error("Unexpected hexadecimal: {0}")]
    UnexpectedHex(char),
    #[error("Missing: {0}")]
    Missing(String),
    #[error("Sfx error: {0}")]
    Sfx(#[from] SfxError)
}

#[derive(Debug)]
pub struct CartParts {
    pub lua: String,
    pub sprites: Option<Image>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
    pub sfx: Vec<Sfx>,
}

#[derive(Asset, TypePath, Debug)]
pub struct Cart {
    pub lua: Handle<ScriptAsset>,
    pub sprites: Handle<Image>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
    pub sfx: Vec<Handle<Sfx>>,
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

#[derive(Component)]
pub struct LoadCart(pub Handle<Cart>);

fn load_cart(query: Query<(Entity, &LoadCart)>,
             carts: Res<Assets<Cart>>,
             mut commands: Commands,
             asset_server: Res<AssetServer>,
             mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for (id, load_cart) in &query {
        if let Some(cart) = carts.get(&load_cart.0) {
            // It's available to load.
            let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
                // Use `nearest` image sampling to preserve the pixel art style.
                settings.sampler = ImageSampler::nearest();
            };
            let state = Pico8State {
                palette: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
                border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
                sprites: cart.sprites.clone(),
                cart: Some(load_cart.0.clone()),
                layout: layouts.add(TextureAtlasLayout::from_grid(UVec2::new(8, 8),
                                                                  16, 16,
                                                                  None, None)),
                sprite_size: UVec2::splat(8),
                draw_state: DrawState::default(),
                font: asset_server.load(PICO8_FONT),
            };
            commands.insert_resource(state);
            // commands.entity(id).insert(ScriptComponent(
            //         vec![
            //             format!("{}#lua", load_cart.0.path())
            //             // Script::new(
            //             // load_cart.0.path().map(|path| path.to_string()).unwrap_or_else(|| format!("cart {:?}", &load_cart.0)),
            //             // cart.lua.clone())
            //         ],
            //     ));
            commands.entity(id).remove::<LoadCart>();
        }
    }
}

impl CartParts {
    fn from_str(content: &str, settings: &CartLoaderSettings) -> Result<CartParts, CartLoaderError> {
        const LUA: usize = 0;
        const GFX: usize = 1;
        const MAP: usize = 4;
        const SFX: usize = 5;
        let headers = ["lua", "gfx", "gff", "label", "map", "sfx", "music"];
        let mut sections = [(None,None); 7];
        let mut even_match: Option<usize> = None;
        for (index, _) in content.match_indices("__") {
            if let Some(begin) = even_match {
                let header = &content[begin + 2..index];
                if let Some(h) = headers.iter().position(|word| *word == header) {
                    sections[h].0 = Some(index + 3);
                    if let Some(last_section) = sections[0..h].iter_mut().rev().find(|(start, end)| start.is_some() && end.is_none()) {
                        last_section.1 = Some(begin - 1);
                    }
                } else {
                    Err(CartLoaderError::UnexpectedHeader(String::from(header)))?;
                }
                even_match = None;
            } else {
                // first or odd match of '__'
                if index == 0 || content.as_bytes()[index - 1] == b'\n' {
                    even_match = Some(index);
                }
            }
        }
        // Close the last segment.
        //
        if let Some(last_section) = sections.iter_mut().rev().find(|(start, end)| start.is_some() && end.is_none()) {
            last_section.1 = Some(content.len());
        }
        let get_segment = |(i, j): &(Option<usize>, Option<usize>)| -> Option<&str> {
            i.zip(*j).map(|(i,j)| &content[i..j])
        };

        // lua
        let lua: String = get_segment(&sections[LUA]).unwrap_or("").into();
        // gfx
        let mut sprites = None;
        if let Some(content) = get_segment(&sections[GFX]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let mut rows = lines.count() + 1;
                // rows needs to be a multiple of 8.
                let partial_rows = rows % 8;
                if partial_rows != 0 {
                    rows = (rows / 8 + 1) * 8;
                }
                let mut bytes = vec![0x00; columns * rows * 4];
                let mut set_color = |pixel_index: usize, palette_index: u8| {
                    let pi = palette_index as usize;
                    // PERF: We should just set the 24 or 32 bits in one go, right?
                    bytes[pixel_index * 4] = PALETTE[pi][0];
                    bytes[pixel_index * 4 + 1] = PALETTE[pi][1];
                    bytes[pixel_index * 4 + 2] = PALETTE[pi][2];
                    bytes[pixel_index * 4 + 3] = if settings.is_transparent(pi) { 0x00 } else { 0xff };
                };
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len(), "line: {}", &line);
                    for c in line.as_bytes() {
                        let c = *c as char;
                        let digit: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        set_color(i, digit);
                        i += 1;
                    }
                }

                // if partial_rows != 0 {
                //     let missing_rows = 8 - partial_rows;
                //     for j in 0..missing_rows {
                //         for k in 0..columns {

                //         }
                //     }
                // }
                // assert_eq!(i, columns * rows);
                sprites = Some(Image::new(Extent3d {
                    width: columns as u32,
                    height: rows as u32,
                    ..default()
                },
                                TextureDimension::D2,
                                bytes,
                                TextureFormat::Rgba8UnormSrgb,
                                RenderAssetUsages::RENDER_WORLD |
                                RenderAssetUsages::MAIN_WORLD,
                ));
            }
        }
        let mut map = Vec::new();
        if let Some(content) = get_segment(&sections[MAP]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let rows = lines.count() + 1;
                let mut bytes = vec![0x00; columns / 2 * rows];
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len());
                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = high << 4 | low;
                        i += 1;
                        j += 2;
                    }
                }
                map = bytes;
            }
        }

        let sfx = if let Some(content) = get_segment(&sections[SFX]) {
            let count = content.lines().count();
            let mut sfxs = Vec::with_capacity(count);
            for line in content.lines() {
                sfxs.push(Sfx::try_from(line)?);
            }
            sfxs
        } else {
            Vec::new()
        };
        Ok(CartParts {
            lua,
            sprites,
            map,
            flags: Vec::new(),
            sfx,
        })
    }
}

pub(crate) fn to_nybble(a: u8) -> Option<u8> {
    let b = a as char;
    b.to_digit(16).map(|x| x as u8)
}

pub(crate) fn to_byte(a: u8, b: u8) -> Option<u8> {
    let a = to_nybble(a)?;
    let b = to_nybble(b)?;
    Some(a << 4 | b)
}

#[derive(Clone, Serialize, Deserialize)]
struct CartLoaderSettings {
    // Which color indices are transparent?
    palette_transparency: u16,
}

impl CartLoaderSettings {
    fn is_transparent(&self, palette_index: usize) -> bool {
        ((0b1 << palette_index) & self.palette_transparency) != 0
    }
}

impl Default for CartLoaderSettings {
    fn default() -> Self {
        CartLoaderSettings {
            palette_transparency: 0b0000_0000_0000_0001, // black is transparent by default.
        }
    }
}

#[derive(Default)]
struct CartLoader;

impl AssetLoader for CartLoader {
    type Asset = Cart;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;
        let parts = CartParts::from_str(&content, settings)?;
        let code = parts.lua;
        // cart.lua = Some(load_context.add_labeled_asset("lua".into(), ScriptAsset { bytes: code.into_bytes() }));
        let sprites = parts.sprites.unwrap_or_else(Image::default);
        let code_path = load_context.path().into();
        Ok(Cart {
            lua: load_context.labeled_asset_scope("lua".into(), move |_load_context| ScriptAsset { content: code.into_bytes().into_boxed_slice(),
            asset_path: code_path }),
            sprites: load_context.labeled_asset_scope("sprites".into(), move |_load_context| sprites),
            map: parts.map,
            flags: parts.flags,
            sfx: parts.sfx.into_iter().enumerate().map(|(n, sfx)| load_context.labeled_asset_scope(format!("sfx{n}"), move |_load_context| sfx)).collect(),
        })
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

    const black_row_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
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
"#;

    const map_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
 cls()
	spr(1, 0, 0)
end
__map__
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const test_map_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
cls()
map(0, 0, 10, 10)
end
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006006600000000000880800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000006600600000000000808800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000006066000000000000888000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006660000000000000088000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__map__
0000010101000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0000010001010100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001000303030100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0101030300000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001030303000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001010303010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0000010101010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;
    const test_sfx_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__sfx__
00010000020503f050200002107000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const pooh_sfx_cart: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__sfx__
000100001b02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;
    #[test]
    fn test_string_find() {
        let s = String::from("Hello World");
        assert_eq!(s.find('o'), Some(4));
        assert_eq!(s[5..].find('o'), Some(2));
    }

    #[test]
    fn test_cart_from() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(sample_cart, &settings).unwrap();
        assert_eq!(cart.lua, r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#);
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(128));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
    }

    #[test]
    fn test_cart_black_row() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(black_row_cart, &settings).unwrap();
        assert_eq!(cart.lua, r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#);
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(128));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
    }

    #[test]
    fn palette_transparency() {
        let settings = CartLoaderSettings::default();
        assert!(settings.is_transparent(0));
        for i in 1..15 {
            assert!(!settings.is_transparent(i));
        }
    }

    #[test]
    fn map() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(map_cart, &settings).unwrap();
        assert_eq!(cart.map[5], 136);
    }

    #[test]
    fn test_cart_map() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(test_map_cart, &settings).unwrap();
        assert_eq!(cart.lua, r#"function _draw()
cls()
map(0, 0, 10, 10)
end"#);
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(128));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
        assert_eq!(cart.map.len(), 128 * 7);
    }

    #[test]
    fn test_cart_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(test_sfx_cart, &settings).unwrap();
        assert_eq!(cart.lua, "");
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(128));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[1].volume(), 5.0 / 7.0);
        assert_eq!(notes[2].volume(), 0.0);
        assert_eq!(notes[3].volume(), 1.0);
        assert_eq!(notes[0].pitch(), 26);
        assert_eq!(notes[1].pitch(), 87);
        assert_eq!(notes[2].pitch(), 56);
        assert_eq!(notes[3].pitch(), 57);
    }

    #[test]
    fn test_pooh_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(test_sfx_cart, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[0].pitch(), 26);
        assert_eq!(notes[0].wave(), WaveForm::Triangle);
    }
}
