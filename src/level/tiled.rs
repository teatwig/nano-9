use bevy::{asset::{AssetLoader, LoadContext, io::Reader}, prelude::*};
use tiled::Tileset;
use crate::pico8;
use std::{path::Path, io::ErrorKind};

pub(crate) fn layout_from_tileset(tileset: &Tileset) -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(
        UVec2::new(tileset.tile_width, tileset.tile_height),
        tileset.columns,
        tileset.tilecount / tileset.columns,
        (tileset.spacing != 0).then_some(UVec2::new(tileset.spacing, tileset.spacing)),
        (tileset.offset_x != 0 || tileset.offset_y != 0).then_some(UVec2::new(tileset.offset_x as u32, tileset.offset_y as u32)))
}


pub(crate) fn flags_from_tileset(tileset: &Tileset) -> Vec<u8> {
    let flags: Vec<u8> = tileset.tiles().map(|(id, tile)| {
        tile.properties.get("p8flags").map(|value| match value {
            tiled::PropertyValue::IntValue(x) => *x as u8,
            v => panic!("Expected integer value not {v:?}"),
        }).unwrap_or(0)
    }).collect();
    flags
}

