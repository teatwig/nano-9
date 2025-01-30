use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext, AssetPath},
    image::{ImageLoaderSettings, ImageSampler},
    reflect::TypePath,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    prelude::*,

};
use serde::Deserialize;
use std::{ops::Deref, path::PathBuf};
use crate::pico8;

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct N9Config {
    pub name: Option<String>,
    pub frames_per_second: Option<u8>,
    pub description: Option<String>,
    pub template: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub screen: Option<Screen>,
    pub palette: Option<String>,
    #[serde(default, rename = "font")]
    pub fonts: Vec<Font>,
    #[serde(default, rename = "sprite_sheet")]
    pub sprite_sheets: Vec<SpriteSheet>,
    pub code: Option<PathBuf>,
    #[serde(default, rename = "audio_bank")]
    pub audio_banks: Vec<AudioBank>,
    // TODO: Add font
}

impl Command for N9Config {
    fn apply(self, world: &mut World) {
        let layouts = {
            let mut layout_assets = world.resource_mut::<Assets<TextureAtlasLayout>>();
            self.sprite_sheets.iter().map(|sheet|
                                      if let Some((size, counts)) = sheet.sprite_size.zip(sheet.sprite_counts) {
                                      Some(layout_assets.add(TextureAtlasLayout::from_grid(
                                          size,
                                          counts.x,
                                          counts.y,
                                          None,
                                          None)))
                                      } else {
                                          None
                                      }).collect();
        };




        let asset_server = world.resource::<AssetServer>();
        // insert the right Pico8State, right?
            // It's available to load.
            let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
                // Use `nearest` image sampling to preserve the pixel art style.
                settings.sampler = ImageSampler::nearest();
            };
            let state = pico8::Pico8State {
                palette: asset_server.load_with_settings(self.palette.as_deref().unwrap_or(pico8::PICO8_PALETTE), pixel_art_settings),
                border: asset_server.load_with_settings(pico8::PICO8_BORDER, pixel_art_settings),
                maps: vec![].into(),//vec![pico8::Map { entries: cart.map.clone(), sheet_index: 0 }].into(),
                audio_banks: self.audio_banks.into_iter().map(|bank| pico8::AudioBank(match bank {
                    AudioBank::P8 { p8, count } => {
                            (0..count).map(|i| pico8::Audio::Sfx(asset_server.load(AssetPath::from_path(&p8).with_label(format!("sfx{i}"))))).collect()
                    }
                    AudioBank::Paths { paths } => {
                        paths.into_iter().map(|p| pico8::Audio::AudioSource(asset_server.load(p))).collect::<Vec<pico8::Audio>>()
                    }
                })).collect::<Vec<_>>().into(),

                        // vec![AudioBank(cart.sfx.clone().into_iter().map(Audio::Sfx).collect())].into(),
                sprite_sheets: self.sprite_sheets.into_iter().zip(layouts.into_iter()).map(|(sheet, layout)| pico8::SpriteSheet {
                        handle: asset_server.load(sheet.path),
                        size: sheet.size,
                        flags: vec![],
                        layout: layout.unwrap_or(Handle::default()),
                    }).collect().into(),
                //vec![SpriteSheet { handle: cart.sprites.clone(), size: UVec2::splat(8), flags: cart.flags.clone() }].into(),
                // cart: Some(load_cart.0.clone()),
                // layout: layouts.add(TextureAtlasLayout::from_grid(
                //     PICO8_SPRITE_SIZE,
                //     PICO8_TILE_COUNT.x,
                //     PICO8_TILE_COUNT.y,
                //     None,
                //     None,
                // )),
                // code: cart.lua.clone(),
                draw_state: crate::DrawState::default(),
                font: self.fonts.into_iter().map(|font| pico8::N9Font {
                    handle: asset_server.load(font.path),
                    height: None,
                }).collect().into(),
            };
            world.insert_resource(state);


    }
}

impl N9Config {
    pub fn pico8() -> Self {
        let mut config = N9Config::default();
        config.inject_pico8();
        config
        // Self {
        //     frames_per_second: Some(30),
        //     screen: Some(Screen {
        //         canvas_size: UVec2::splat(128),
        //         screen_size: Some(UVec2::splat(512)),
        //     }),
        //     palette: Some(pico8::PICO8_PALETTE.into()),
        //     ..default()
        // }
    }

    pub fn inject_template(mut self) -> Self {
        if let Some(ref template) = self.template {
            match template.deref() {
                "gameboy" => { self.inject_gameboy() },
                "pico8" => { self.inject_pico8() },
                x => { panic!("No template {x:?}") },
            }
            self
        } else {
            self
        }
    }

    pub fn with_default_font(mut self) -> Self {
        if self.fonts.is_empty() {
            self.fonts.push(Font::Default { default: true });
        }
        self
    }

    pub fn inject_pico8(&mut self) {
        if self.frames_per_second.is_none() {
            self.frames_per_second = Some(30);
        }
        if self.screen.is_none() {
            self.screen = Some(Screen {
                canvas_size: UVec2::splat(128),
                screen_size: Some(UVec2::splat(512)),
            });
        }
        if self.palette.is_none() {
            self.palette = Some(pico8::PICO8_PALETTE.into());
        }
    }

    pub fn inject_gameboy(&mut self) {
        if self.frames_per_second.is_none() {
            self.frames_per_second = Some(60);
        }
        if self.screen.is_none() {
            self.screen = Some(Screen {
                canvas_size: UVec2::new(240, 160),
                screen_size: Some(UVec2::new(480, 320)),
            });
        }
        if self.palette.is_none() {
            //['#9bbc0f', '#77a112', '#306230', '#0f380f'],
            self.palette = Some("images/gameboy-palettes.png".into());
        }
    }

    pub fn gameboy() -> Self {
        let mut config = N9Config::default();
        config.inject_gameboy();
        config
        // Self {
        //     frames_per_second: Some(60),
        //     screen: Some(Screen {
        //         canvas_size: UVec2::new(240, 160),
        //         screen_size: Some(UVec2::new(480, 320)),
        //     }),
        //     ..default()
        //     // palette: Some(PICO8_PALETTE.into()),
        // }
    }

    pub fn load_config(&self, asset_server: Res<AssetServer>, mut commands: Commands) {
        let palette: Option<Handle<Image>> = self.palette.as_deref().map(|path| asset_server.load(path));
        let sprite_sheets: Vec<pico8::SpriteSheet> = self.sprite_sheets.iter().map(|sprite_sheet| pico8::SpriteSheet {
            handle: asset_server.load(&sprite_sheet.path),
            size: sprite_sheet.sprite_size,
            flags: Vec::new(),
        }).collect();

        // let cart: Handle<Cart> = asset_server.load(&script_path);
        // commands.send_event(LoadCart(cart));
        // commands.spawn(ScriptComponent(
        //     vec![format!("{}#lua", &script_path).into()],
        // ));
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AudioBank {
    // #[serde(rename = "p8")]
    P8 { p8: PathBuf, count: usize },
    // #[serde(rename = "paths")]
    Paths { paths: Vec<PathBuf> }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Screen {
    pub canvas_size: UVec2,
    pub screen_size: Option<UVec2>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SpriteSheet {
    pub path: String,
    pub sprite_size: Option<UVec2>,
    pub sprite_counts: Option<UVec2>,
}


#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Font {
    Default { default: bool },
    Path { path: String, height: Option<f32> },
    // pub path: String,
    // pub height: Option<f32>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_0() {
        let config: N9Config = toml::from_str(r#"
sprite_sheet = []
"#).unwrap();
        assert_eq!(config.sprite_sheets.len(), 0);
        assert!(config.screen.is_none());
    }

    #[test]
    fn test_config_1() {
        let config: N9Config = toml::from_str(r#"
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [8, 8]
"#).unwrap();
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, "sprites.png");
        assert_eq!(config.sprite_sheets[0].sprite_size, UVec2::splat(8));
    }

    #[test]
    fn test_config_2() {
        let config: N9Config = toml::from_str(r#"
[screen]
canvas_size = [128,128]
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [8, 8]
"#).unwrap();
        assert_eq!(config.screen.map(|s| s.canvas_size), Some(UVec2::splat(128)));
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, "sprites.png");
        assert_eq!(config.sprite_sheets[0].sprite_size, UVec2::splat(8));
    }

    #[test]
    fn test_config_3() {
        let config: N9Config = toml::from_str(r#"
[[audio_bank]]
p8 = "blah.p8"
"#).unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(config.audio_banks[0], AudioBank::P8("blah.p8".into()));
    }

    #[test]
    fn test_config_4() {
        let config: N9Config = toml::from_str(r#"
[[audio_bank]]
paths = [
"blah.mp3"
]
"#).unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(config.audio_banks[0], AudioBank::Paths(vec!["blah.mp3".into()]));
    }

    #[test]
    fn test_config_5() {
        let config: N9Config = toml::from_str(r#"
[[font]]
path = "blah.tff"
[[font]]
path = "dee.tff"
height = 3.0
[[font]]
default = true
"#).unwrap();
        assert_eq!(config.fonts.len(), 3);
        // assert_eq!(config.fonts[0].path, "blah.tff");
    }

}
