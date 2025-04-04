#[cfg(feature = "level")]
use crate::level::{self, tiled::*};
use crate::{call, level::asset::TiledSet, pico8};
use bevy::{
    asset::{embedded_asset, io::AssetSourceId, io::Reader, AssetLoader, AssetPath, LoadContext},
    image::{ImageLoaderSettings, ImageSampler},
    prelude::*,
};
use bevy_mod_scripting::core::{
    bindings::script_value::ScriptValue, event::ScriptCallbackEvent, script::ScriptComponent,
};
use serde::Deserialize;
use std::{ffi::OsStr, io, ops::Deref, path::PathBuf};

pub const DEFAULT_CANVAS_SIZE: UVec2 = UVec2::splat(128);
pub const DEFAULT_SCREEN_SIZE: UVec2 = UVec2::splat(512);

pub(crate) fn plugin(app: &mut App) {
    embedded_asset!(app, "gameboy-palettes.png");
    embedded_asset!(app, "gameboy.ttf");
    app.init_asset_loader::<ConfigLoader>()
        .add_systems(Update, update_asset);
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct Config {
    pub name: Option<String>,
    pub frames_per_second: Option<u8>,
    pub description: Option<String>,
    pub template: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub screen: Option<Screen>,
    pub palette: Option<Palette>,
    #[serde(default, rename = "font")]
    pub fonts: Vec<Font>,
    #[serde(default, rename = "sprite_sheet")]
    pub sprite_sheets: Vec<SpriteSheet>,
    pub code: Option<PathBuf>,
    #[serde(default, rename = "audio_bank")]
    pub audio_banks: Vec<AudioBank>,
    #[serde(default, rename = "map")]
    pub maps: Vec<Map>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AudioBank {
    // #[serde(rename = "p8")]
    P8 { p8: PathBuf, count: usize },
    // #[serde(rename = "paths")]
    Paths { paths: Vec<PathBuf> },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SpriteSheet {
    pub path: PathBuf,
    pub sprite_size: Option<UVec2>,
    pub sprite_counts: Option<UVec2>,
}

#[derive(Debug, Clone, Deserialize)]
// #[serde(untagged)]
pub struct Map {
    path: PathBuf,
    // P8 { p8: PathBuf },
    // Ldtk { ldtk: PathBuf },
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Font {
    Default { default: bool },
    Path { path: String, height: Option<f32> },
    // pub path: String,
    // pub height: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct Palette {
    path: String,
    row: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoaderError {
    #[error("Could not load config file: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    /// An [IO](std::io) Error
    #[error("Could not load Tiled file: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
    #[error("Could not load dependency: {0}")]
    Load(#[from] bevy::asset::LoadDirectError),
}

#[derive(Default)]
pub struct ConfigLoader;

impl AssetLoader for ConfigLoader {
    type Asset = pico8::Pico8State;
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
        dbg!(&content);
        let config: Config = toml::from_str::<Config>(content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?
            .inject_template();
        dbg!(&config);
        dbg!(config.sprite_sheets.len());
        let mut layouts: Vec<Option<Handle<TextureAtlasLayout>>> = vec![];
        // let mut layout_assets = world.resource_mut::<Assets<TextureAtlasLayout>>();
        for (i, sheet) in config.sprite_sheets.iter().enumerate() {
            if sheet.path.extension() == Some(OsStr::new("tsx")) {
                #[cfg(feature = "level")]
                {
                    // let mut loader = tiled::Loader::new();
                    // let tileset = loader.load_tsx_tileset(&sheet.path).unwrap();
                    // Some(layout_assets.add(layout_from_tileset(&tileset)))
                    // Some(load_context.add_labeled_asset(format!("atlas{i}"), layout_from_tileset(&tileset)))
                    let tileset = load_context
                        .loader()
                        .immediate()
                        .load::<TiledSet>(&*sheet.path)
                        .await?;
                    layouts.push(Some(load_context.add_labeled_asset(
                        format!("atlas{i}"),
                        layout_from_tileset(&tileset.get().0),
                    )));
                }
                #[cfg(not(feature = "level"))]
                Err(ConfigLoaderError::Message(format!(
                    "Can not load {:?} file without 'level' feature.",
                    &sheet.path
                )))?;
            } else if let Some((size, counts)) = sheet.sprite_size.zip(sheet.sprite_counts) {
                layouts.push(Some(load_context.add_labeled_asset(
                    format!("atlas{i}"),
                    TextureAtlasLayout::from_grid(size, counts.x, counts.y, None, None),
                )))
            } else {
                layouts.push(None);
            }
        }
        dbg!(&layouts);
        let code_path = config.code.unwrap_or_else(|| "main.lua".into());

        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };

        let mut sprite_sheets = vec![];
        for (i, sheet) in config.sprite_sheets.into_iter().enumerate() {
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
                        handle: load_context
                            .loader()
                            .with_settings(pixel_art_settings)
                            .load(&*tileset.image.as_ref().expect("tileset image").source),
                        //load_context
                        // .load_with_settings(&*tileset.image.expect("tileset image").source,
                        //                     pixel_art_settings),
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
            } else {
                let layout =
                    if let Some((size, counts)) = sheet.sprite_size.zip(sheet.sprite_counts) {
                        Some(load_context.add_labeled_asset(
                            format!("atlas{i}"),
                            TextureAtlasLayout::from_grid(size, counts.x, counts.y, None, None),
                        ))
                    } else {
                        None
                    };
                sprite_sheets.push(pico8::SpriteSheet {
                    handle: load_context
                        .loader()
                        .with_settings(pixel_art_settings)
                        .load(&*sheet.path),
                    sprite_size: sheet.sprite_size.unwrap_or(UVec2::splat(8)),
                    flags: vec![],
                    layout: layout.unwrap_or(Handle::default()),
                })
            }
        }
        let state = pico8::Pico8State {
                code: load_context.load(&*code_path),
                palette: pico8::Palette {
                    handle: load_context.loader()
                                        .with_settings(pixel_art_settings)
                                        .load(config.palette.as_ref().map(|p| p.path.as_str()).unwrap_or(pico8::PICO8_PALETTE)),
                    row: config.palette.and_then(|p| p.row).unwrap_or(0),
                },
                border: load_context.loader()
                                    .with_settings(pixel_art_settings)
                                    .load(pico8::PICO8_BORDER),
                maps: config.maps.into_iter().map(|map| {
                    let extension = map.path.extension().and_then(|s| s. to_str());
                    if let Some(ext) = extension {
                        match ext {
                            "p8" => todo!(),
                            "tmx" => {
                                if cfg!(not(feature = "level")) {
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled map; consider using the '--features=level' flag.", &map.path)))
                                } else {
                                    Ok(level::Tiled::Map {
                                        handle: load_context.load(&*map.path),
                                    }.into())
                                }
                            }
                            "world" => {
                                if cfg!(not(feature = "level")) {
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled world; consider using the '--features=level' flag.", &map.path)))
                                } else {
                                    Ok(level::Tiled::World {
                                        handle: load_context.load(&*map.path),
                                    }.into())
                                }
                            }
                            x => Err(ConfigLoaderError::Message(format!("Unknown map format {:?}", &map.path)))
                        }
                    } else {
                        Err(ConfigLoaderError::Message(format!("The map path {:?} did not have an extension.", &map.path)))
                    }
                }).collect::<Result<Vec<_>, _>>()?.into(),
                audio_banks: config.audio_banks.into_iter().map(|bank| pico8::AudioBank(match bank {
                    AudioBank::P8 { p8, count } => {
                            // (0..count).map(|i|
                            //                pico8::Audio::Sfx(load_context.load(&AssetPath::from_path(&p8).with_label(&format!("sfx{i}"))))
                            // ).collect::<Vec<_>>()
                            vec![]
                    }
                    AudioBank::Paths { paths } => {
                        paths.into_iter().map(|p| pico8::Audio::AudioSource(load_context.load(p))).collect::<Vec<_>>()
                    }
                })).collect::<Vec<_>>().into(),
                sprite_sheets: sprite_sheets.into(),

                        // vec![AudioBank(cart.sfx.clone().into_iter().map(Audio::Sfx).collect())].into(),
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
                font: config.fonts.into_iter().map(|font|
                                                     match font {
                                                         Font::Default { default: yes } if yes => {
                                                             pico8::N9Font {
                                                                 handle: TextFont::default().font,
                                                                 height: None,
                                                             }
                                                         },
                                                         Font::Path { path, height: _ } => {
                                                             pico8::N9Font {
                                                                 handle: load_context.load(path),
                                                                 height: None,
                                                             }
                                                         }
                                                         Font::Default { .. } => { panic!("Must use a path if not default font.") }
                                                     }).collect::<Vec<_>>().into(),
            };
        Ok(state)
        //     world.insert_resource(state);
        // world.spawn(ScriptComponent(vec![code_path.path().to_str().unwrap().to_string().into()]));
        //
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["toml"];
        EXTENSIONS
    }
}

pub fn update_asset(
    mut reader: EventReader<AssetEvent<pico8::Pico8State>>,
    assets: Res<Assets<pico8::Pico8State>>,
    mut commands: Commands,
) {
    for e in reader.read() {
        info!("update asset event {e:?}");
        if let AssetEvent::LoadedWithDependencies { id } = e {
            if let Some(state) = assets.get(*id) {
                info!("Insert Pico8State {:?}.", state);
                commands.insert_resource(state.clone());
                commands.spawn(ScriptComponent(vec!["main.lua".into()]));
                commands.send_event(
                    // writer.send(
                    ScriptCallbackEvent::new_for_all(call::Init, vec![ScriptValue::Unit]), //);
                );
            } else {
                error!("Pico8State not available.");
            }
        }
    }
}

impl Command for Config {
    fn apply(self, world: &mut World) {
        let layouts: Vec<Option<Handle<TextureAtlasLayout>>> = {
            let mut layout_assets = world.resource_mut::<Assets<TextureAtlasLayout>>();
            self.sprite_sheets
                .iter()
                .map(|sheet| {
                    if sheet.path.extension() == Some(OsStr::new("tsx")) {
                        #[cfg(feature = "level")]
                        {
                            let mut loader = tiled::Loader::new();
                            let tileset = loader.load_tsx_tileset(&sheet.path).unwrap();
                            Some(layout_assets.add(layout_from_tileset(&tileset)))
                        }
                        #[cfg(not(feature = "level"))]
                        panic!(
                            "Can not load {:?} file without 'level' feature.",
                            &sheet.path
                        );
                    } else if let Some((size, counts)) = sheet.sprite_size.zip(sheet.sprite_counts) {
                        Some(layout_assets.add(TextureAtlasLayout::from_grid(
                            size, counts.x, counts.y, None, None,
                        )))
                    } else {
                        None
                    }
                })
                .collect()
        };
        // let source = AssetSourceId::Name("nano9".into());
        let source = AssetSourceId::Default;
        let code_path = self.code.unwrap_or_else(|| "main.lua".into());
        let code_path = AssetPath::from_path(&code_path).with_source(&source);
        let asset_server = world.resource::<AssetServer>();
        // insert the right Pico8State, right?
        // It's available to load.
        let pixel_art_settings = |settings: &mut ImageLoaderSettings| {
            // Use `nearest` image sampling to preserve the pixel art style.
            settings.sampler = ImageSampler::nearest();
        };
        let state = pico8::Pico8State {
                code: asset_server.load(&code_path),
                palette: pico8::Palette {
                    handle: asset_server.load_with_settings(self.palette.as_ref().map(|p| p.path.as_str()).unwrap_or(pico8::PICO8_PALETTE), pixel_art_settings),
                    row: self.palette.and_then(|p| p.row).unwrap_or(0),
                },
                border: asset_server.load_with_settings(pico8::PICO8_BORDER, pixel_art_settings),
                maps: self.maps.into_iter().map(|map| {
                    let extension = map.path.extension().and_then(|s| s. to_str());
                    if let Some(ext) = extension {
                        match ext {
                            "p8" => todo!(),
                            "tmx" => {
                                if cfg!(not(feature = "level")) {
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled map; consider using the '--features=level' flag.", &map.path)))
                                } else {
                                    Ok(level::Tiled::Map {
                                        handle: asset_server.load(AssetPath::from_path(&map.path).with_source(&source)),
                                    }.into())
                                }
                            }
                            "world" => {
                                if cfg!(not(feature = "level")) {
                                    Err(ConfigLoaderError::Message(format!("The map {:?} is a Tiled world; consider using the '--features=level' flag.", &map.path)))
                                } else {
                                    Ok(level::Tiled::World {
                                        handle: asset_server.load(AssetPath::from_path(&map.path).with_source(&source)),
                                    }.into())
                                }
                            }
                            x => todo!("No idea how to load map {:?}", x),
                        }
                    } else {
                        Err(ConfigLoaderError::Message(format!("The map path {:?} did not have an extension.", &map.path)))
                    }
                }).collect::<Result<Vec<_>, _>>().expect("load map").into(),
                audio_banks: self.audio_banks.into_iter().map(|bank| pico8::AudioBank(match bank {
                    AudioBank::P8 { p8, count } => {
                        // let asset_path = AssetPath::from_path(&p8);
                        //     (0..count).map(|i| {
                        //         let label = format!("sfx{i}");
                        //         pico8::Audio::Sfx(asset_server.load(&asset_path.clone().with_label(&label)))
                        //     }).collect::<Vec<_>>()
                        vec![]
                    }
                    AudioBank::Paths { paths } => {
                        paths.into_iter().map(|p| pico8::Audio::AudioSource(asset_server.load(p))).collect::<Vec<_>>()
                    }
                })).collect::<Vec<_>>().into(),

                        // vec![AudioBank(cart.sfx.clone().into_iter().map(Audio::Sfx).collect())].into(),
                sprite_sheets: self.sprite_sheets.into_iter().zip(layouts).map(|(sheet, layout)| {
                    let flags: Vec<u8>;
                    if sheet.path.extension() == Some(OsStr::new("tsx")) {
                        #[cfg(feature = "level")]
                        {
                            let mut loader = tiled::Loader::new();
                            let tileset = loader.load_tsx_tileset(&sheet.path).unwrap();
                            let tile_size = UVec2::new(tileset.tile_width, tileset.tile_height);
                            if let Some(sprite_size) = sheet.sprite_size {
                                assert_eq!(sprite_size, tile_size);
                            }
                            let flags = flags_from_tileset(&tileset);
                            pico8::SpriteSheet {
                                handle: asset_server.load_with_settings(&*tileset.image.expect("tileset image").source,
                                                                        pixel_art_settings),
                                sprite_size: tile_size,
                                flags,
                                layout: layout.unwrap_or(Handle::default()),
                            }
                        }
                        #[cfg(not(feature = "level"))]
                        panic!("Can not load {:?} file without 'level' feature.", &sheet.path);
                    } else {
                        let mut path = AssetPath::from_path(&sheet.path);
                        if *path.source() == AssetSourceId::Default {
                            path = path.with_source(&source);
                        }
                        dbg!(&layout);
                        pico8::SpriteSheet {
                            handle: asset_server.load_with_settings(path, pixel_art_settings),
                            sprite_size: sheet.sprite_size.unwrap_or(UVec2::splat(8)),
                            flags: vec![],
                            layout: layout.unwrap_or(Handle::default()),
                        }
                    }
                }).collect::<Vec<_>>().into(),
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
                font: self.fonts.into_iter().map(|font|
                                                     match font {
                                                         Font::Default { default: yes } if yes => {
                                                             pico8::N9Font {
                                                                 handle: TextFont::default().font,
                                                                 height: None,
                                                             }
                                                         },
                                                         Font::Path { path, height } => {

                                                             pico8::N9Font {
                                                                 handle: asset_server.load(path),
                                                                 height: None,
                                                             }
                                                         }
                                                         Font::Default { .. } => { panic!("Must use a path if not default font.") }
                                                     }).collect::<Vec<_>>().into(),
            };
        world.insert_resource(state);
        world.spawn(ScriptComponent(vec![code_path
            .path()
            .to_str()
            .unwrap()
            .to_string()
            .into()]));
    }
}

impl Config {
    pub fn pico8() -> Self {
        let mut config = Config::default();
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
                "gameboy" => self.inject_gameboy(),
                "pico8" => self.inject_pico8(),
                x => {
                    panic!("No template {x:?}")
                }
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
                // screen_size: Some(UVec2::splat(128)),
            });
        }
        if self.palette.is_none() {
            self.palette = Some(Palette {
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
        if self.palette.is_none() {
            self.palette = Some(Palette {
                path: "embedded://nano_9/config/gameboy-palettes.png".into(),
                row: Some(17),
            });
        }

        if self.fonts.is_empty() {
            self.fonts.push(Font::Path {
                path: "embedded://nano_9/config/gameboy.ttf".into(),
                height: None,
            });
        }
    }

    pub fn gameboy() -> Self {
        let mut config = Config::default();
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

    // pub fn load_config(&self, asset_server: Res<AssetServer>, mut commands: Commands) {
    //     let palette: Option<Handle<Image>> = self.palette.as_deref().map(|path| asset_server.load(path));
    //     let sprite_sheets: Vec<pico8::SpriteSheet> = self.sprite_sheets.iter().map(|sprite_sheet| pico8::SpriteSheet {
    //         handle: asset_server.load(&sprite_sheet.path),
    //         sprite_size: sprite_sheet.sprite_size.unwrap_or(UVec2::splat(8)),
    //         flags: Vec::new(),
    //     }).collect();

    //     // let cart: Handle<Cart> = asset_server.load(&script_path);
    //     // commands.send_event(LoadCart(cart));
    //     // commands.spawn(ScriptComponent(
    //     //     vec![format!("{}#lua", &script_path).into()],
    //     // ));
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_config_0() {
        let config: Config = toml::from_str(
            r#"
sprite_sheet = []
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
[[sprite_sheet]]
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
    fn test_config_2() {
        let config: Config = toml::from_str(
            r#"
[screen]
canvas_size = [128,128]
[[sprite_sheet]]
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
