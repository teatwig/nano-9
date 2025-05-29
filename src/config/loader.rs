#[cfg(feature = "level")]
use crate::level::{self, asset::TiledSet, tiled::*};
use crate::{
    config::{self, *},
    error::RunState,
    pico8::{self, image::pixel_art_settings, Gfx, Pico8Asset, Pico8Handle, Pico8State},
};
use bevy::{
    asset::{embedded_asset, io::Reader, AssetLoader, AssetPath, LoadContext},
    prelude::*,
};
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{asset::{ScriptAsset, ScriptAssetSettings}, script::ScriptComponent};
use serde::{Deserialize, Serialize};
use std::{ffi::OsStr, io, path::PathBuf};

pub(crate) fn plugin(app: &mut App) {
    app
        .init_asset_loader::<ConfigLoader>()
        .init_asset_loader::<LuaLoader>();
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigLoaderError {
    #[error("Could not read str: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Could not read string: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
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
    #[error("invalid template {0:?}")]
    InvalidTemplate(String),
    #[error("include error: {0}")]
    Cart(#[from] pico8::CartLoaderError),
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
        into_asset(config, load_context).await
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["toml"];
        EXTENSIONS
    }
}

#[derive(Default)]
pub struct LuaLoader;

impl AssetLoader for LuaLoader {
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
        let mut content = String::from_utf8(bytes)?;

        let config = if let Some(front_matter) = front_matter::LUA.parse_in_place(&mut content) {
            let mut config: Config = toml::from_str::<Config>(&front_matter)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            if let Some(template) = config.template.take() {
                config.inject_template(&template)?;
            }
            config
        } else {
            Config::pico8()
        };
        let mut asset = into_asset(config, load_context).await?;
        assert!(asset.code.is_none());

        let code_path: PathBuf = load_context.path().into();
        let code = content;
        asset.code = Some(load_context.add_labeled_asset("lua".into(), ScriptAsset {
            content: code.into_bytes().into_boxed_slice(),
            asset_path: code_path.into(),
        }));
        Ok(asset)
        // #[cfg(feature = "pico8-to-lua")]
        // if let Some(patched_code) = pico8::translate_pico8_to_lua(&code, load_context).await? {
        //     code = patched_code;
        // }

        // asset.code =

        // code: config.code.map(|p| load_context.load(&*p)),

    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["lua"];
        EXTENSIONS
    }
}

async fn into_asset(config: Config, load_context: &mut LoadContext<'_>) -> Result<Pico8Asset, ConfigLoaderError> {
        let mut sprite_sheets = vec![];
        for (i, mut sheet) in config.sprite_sheets.into_iter().enumerate() {
            // let flags: Vec<u8>;
            // if sheet.path.extension() == Some(OsStr::new("tsx")) {
            //     #[cfg(feature = "level")]
            //     {
            //         let tiledset = load_context
            //             .loader()
            //             .immediate()
            //             .load::<TiledSet>(&*sheet.path)
            //             .await?;
            //         let tileset = &tiledset.get().0;
            //         let handle = load_context
            //             .add_labeled_asset(format!("atlas{i}"), layout_from_tileset(tileset));
            //         let tile_size = UVec2::new(tileset.tile_width, tileset.tile_height);
            //         if let Some(sprite_size) = sheet.sprite_size {
            //             assert_eq!(sprite_size, tile_size);
            //         }
            //         let flags = flags_from_tileset(tileset);
            //         sprite_sheets.push(pico8::SpriteSheet {
            //             handle: pico8::SprHandle::Image(
            //                 load_context
            //                     .loader()
            //                     .with_settings(pixel_art_settings)
            //                     .load(
            //                         &*tileset
            //                             .image
            //                             .as_ref()
            //                             .ok_or(ConfigLoaderError::Message(format!(
            //                                 "could not load .tsx image {i}"
            //                             )))?
            //                             .source,
            //                     ),
            //             ),
            //             sprite_size: tile_size,
            //             flags,
            //             layout: handle,
            //         })
            //     }
            //     #[cfg(not(feature = "level"))]
            //     panic!(
            //         "Can not load {:?} file without 'level' feature.",
            //         &sheet.path
            //     );
            // } else if sheet.path.extension() == Some(OsStr::new("p8")) {
            //     todo!()
            // } else {
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
                        .load::<Image>(dbg!(&*sheet.path))
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
            // }
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
                                                         config::Font::Default { default: yes } if yes => {
                                                             pico8::N9Font {
                                                                 handle: TextFont::default().font,
                                                             }
                                                         },
                                                         config::Font::Path { path, height: _ } => {
                                                             pico8::N9Font {
                                                                 handle: load_context.load(path),
                                                             }
                                                         }
                                                         config::Font::Default { .. } => { panic!("Must use a path if not default font.") }
                                                     }).collect::<Vec<_>>().into(),

            };
        Ok(state)
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
