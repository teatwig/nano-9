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
        .register_type::<Cart>()
        .add_event::<LoadCart>()
        .init_asset::<Cart>()
        .init_asset::<Gfx>()
        .init_asset_loader::<CartLoader>()
        .add_systems(PostUpdate, load_cart);
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
    Sfx(#[from] SfxError),
    #[error("Load error: {0}")]
    LoadDirect(#[from] bevy::asset::LoadDirectError),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MusicParts {
    begin: bool,
    end: bool,
    stop: bool,
    patterns: Vec<u8>,
}

#[derive(Debug)]
pub struct CartParts {
    pub lua: String,
    pub gfx: Option<Gfx>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
    pub sfx: Vec<Sfx>,
    pub music: Vec<MusicParts>,
}

#[derive(Asset, Debug, Reflect)]
pub struct Cart {
    pub lua: Handle<ScriptAsset>,
    pub gfx: Option<Handle<Gfx>>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
    pub sfx: Vec<Handle<Sfx>>,
}

#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx {
    nybbles: Vec<u8>,
    pixel_width: usize,
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
            write_color(byte >> 4, &mut pixel_bytes[i * 4..(i + 1) * 4]);
            i += 1;
            // second nybble
            write_color(byte & 0x0f, &mut pixel_bytes[i * 4..(i + 1) * 4]);
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

pub const PALETTE: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00], //black
    [0x1d, 0x2b, 0x53], //dark-blue
    [0x7e, 0x25, 0x53], //dark-purple
    [0x00, 0x87, 0x51], //dark-green
    [0xab, 0x52, 0x36], //brown
    [0x5f, 0x57, 0x4f], //dark-grey
    [0xc2, 0xc3, 0xc7], //light-grey
    [0xff, 0xf1, 0xe8], //white
    [0xff, 0x00, 0x4d], //red
    [0xff, 0xa3, 0x00], //orange
    [0xff, 0xec, 0x27], //yellow
    [0x00, 0xe4, 0x36], //green
    [0x29, 0xad, 0xff], //blue
    [0x83, 0x76, 0x9c], //lavender
    [0xff, 0x77, 0xa8], //pink
    [0xff, 0xcc, 0xaa], //light-peach
];

#[derive(Event)]
pub struct LoadCart(pub Handle<Cart>);

fn load_cart(
    mut reader: EventReader<LoadCart>,
    carts: Res<Assets<Cart>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut next_state: ResMut<NextState<RunState>>,
) {
    for load_cart in reader.read() {
        if let Some(cart) = carts.get(&load_cart.0) {
            // It's available to load.
            let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
                // Use `nearest` image sampling to preserve the pixel art style.
                settings.sampler = ImageSampler::nearest();
            };
            let sprite_sheets: Vec<_> = cart.gfx.as_ref().map(|gfx| {
                SpriteSheet {
                    handle: SprAsset::Gfx(gfx.clone()),
                    sprite_size: UVec2::splat(8),
                    flags: cart.flags.clone(),
                    layout: layouts.add(TextureAtlasLayout::from_grid(
                        PICO8_SPRITE_SIZE,
                        PICO8_TILE_COUNT.x,
                        PICO8_TILE_COUNT.y,
                        None,
                        None,
                    )),
                }
            }).into_iter().collect();
            let state = Pico8State {
                palette: Palette {
                    handle: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
                    row: 0,
                },
                pal: Pal::default(),
                gfx_handles: HashMap::default(),
                border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
                maps: vec![P8Map {
                    entries: cart.map.clone(),
                    sheet_index: 0,
                }
                .into()]
                .into(),
                audio_banks: vec![AudioBank(
                    cart.sfx.clone().into_iter().map(Audio::Sfx).collect(),
                )]
                .into(),
                sprite_sheets: sprite_sheets.into(),
                code: cart.lua.clone(),
                draw_state: DrawState::default(),
                font: vec![N9Font {
                    handle: asset_server.load(PICO8_FONT),
                    height: Some(7.0),
                }]
                .into(),
            };
            commands.insert_resource(state);
            info!("State inserted");
            next_state.set(RunState::Run);
            // commands.entity(id).insert(ScriptComponent(
            //         vec![
            //             format!("{}#lua", load_cart.0.path())
            //             // Script::new(
            //             // load_cart.0.path().map(|path| path.to_string()).unwrap_or_else(|| format!("cart {:?}", &load_cart.0)),
            //             // cart.lua.clone())
            //         ],
            //     ));
        }
    }
}

impl CartParts {
    fn from_str(
        content: &str,
        settings: &CartLoaderSettings,
    ) -> Result<CartParts, CartLoaderError> {
        const LUA: usize = 0;
        const GFX: usize = 1;
        const GFF: usize = 3;
        const MAP: usize = 4;
        const SFX: usize = 5;
        const MUSIC: usize = 6;
        let headers = ["lua", "gfx", "label", "gff", "map", "sfx", "music"];
        let mut sections = [(None, None); 7];
        let mut even_match: Option<usize> = None;
        for (index, _) in content.match_indices("__") {
            if let Some(begin) = even_match {
                let header = &content[begin + 2..index];
                if let Some(h) = headers.iter().position(|word| *word == header) {
                    sections[h].0 = Some(index + 3);
                    if let Some(last_section) = sections[0..h]
                        .iter_mut()
                        .rev()
                        .find(|(start, end)| start.is_some() && end.is_none())
                    {
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
        if let Some(last_section) = sections
            .iter_mut()
            .rev()
            .find(|(start, end)| start.is_some() && end.is_none())
        {
            last_section.1 = Some(content.len());
        }
        let get_segment = |(i, j): &(Option<usize>, Option<usize>)| -> Option<&str> {
            i.zip(*j).map(|(i, j)| &content[i..j])
        };

        // lua
        let lua: String = get_segment(&sections[LUA]).unwrap_or("").into();
        // gfx
        let mut gfx = None;
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
                let mut bytes = vec![0x00; columns * rows / 2];
                let write_color = |palette_index: u8, pixel_bytes: &mut [u8]| {
                    let pi = palette_index as usize;
                    // PERF: We should just set the 24 or 32 bits in one go, right?
                    pixel_bytes[0] = PALETTE[pi][0];
                    pixel_bytes[1] = PALETTE[pi][1];
                    pixel_bytes[2] = PALETTE[pi][2];
                    pixel_bytes[3] = if settings.is_transparent(pi) {
                        0x00
                    } else {
                        0xff
                    };
                };
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len(), "line: {}", &line);

                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                gfx = Some(Gfx { nybbles: bytes, pixel_width: columns });
                // sprites = Some(gfx.to_image(write_color));
            }
        }
        // gff
        let mut gff = Vec::new();
        if let Some(content) = get_segment(&sections[GFF]) {
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
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                gff = bytes;
            }
        }
        info!("Got {} flags", gff.len());
        // map
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
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                map = bytes;
            }
        }
        // music
        let mut music = Vec::new();
        if let Some(content) = get_segment(&sections[MUSIC]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                // let rows = lines.count() + 1;
                for line in content.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    assert_eq!(columns, line.len());
                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    let c = line_bytes[j] as char;
                    let high: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                    let c = line_bytes[j + 1] as char;
                    let low: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                    let lead_byte = (high << 4) | low;
                    j += 3;

                    let mut i = 0;
                    let mut patterns = [0u8; 4];
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        patterns[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                    music.push(MusicParts {
                        begin: lead_byte & 1 != 0,
                        end: lead_byte & 2 != 0,
                        stop: lead_byte & 4 != 0,
                        patterns: patterns.into_iter().filter(|p| p & 64 != 0).collect(),
                    })
                }
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
            gfx,
            map,
            flags: gff,
            sfx,
            music,
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
    Some((a << 4) | b)
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
        let gfx = parts.gfx.clone();
        let mut code_path: PathBuf = load_context.path().into();
        let path = code_path.as_mut_os_string();
        path.push("#lua");
        Ok(Cart {
            lua: load_context.labeled_asset_scope("lua".into(), move |_load_context| ScriptAsset {
                content: code.into_bytes().into_boxed_slice(),
                asset_path: code_path.into(),
            }),
            gfx: gfx.map(|gfx| load_context
                .labeled_asset_scope("gfx".into(), move |_load_context| gfx)),
            map: parts.map,
            flags: parts.flags,
            sfx: parts
                .sfx
                .into_iter()
                .enumerate()
                .map(|(n, sfx)| {
                    load_context.labeled_asset_scope(format!("sfx{n}"), move |_load_context| sfx)
                })
                .collect(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const SAMPLE_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
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

    const BLACK_ROW_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
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

    const MAP_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
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

    const TEST_MAP_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
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
    const TEST_SFX_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
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

    const POOH_SFX_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__sfx__
000100001b02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const GFF_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700066600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000060006000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000066000600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006600600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000066600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__gff__
0002020200020200000000000002020000000000000202000300000000020200000000000001010101000000000202000000000000000002000000000202020000000000000000000000000000000000000200000000000000000000000002000000000000000000020000000000000000000000000000000000000000000000
0000000000000000000200000000000002020000000000000002000000000200020200000001010303000000000000000202000000010103030000000000000000000000020200020000000000000000020200000202020200000000030303030202000002020002000002020000000002020002020200020000020200000000
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
        let cart = CartParts::from_str(SAMPLE_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.pixel_width),
            Some(128)
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.nybbles.len() * 2 / gfx.pixel_width),
            Some(8)
        );
    }

    #[test]
    fn test_cart_black_row() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(BLACK_ROW_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.pixel_width),
            Some(128)
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.nybbles.len() * 2 / gfx.pixel_width),
            Some(8)
        );
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
        let cart = CartParts::from_str(MAP_CART, &settings).unwrap();
        assert_eq!(cart.map[5], 136);
    }

    #[test]
    fn test_cart_map() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(TEST_MAP_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
cls()
map(0, 0, 10, 10)
end"#
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.pixel_width),
            Some(128)
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.nybbles.len() * 2 / gfx.pixel_width),
            Some(8)
        );
        assert_eq!(cart.map.len(), 128 * 7);
    }

    #[test]
    fn test_cart_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(TEST_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.pixel_width),
            Some(128)
        );
        assert_eq!(
            cart.gfx
                .as_ref()
                .map(|gfx| gfx.nybbles.len() * 2 / gfx.pixel_width),
            Some(8)
        );

        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[1].volume(), 5.0 / 7.0);
        assert_eq!(notes[2].volume(), 0.0);
        assert_eq!(notes[3].volume(), 1.0);
        assert_eq!(notes[0].pitch(), 37);
        assert_eq!(notes[1].pitch(), 98);
        assert_eq!(notes[2].pitch(), 67);
        assert_eq!(notes[3].pitch(), 68);
    }

    #[test]
    fn test_pooh_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(POOH_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 2.0 / 7.0);
        assert_eq!(notes[0].pitch(), 62);
        assert_eq!(notes[0].wave(), WaveForm::Triangle);
    }

    #[test]
    fn test_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(TEST_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[0].pitch(), 37);
        assert_eq!(notes[0].wave(), WaveForm::Triangle);
    }

    #[test]
    fn test_gff_cart() {
        let settings = CartLoaderSettings::default();
        let cart = CartParts::from_str(GFF_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 0);
        assert_eq!(cart.flags.len(), 256);
        assert_eq!(cart.flags[1], 2);
        assert_eq!(cart.flags[37], 1);
    }

    #[test]
    fn test_non_marking_space() {
        let s = "➡️ o";
        assert_eq!(s.len(), 8);
        assert_eq!(s.chars().count(), 4);
    }
}
