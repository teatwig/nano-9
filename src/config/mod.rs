mod memory_dir;

pub use memory_dir::*;
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
        .init_asset_loader::<ConfigLoader>()
        .add_systems(Update, update_asset);
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoaderError {
    #[error("Could not load config file: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// An [IO](std::io) Error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
    #[error("Could not load dependency: {0}")]
    Load(#[from] bevy::asset::LoadDirectError),
    #[error("Could not read asset: {0}")]
    AssetBytes(#[from] bevy::asset::ReadAssetBytesError),
    #[error("Decoding error: {0}")]
    Decoding(#[from] png::DecodingError),
    #[error("image {image_index} ({image_size:?}) does not fit sprite size {sprite_size:?}")]
    InvalidSpriteSize {
        image_index: usize,
        image_size: UVec2,
        sprite_size: UVec2,
    },
    #[error("image {image_index} ({image_size:?}) does not fit sprite counts {sprite_counts:?}")]
    InvalidSpriteCounts {
        image_index: usize,
        image_size: UVec2,
        sprite_counts: UVec2,
    },
    #[error("invalid template: {0}")]
    InvalidTemplate(String),
}

#[derive(Default)]
pub struct ConfigLoader;

impl AssetLoader for ConfigLoader {
    type Asset = pico8::Pico8Asset;
    type Settings = ();
    type Error = ConfigLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes).await?;
        let content = std::str::from_utf8(&bytes)?;
        let mut config: Config = toml::from_str::<Config>(content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
        if let Some(template) = config.template.take() {
            config.inject_template(&template)?;
        }
        let mut sprite_sheets = vec![];
        for (i, mut sheet) in config.sprite_sheets.into_iter().enumerate() {
            // let flags: Vec<u8>;
            if sheet.path.extension() == Some(OsStr::new("tsx")) {
                #[cfg(feature = "level")]
                {
                    let tiledset = load_context
                        .loader()
                        .immediate()
                        .load::<TiledSet>(&*sheet.path)
                        .await?;
                    let tileset = &tiledset.get().0;
                    let handle = load_context
                        .add_labeled_asset(format!("atlas{i}"), layout_from_tileset(tileset));
                    let tile_size = UVec2::new(tileset.tile_width, tileset.tile_height);
                    if let Some(sprite_size) = sheet.sprite_size {
                        assert_eq!(sprite_size, tile_size);
                    }
                    let flags = flags_from_tileset(tileset);
                    sprite_sheets.push(pico8::SpriteSheet {
                        handle: pico8::SprHandle::Image(
                            load_context
                                .loader()
                                .with_settings(pixel_art_settings)
                                .load(
                                    &*tileset
                                        .image
                                        .as_ref()
                                        .ok_or(ConfigLoaderError::Message(format!(
                                            "could not load .tsx image {i}"
                                        )))?
                                        .source,
                                ),
                        ),
                        sprite_size: tile_size,
                        flags,
                        layout: handle,
                    })
                }
                #[cfg(not(feature = "level"))]
                panic!(
                    "Can not load {:?} file without 'level' feature.",
                    &sheet.path
                );
            } else if sheet.path.extension() == Some(OsStr::new("p8")) {
                todo!()
            } else {
                let (handle, layout_maybe) = if sheet.indexed {
                    let bytes = load_context.read_asset_bytes(&*sheet.path).await?;
                    let gfx = Gfx::from_png(&bytes)?;
                    let image_size = UVec2::new(gfx.width as u32, gfx.height as u32);
                    let layout = get_layout(
                        i,
                        image_size,
                        &mut sheet.sprite_size,
                        sheet.sprite_counts,
                        sheet.padding,
                        sheet.offset,
                    )?
                    .map(|layout| load_context.add_labeled_asset(format!("atlas{i}"), layout));
                    (
                        pico8::SprHandle::Gfx(
                            load_context.add_labeled_asset(format!("spritesheet{i}"), gfx),
                        ),
                        layout,
                    )
                } else {
                    let loaded = load_context
                        .loader()
                        .immediate()
                        .with_settings(pixel_art_settings)
                        .load::<Image>(&*sheet.path)
                        .await?;
                    let image_size = loaded.get().size();
                    let layout = get_layout(
                        i,
                        image_size,
                        &mut sheet.sprite_size,
                        sheet.sprite_counts,
                        sheet.padding,
                        sheet.offset,
                    )?
                    .map(|layout| load_context.add_labeled_asset(format!("atlas{i}"), layout));

                    (
                        pico8::SprHandle::Image(
                            load_context
                                .add_loaded_labeled_asset(format!("spritesheet{i}"), loaded),
                        ),
                        layout,
                    )
                };
                sprite_sheets.push(pico8::SpriteSheet {
                    handle,
                    sprite_size: sheet.sprite_size.unwrap_or(UVec2::splat(8)),
                    flags: vec![],
                    layout: layout_maybe.unwrap_or(Handle::default()),
                })
            }
        }
        let mut palettes = Vec::new();
        if config.palettes.is_empty() {
            warn!("No palettes were provided.");
            // XXX: Should we provide a default pico8 palette?
            // config.palettes.push(Palette { path: pico8::PICO8_PALETTE.to_string(), row: None });
        } else {
            palettes = Vec::with_capacity(config.palettes.len());
            for palette in config.palettes.iter() {
                let image = load_context
                    .loader()
                    .immediate()
                    .with_settings(pixel_art_settings)
                    .load(&palette.path)
                    .await?;
                palettes.push(pico8::Palette::from_image(image.get(), palette.row));
            }
        }
        let state = pico8::Pico8Asset {
#[cfg(feature = "scripting")]
                code: config.code.map(|p| load_context.load(&*p)),
                palettes: palettes.into(),
                border: load_context.loader()
                                    .with_settings(pixel_art_settings)
                                    .load(pico8::PICO8_BORDER),
                maps: config.maps.into_iter().map(|map| {
                    let extension = map.path.extension().and_then(|s| s. to_str());
                    if let Some(ext) = extension {
                        match ext {
                            "p8" => todo!(),
                            "tmx" => {
                                    #[cfg(feature = "level")]
                                    return Ok(level::Tiled::Map {
                                        handle: load_context.load(&*map.path),
                                    }.into());
                                    #[cfg(not(feature = "level"))]
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled map; consider using the '--features=level' flag.", &map.path)))
                            }
                            "world" => {
                                    #[cfg(feature = "level")]
                                    return Ok(level::Tiled::World {
                                        handle: load_context.load(&*map.path),
                                    }.into());
                                    #[cfg(not(feature = "level"))]
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled world; consider using the '--features=level' flag.", &map.path)))
                            }
                            _ => Err(ConfigLoaderError::Message(format!("Unknown map format {:?}", &map.path)))
                        }
                    } else {
                        Err(ConfigLoaderError::Message(format!("The map path {:?} did not have an extension.", &map.path)))
                    }
                }).collect::<Result<Vec<_>, _>>()?.into(),
                audio_banks: config.audio_banks.into_iter().map(|bank| pico8::audio::AudioBank(match bank {
                    AudioBank::P8 { p8, count } => {
                            (0..count).map(|i|
                                           pico8::audio::Audio::Sfx(load_context.load(AssetPath::from_path(&p8).into_owned().with_label(format!("sfx{i}"))))
                            ).collect::<Vec<_>>()
                    }
                    AudioBank::Paths { paths } => {
                        paths.into_iter().map(|p| pico8::audio::Audio::AudioSource(load_context.load(p))).collect::<Vec<_>>()
                    }
                })).collect::<Vec<_>>().into(),
                sprite_sheets: sprite_sheets.into(),
                font: config.fonts.into_iter().map(|font|
                                                     match font {
                                                         Font::Default { default: yes } if yes => {
                                                             pico8::N9Font {
                                                                 handle: TextFont::default().font,
                                                             }
                                                         },
                                                         Font::Path { path, height: _ } => {
                                                             pico8::N9Font {
                                                                 handle: load_context.load(path),
                                                             }
                                                         }
                                                         Font::Default { .. } => { panic!("Must use a path if not default font.") }
                                                     }).collect::<Vec<_>>().into(),

            };
        Ok(state)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["toml"];
        EXTENSIONS
    }
}

fn get_layout(
    image_index: usize,
    image_size: UVec2,
    sprite_size: &mut Option<UVec2>,
    sprite_counts: Option<UVec2>,
    padding: Option<UVec2>,
    offset: Option<UVec2>,
) -> Result<Option<TextureAtlasLayout>, ConfigLoaderError> {
    if let Some((size, counts)) = sprite_size.zip(sprite_counts) {
        Ok(Some(TextureAtlasLayout::from_grid(
            size, counts.x, counts.y, padding, offset,
        )))
    } else if let Some(sprite_size) = *sprite_size {
        let counts = image_size / sprite_size;
        let remainders = image_size % sprite_size;
        if remainders == UVec2::ZERO {
            Ok(Some(TextureAtlasLayout::from_grid(
                sprite_size,
                counts.x,
                counts.y,
                padding,
                offset,
            )))
        } else {
            Err(ConfigLoaderError::InvalidSpriteSize {
                image_index,
                image_size,
                sprite_size,
            })
        }
    } else if let Some(sprite_counts) = sprite_counts {
        let size = image_size / sprite_counts;
        *sprite_size = Some(size);
        let remainders = image_size % sprite_counts;
        if remainders == UVec2::ZERO {
            Ok(Some(TextureAtlasLayout::from_grid(
                size,
                sprite_counts.x,
                sprite_counts.y,
                padding,
                offset,
            )))
        } else {
            Err(ConfigLoaderError::InvalidSpriteCounts {
                image_index,
                image_size,
                sprite_counts,
            })
        }
    } else {
        Ok(None)
    }
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
