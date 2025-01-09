use bevy::{
    render::{render_asset::RenderAssetUsages,
             render_resource::{Extent3d, TextureDimension, TextureFormat}
    },
    image::{ImageLoaderSettings, ImageSampler, TextureAccessError},
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::TypePath,
};
use bevy_mod_scripting::prelude::*;
use crate::{DrawState, pico8::*};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_asset::<Cart>()
        .init_asset_loader::<CartLoader>()
        .add_systems(PreUpdate, load_cart);
}

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

#[derive(Debug)]
pub struct CartParts {
    pub lua: String,
    pub sprites: Option<Image>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
}

#[derive(Asset, TypePath, Debug)]
pub struct Cart {
    pub lua: Handle<LuaFile>,
    pub sprites: Option<Handle<Image>>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
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
             palette: Local<Option<Handle<Image>>>,
             mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for (id, load_cart) in &query {
        if let Some(cart) = carts.get(&load_cart.0) {
            // It's available to load.
            let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
                // Use `nearest` image sampling to preserve the pixel art style.
                settings.sampler = ImageSampler::nearest();
            };
            commands.insert_resource(Pico8State {
                palette: asset_server.load_with_settings(PICO8_PALETTE, pixel_art_settings),
                sprites: cart.sprites.clone().expect("sprites"),
                layout: layouts.add(TextureAtlasLayout::from_grid(UVec2::new(8, 8),
                                                                  16, 16,
                                                                  None, None)),
                sprite_size: UVec2::splat(8),
                draw_state: DrawState::default(),
                font: asset_server.load(PICO8_FONT),
            });
            commands.entity(id).insert(ScriptCollection::<LuaFile> {
                    scripts: vec![Script::new(
                        load_cart.0.path().map(|path| path.to_string()).unwrap_or_else(|| format!("cart {:?}", &load_cart.0)),
                        cart.lua.clone(),
                    )],
                });
            commands.entity(id).remove::<LoadCart>();
        }
    }
}

impl Cart {
    fn parts_from_str(content: &str) -> Result<CartParts, CartLoaderError> {
        let headers = ["lua", "gfx", "gff", "label", "map", "sfx", "music"];
        let mut sections = [(None,None); 7];
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
        let mut sprites = None;
        if let Some(content) = get_segment(&sections[1]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let rows = lines.count() + 1;
                let mut bytes = vec![0x00; columns * rows * 4];
                let mut set_color = |pixel_index: usize, palette_index: u8| {
                    // PERF: We should just set the 24 or 32 bits in one go, right?
                    bytes[pixel_index * 4 + 0] = PALETTE[palette_index as usize][0];
                    bytes[pixel_index * 4 + 1] = PALETTE[palette_index as usize][1];
                    bytes[pixel_index * 4 + 2] = PALETTE[palette_index as usize][2];
                    bytes[pixel_index * 4 + 3] = 0xff;
                };
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len());
                    for c in line.as_bytes() {
                        let c = *c as char;
                        let digit: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        // let left = digit >> 4;
                        // let right = digit & 0x0f;
                        // set_color(i, left);
                        // set_color(i + 1, left);
                        // i += 2;
                        set_color(i, digit);
                        i += 1;
                    }
                }
                assert_eq!(i, columns * rows);
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
        Ok(CartParts {
            lua: lua,
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
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;
        let mut parts = Cart::parts_from_str(&content)?;
        let code = parts.lua;
        // cart.lua = Some(load_context.add_labeled_asset("lua".into(), LuaFile { bytes: code.into_bytes() }));
        Ok(Cart {
            lua: load_context.labeled_asset_scope("lua".into(), move |_load_context| LuaFile { bytes: code.into_bytes() }),
            sprites: parts.sprites.map(|images| load_context.labeled_asset_scope("sprites".into(), move |_load_context| images)),
            map: parts.map,
            flags: parts.flags,
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
    #[test]
    fn test_string_find() {
        let s = String::from("Hello World");
        assert_eq!(s.find('o'), Some(4));
        assert_eq!(s[5..].find('o'), Some(2));
    }

    #[test]
    fn test_cart_from() {
        let cart = Cart::parts_from_str(sample_cart).unwrap();
        assert_eq!(cart.lua, r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#);
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), Some(128));
        assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), Some(8));
    }


}
