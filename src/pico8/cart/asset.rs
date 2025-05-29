use crate::pico8::{image::pixel_art_settings, *};
use bevy::asset::{io::Reader, AssetLoader, LoadContext};

use super::*;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::asset::ScriptAsset;
use std::path::PathBuf;

pub(crate) fn plugin(app: &mut App) {
    app.init_asset_loader::<P8AssetLoader>()
        .init_asset_loader::<PngAssetLoader>();
}

#[derive(Default)]
struct P8AssetLoader;

impl AssetLoader for P8AssetLoader {
    type Asset = Pico8Asset;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let cart = P8CartLoader.load(reader, settings, load_context).await?;
        log_lua_code(&cart.lua);
        to_asset(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

#[derive(Default)]
struct PngAssetLoader;

impl AssetLoader for PngAssetLoader {
    type Asset = Pico8Asset;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let cart = PngCartLoader.load(reader, settings, load_context).await?;
        to_asset(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

fn to_asset(cart: Cart, load_context: &mut LoadContext) -> Result<Pico8Asset, CartLoaderError> {
    let layout = load_context.labeled_asset_scope("atlas".into(), move |_load_context| {
        TextureAtlasLayout::from_grid(
            PICO8_SPRITE_SIZE,
            PICO8_TILE_COUNT.x,
            PICO8_TILE_COUNT.y,
            None,
            None,
        )
    });
    let sprite_sheets: Vec<_> = cart
        .gfx
        .map(|gfx| SpriteSheet {
            handle: SprHandle::Gfx(
                load_context.labeled_asset_scope("gfx".into(), move |_load_context| gfx),
            ),
            sprite_size: UVec2::splat(8),
            flags: cart.flags.clone(),
            layout,
        })
        .into_iter()
        .collect();
    let code = cart.lua;
    let code_path: PathBuf = load_context.path().into();
    let asset = Pico8Asset {
        #[cfg(feature = "scripting")]
        code: if cfg!(feature = "scripting") {
            Some(
                load_context.labeled_asset_scope("lua".into(), move |_load_context| ScriptAsset {
                    content: code.into_bytes().into_boxed_slice(),
                    asset_path: code_path.into(),
                }),
            )
        } else {
            None
        },
        palettes: vec![Palette::from_slice(&PALETTE)],
        border: load_context
            .loader()
            .with_settings(pixel_art_settings)
            .load(pico8::PICO8_BORDER),
        maps: vec![P8Map {
            entries: cart.map.clone(),
            sheet_index: 0,
        }
        .into()],
        audio_banks: vec![AudioBank(
            cart.sfx
                .into_iter()
                .enumerate()
                .map(|(n, sfx)| {
                    Audio::Sfx(
                        load_context
                            .labeled_asset_scope(format!("sfx{n}"), move |_load_context| sfx),
                    )
                })
                .collect(),
        )],
        sprite_sheets,
        font: vec![N9Font {
            handle: load_context.load(PICO8_FONT),
        }],
    };
    Ok(asset)
}
