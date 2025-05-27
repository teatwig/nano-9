mod memory_dir;

pub use memory_dir::*;
mod loader;
pub use loader::*;
pub mod front_matter;
#[cfg(feature = "level")]
use crate::level::{self, asset::TiledSet, tiled::*};
use crate::{
    error::RunState,
    pico8::{self, image::pixel_art_settings, Gfx, Pico8Asset, Pico8Handle, Pico8State},
};
use bevy::{
    asset::{embedded_asset, io::Reader, AssetLoader, AssetPath, LoadContext},
    prelude::*,
};
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{asset::ScriptAssetSettings, script::ScriptComponent};
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, io, path::PathBuf};

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "gameboy-palettes.png");
    embedded_asset!(app, "gameboy.ttf");
    app
        // .register_type::<AudioBank>()
        // .register_type::<SpriteSheet>()
        .add_systems(Update, update_asset)
        .add_plugins(loader::plugin);
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub name: Option<String>,
    pub frames_per_second: Option<u8>,
    pub description: Option<String>,
    pub template: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub screen: Option<Screen>,
    #[serde(default, rename = "palette")]
    pub palettes: Vec<Palette>,
    // pub nearest_sampling: Option<bool>,
    #[serde(default, rename = "font")]
    pub fonts: Vec<Font>,
    #[serde(default, rename = "image")]
    pub sprite_sheets: Vec<SpriteSheet>,
    #[cfg(feature = "scripting")]
    pub code: Option<PathBuf>,
    #[serde(default, rename = "audio_bank")]
    pub audio_banks: Vec<AudioBank>,
    #[serde(default, rename = "map")]
    pub maps: Vec<Map>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AudioBank {
    // #[serde(rename = "p8")]
    P8 { p8: PathBuf, count: usize },
    // #[serde(rename = "paths")]
    Paths { paths: Vec<PathBuf> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Screen {
    pub canvas_size: UVec2,
    pub screen_size: Option<UVec2>,
}

// #[derive(Debug, Clone, Deserialize)]
// #[serde(untagged)]
// pub enum Sprite {
//     Sheet { sheet: SpriteSheet },
//     Tiled { path: PathBuf },
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteSheet {
    pub path: PathBuf,
    pub sprite_size: Option<UVec2>,
    pub sprite_counts: Option<UVec2>,
    pub padding: Option<UVec2>,
    pub offset: Option<UVec2>,
    #[serde(default)]
    pub indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// #[serde(untagged)]
pub struct Map {
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Font {
    Default { default: bool },
    Path { path: String, height: Option<f32> },
    // pub path: String,
    // pub height: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Palette {
    pub path: String,
    pub row: Option<u32>,
}


pub fn update_asset(
    mut reader: EventReader<AssetEvent<pico8::Pico8Asset>>,
    mut assets: ResMut<Assets<pico8::Pico8Asset>>,

    mut next_state: ResMut<NextState<RunState>>,
    mut pico8_state: ResMut<Pico8State>,
    mut pico8_handle: Option<ResMut<Pico8Handle>>,
    #[cfg(feature = "scripting")]
    mut commands: Commands,
    #[cfg(feature = "scripting")] script_settings: Res<ScriptAssetSettings>,
) {
    for e in reader.read() {
        info!("update asset event {e:?}");
        if let AssetEvent::LoadedWithDependencies { id } = e {
            if let Some(ref mut pico8_handle) = pico8_handle {
                if let Some(pico8_asset) = assets.get(*id) {
                    if pico8_handle.handle.id() != *id {
                        warn!("Script loaded but does not match Pico8Handle.");
                        continue;
                    }
                    // XXX: It happens here too!
                    #[cfg(feature = "scripting")]
                    {
                        if let Some(code) = &pico8_asset.code {
                            if pico8_handle.script_component.is_none() {
                                let path: &AssetPath<'static> = code.path().unwrap();
                                let script_path = (script_settings.script_id_mapper.map)(path);
                                info!("Add script component path {}", &script_path);
                                pico8_handle.script_component =
                                    Some(commands.spawn(ScriptComponent(vec![script_path])).id());
                            }
                        }
                    }
                    info!("Goto Loaded state");
                    next_state.set(RunState::Loaded);
                } else {
                    error!("Pico8Asset not available.");
                }
            } else {
                warn!("Script loaded but no Pico8Handle is loaded.");
            }
        }
    }
}

pub fn run_pico8_when_loaded(state: Res<State<RunState>>, mut next_state: ResMut<NextState<RunState>>) {
    match **state {
        RunState::Loaded => {
            next_state.set(RunState::Init);
        }
        RunState::Init => {
            next_state.set(RunState::Run);
        }
        _ => ()
    }
}

impl Config {
    pub fn pico8() -> Self {
        let mut config = Config::default();
        config.inject_pico8();
        config
    }

    pub fn inject_template(&mut self, template_name: &str) -> Result<(), ConfigLoaderError> {
        match template_name {
            "gameboy" => self.inject_gameboy(),
            "pico8" => self.inject_pico8(),
            x => {
                return Err(ConfigLoaderError::InvalidTemplate(x.to_string()));
            }
        }
        Ok(())
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
        if self.palettes.is_empty() {
            self.palettes.push(Palette {
                path: pico8::PICO8_PALETTE.into(),
                row: None,
            });
        }
        if self.fonts.is_empty() {
            self.fonts.push(Font::Path {
                path: pico8::PICO8_FONT.into(),
                height: None,
            });
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
        if self.palettes.is_empty() {
            self.palettes.push(Palette {
                path: "embedded://nano9/config/gameboy-palettes.png".into(),
                row: Some(15),
            });
        }

        if self.fonts.is_empty() {
            self.fonts.push(Font::Path {
                path: "embedded://nano9/config/gameboy.ttf".into(),
                height: None,
            });
        }
    }

    pub fn gameboy() -> Self {
        let mut config = Config::default();
        config.inject_gameboy();
        config
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_config_0() {
        let config: Config = toml::from_str(
            r#"
image = []
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets.len(), 0);
        assert!(config.screen.is_none());
    }

    #[test]
    fn test_config_1() {
        let config: Config = toml::from_str(
            r#"
[[image]]
path = "sprites.png"
sprite_size = [8, 8]
"#,
        )
        .unwrap();
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, Path::new("sprites.png"));
        assert_eq!(config.sprite_sheets[0].sprite_size, Some(UVec2::splat(8)));
    }

    #[test]
    fn test_palete_0() {
        let config: Config = toml::from_str(
            r#"
[[palette]]
path = "sprites.png"
"#,
        )
        .unwrap();
        assert_eq!(
            config.palettes,
            vec![Palette {
                path: "sprites.png".into(),
                row: None
            }]
        );
    }

    #[test]
    fn test_config_2() {
        let config: Config = toml::from_str(
            r#"
[screen]
canvas_size = [128,128]
[[image]]
path = "sprites.png"
sprite_size = [8, 8]
"#,
        )
        .unwrap();
        assert_eq!(
            config.screen.map(|s| s.canvas_size),
            Some(UVec2::splat(128))
        );
        assert_eq!(config.sprite_sheets.len(), 1);
        assert_eq!(config.sprite_sheets[0].path, Path::new("sprites.png"));
        assert_eq!(config.sprite_sheets[0].sprite_size, Some(UVec2::splat(8)));
    }

    #[test]
    fn test_config_3() {
        let config: Config = toml::from_str(
            r#"
[[audio_bank]]
p8 = "blah.p8"
count = 1
"#,
        )
        .unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(
            config.audio_banks[0],
            AudioBank::P8 {
                p8: "blah.p8".into(),
                count: 1
            }
        );
    }

    #[test]
    fn test_config_4() {
        let config: Config = toml::from_str(
            r#"
[[audio_bank]]
paths = [
"blah.mp3"
]
"#,
        )
        .unwrap();
        assert_eq!(config.audio_banks.len(), 1);
        assert_eq!(
            config.audio_banks[0],
            AudioBank::Paths {
                paths: vec!["blah.mp3".into()]
            }
        );
    }

    #[test]
    fn test_config_5() {
        let config: Config = toml::from_str(
            r#"
[[font]]
path = "blah.tff"
[[font]]
path = "dee.tff"
height = 3.0
[[font]]
default = true
"#,
        )
        .unwrap();
        assert_eq!(config.fonts.len(), 3);
        // assert_eq!(config.fonts[0].path, "blah.tff");
    }

    #[test]
    #[cfg(feature = "level")]
    fn test_config_6() {
        let config: Config = toml::from_str(
            r#"
[[map]]
path = "blah.ldtk"
[[map]]
path = "blah.p8"
"#,
        )
        .unwrap();
        assert_eq!(config.maps.len(), 2);
        assert_eq!(config.maps[0].path, PathBuf::from("blah.ldtk"));
    }
}
