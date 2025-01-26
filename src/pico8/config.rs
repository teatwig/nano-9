use bevy::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct N9Config {
    name: Option<String>,
    description: Option<String>,
    author: Option<String>,
    license: Option<String>,
    screen: Option<Screen>,
    palette: Option<PathBuf>,
    #[serde(default, rename = "sprite_sheet")]
    sprite_sheets: Vec<SpriteSheet>,
    code: Option<PathBuf>,
    #[serde(default, rename = "audio_bank")]
    audio_banks: Vec<AudioBank>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
enum AudioBank {
    P8(PathBuf),
    Paths(Vec<PathBuf>)
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct Screen {
    pixel_count: UVec2,
    screen_size: Option<UVec2>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct SpriteSheet {
    path: PathBuf,
    sprite_size: UVec2,
    sprite_counts: Option<UVec2>,
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
pixel_count = [128,128]
[[sprite_sheet]]
path = "sprites.png"
sprite_size = [8, 8]
"#).unwrap();
        assert_eq!(config.screen.map(|s| s.pixel_count), Some(UVec2::splat(128)));
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path.to_str(), Some("sprites.png"));
        assert_eq!(config.sprite_sheets[0].sprite_size, UVec2::splat(8));
    }

    #[test]
    fn test_config_3() {
        let config: N9Config = toml::from_str(r#"
[[audio_bank]]
P8 = "blah.p8"
"#).unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(config.audio_banks[0], AudioBank::P8("blah.p8".into()));
    }

    #[test]
    fn test_config_4() {
        let config: N9Config = toml::from_str(r#"
[[audio_bank]]
Paths = [
"blah.mp3"
]
"#).unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(config.audio_banks[0], AudioBank::Paths(vec!["blah.mp3".into()]));
    }

}
