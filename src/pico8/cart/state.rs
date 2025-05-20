use crate::{
    pico8::{audio::*, image::pixel_art_settings, *},
    DrawState,
};
use bevy::{
    asset::{io::{Reader, AssetSourceId}, AssetLoader, LoadContext, AssetPath, },
};
use bevy_mod_scripting::core::asset::ScriptAsset;
use bitvec::prelude::*;
use pico8_decompress::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use super::*;

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
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let content = String::from_utf8(bytes)?;
        let mut parts = CartParts::from_str(&content, settings)?;
        let code: String = std::mem::take(&mut parts.lua);
        #[cfg(feature = "pico8-to-lua")]
        {
            let mut include_paths = vec![];
            // Patch the includes.
            let mut include_patch = pico8_to_lua::patch_includes(&code, |path| {
                include_paths.push(path.to_string());
                "".into()
            });
            if !include_paths.is_empty() {
                // There are included files, let's read them all then add them.
                let mut path_contents = std::collections::HashMap::new();
                for path in include_paths.into_iter() {
                    let mut cart_path: PathBuf = load_context.path().to_owned();
                    cart_path.pop();
                    cart_path.push(&path);
                    let source: AssetSourceId<'static> = load_context.asset_path().source().clone_owned();
                    if cart_path.extension() == Some(std::ffi::OsStr::new("p8")) {
                        // let contents = load_context.load(&include_path.with_label("lua")).await?;

                        todo!()
                    } else {
                        let include_path = AssetPath::from(cart_path).with_source(source);
                        warn!("include_path {:?}", &include_path);
                        let contents = load_context.read_asset_bytes(&include_path).await?;
                        path_contents.insert(path, String::from_utf8(contents)?);
                    }
                }

                include_patch = pico8_to_lua::patch_includes(&code, |path| path_contents.remove(path).unwrap());
            }

            // Patch the code.
            let result = pico8_to_lua::patch_lua(include_patch);
            if pico8_to_lua::was_patched(&result) {
                parts.lua = result.to_string();
                std::fs::write("cart-patched.lua", &code).unwrap();
                info!("WROTE PATCHED CODE to cart-patched.lua");
            }
        }
        let gfx = parts.gfx.clone();
        // let code_path = load_context.asset_path().clone().with_label("lua");
        let mut code_path: PathBuf = load_context.path().into();
        // let path = code_path.as_mut_os_string();
        // code_path.
        // {
        //     let path = code_path.as_mut_os_string();
        //     path.push("#lua");
        // }
        // warn!("script asset path {}", code_path.display());
        // let cart = Cart {
        //     lua: ,
        //     gfx: gfx.map(|gfx| {
        //         load_context.labeled_asset_scope("gfx".into(), move |_load_context| gfx)
        //     }),
        //     map: parts.map,
        //     flags: parts.flags,
        //     sfx: parts
        //         .sfx
        //         .into_iter()
        //         .enumerate()
        //         .map(|(n, sfx)| {
        //             load_context.labeled_asset_scope(format!("sfx{n}"), move |_load_context| sfx)
        //         })
        //         .collect(),
        // };

        to_state(parts, load_context)

    }

    fn extensions(&self) -> &[&str] {
        &["p8"]
    }
}

fn to_state(cart: CartParts, load_context: &mut LoadContext) -> Result<Pico8State, CartLoaderError> {
        let layout = load_context.labeled_asset_scope("atlas".into(), move |_load_context| TextureAtlasLayout::from_grid(
                    PICO8_SPRITE_SIZE,
                    PICO8_TILE_COUNT.x,
                    PICO8_TILE_COUNT.y,
                    None,
                    None));
        let sprite_sheets: Vec<_> = cart
            .gfx
            .map(|gfx| SpriteSheet {
                handle: SprAsset::Gfx(load_context.labeled_asset_scope("gfx".into(), move |_load_context| gfx)),
                sprite_size: UVec2::splat(8),
                flags: cart.flags.clone(),
                layout,
            })
            .into_iter()
            .collect();
        let code = cart.lua;
        let code_path: PathBuf = load_context.path().into();

        let state = Pico8State {

            code: load_context.labeled_asset_scope("lua".into(), move |_load_context| ScriptAsset {
                content: code.into_bytes().into_boxed_slice(),
                asset_path: code_path.into(),
            }),
            palettes: vec![Palette::from_slice(&PALETTE)].into(),
            pal_map: PalMap::default(),
            border: load_context.loader()
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
                    Audio::Sfx(load_context.labeled_asset_scope(format!("sfx{n}"), move |_load_context| sfx))
                })
                .collect(),
            )]
            .into(),
            sprite_sheets: sprite_sheets.into(),
            draw_state: DrawState::default(),
            font: vec![N9Font {
                handle: load_context.load(PICO8_FONT),
                height: Some(7.0),
            }]
            .into(),
        };
    Ok(state)
}
