use bevy::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use crate::pico8;

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);

#[derive(Default, Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct N9Config {
    pub name: Option<String>,
    pub frames_per_second: Option<u8>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub screen: Option<Screen>,
    pub palette: Option<String>,
    #[serde(default, rename = "sprite_sheet")]
    pub sprite_sheets: Vec<SpriteSheet>,
    pub code: Option<PathBuf>,
    #[serde(default, rename = "audio_bank")]
    pub audio_banks: Vec<AudioBank>,
    // TODO: Add font
}

impl N9Config {
    pub fn pico8() -> Self {
        Self {
            frames_per_second: Some(30),
            screen: Some(Screen {
                canvas_size: UVec2::splat(128),
                screen_size: Some(UVec2::splat(512)),
            }),
            palette: Some(pico8::PICO8_PALETTE.into()),
            ..default()
        }
    }

    pub fn gameboy() -> Self {
        Self {
            frames_per_second: Some(60),
            screen: Some(Screen {
                canvas_size: UVec2::new(240, 160),
                screen_size: Some(UVec2::new(480, 320)),
            }),
            //['#9bbc0f', '#77a112', '#306230', '#0f380f'],
            ..default()
            // palette: Some(PICO8_PALETTE.into()),
        }
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
pub enum AudioBank {
    #[serde(rename = "p8")]
    P8(PathBuf),
    #[serde(rename = "paths")]
    Paths(Vec<PathBuf>)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Screen {
    pub canvas_size: UVec2,
    pub screen_size: Option<UVec2>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SpriteSheet {
    pub path: String,
    pub sprite_size: UVec2,
    pub sprite_counts: Option<UVec2>,
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
        assert_eq!(config.sprite_sheets[0].path.to_str(), Some("sprites.png"));
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
        assert_eq!(config.sprite_sheets[0].path.to_str(), Some("sprites.png"));
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

}
