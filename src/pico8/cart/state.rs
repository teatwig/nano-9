use crate::{
    pico8::{audio::*, image::pixel_art_settings, *},
    DrawState,
};
use bevy::asset::{
    io::{AssetSourceId, Reader},
    AssetLoader, AssetPath, LoadContext,
};

use super::*;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::asset::ScriptAsset;
use bitvec::prelude::*;
use pico8_decompress::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) fn plugin(app: &mut App) {
    app.init_asset_loader::<P8StateLoader>()
        .init_asset_loader::<PngStateLoader>();
}

#[derive(Default)]
struct P8StateLoader;

impl AssetLoader for P8StateLoader {
    type Asset = Pico8State;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let cart = P8CartLoader::default()
            .load(reader, settings, load_context)
            .await?;
        std::fs::write("cart-loaded.lua", &cart.lua).unwrap();
        info!("WROTE LOADED CODE to cart-loaded.lua");
        to_state(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

#[derive(Default)]
struct PngStateLoader;

impl AssetLoader for PngStateLoader {
    type Asset = Pico8State;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let cart = PngCartLoader::default()
            .load(reader, settings, load_context)
            .await?;
        to_state(cart, load_context)
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

fn to_state(cart: Cart, load_context: &mut LoadContext) -> Result<Pico8State, CartLoaderError> {
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
            handle: SprAsset::Gfx(
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
    let state = Pico8State {
        #[cfg(feature = "scripting")]
        code: if cfg!(feature = "scripting") {
            load_context.labeled_asset_scope("lua".into(), move |_load_context| ScriptAsset {
                content: code.into_bytes().into_boxed_slice(),
                asset_path: code_path.into(),
            })
        } else {
            Handle::default()
        },
        palettes: vec![Palette::from_slice(&PALETTE)].into(),
        pal_map: PalMap::default(),
        border: load_context
            .loader()
            .with_settings(pixel_art_settings)
            .load(pico8::PICO8_BORDER),
        maps: vec![P8Map {
            entries: cart.map.clone(),
            sheet_index: 0,
        }
        .into()]
        .into(),
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
        )]
        .into(),
        sprite_sheets: sprite_sheets.into(),
        draw_state: DrawState::default(),
        font: vec![N9Font {
            handle: load_context.load(PICO8_FONT),
        }]
        .into(),
    };
    Ok(state)
}
