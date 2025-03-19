use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    ecs::system::{SystemParam, SystemState},
    prelude::*};
use bevy_ecs_tiled::{prelude::{TiledMap, TiledMapHandle}, map::components::TiledIdStorage};
use tiled::{Tileset, PropertyValue};
use crate::pico8::{self, PropBy};
use std::{path::Path, io::ErrorKind};

#[derive(SystemParam)]
pub struct Level<'w, 's> {
    tiled_maps: ResMut<'w, Assets<bevy_ecs_tiled::prelude::TiledMap>>,
    tiled_id_storage: Query<'w, 's, (&'static TiledIdStorage, &'static TiledMapHandle)>,
    sprites: Query<'w, 's, &'static mut Sprite>,
}
impl<'w, 's> Level<'w, 's> {

    pub fn mget(&self, map_handle: &Handle<TiledMap>, pos: Vec2, map_index: Option<usize>, layer_index: Option<usize>) -> Option<usize> {
        self.tiled_maps
            .get(map_handle)
            .and_then(|tiled_map|
                 tiled_map.map.get_layer(layer_index.unwrap_or(0)).and_then(|layer| {
                     let tile_size = UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                     match layer.layer_type() {
                         tiled::LayerType::Tiles(tile_layer) => {
                             tile_layer.get_tile(pos.x as i32, pos.y as i32)
                                       .map(|layer_tile| layer_tile.id() as usize)
                         }
                         tiled::LayerType::Objects(object_layer) => {
                            let mut result = None;
                            // dbg!(pos);
                            // dbg!(tile_size);
                            let posf = pos * tile_size.as_vec2();
                            // dbg!(posf);
                            for object in object_layer.objects() {
                                /// The tiles in Tiled are positioned by their bottom left.
                                let obj_rect = match object.shape {
                                    tiled::ObjectShape::Rect { width, height } => {
                                        Rect::new(object.x,
                                                  object.y,
                                                  object.x + width,
                                                  object.y + height)
                                    }
                                    _ => {
                                        Rect::new(object.x,
                                                  object.y - tile_size.y as f32,
                                                  object.x + tile_size.x as f32,
                                                  object.y)
                                    }
                                };
                                dbg!(obj_rect);
                                if obj_rect.contains(posf) {
                                    result = object.properties.get("p8flags")
                                        .and_then(|value| match value {
                                            PropertyValue::IntValue(i) => Some(*i as usize),
                                            _ => None
                                        });
                                    break;
                                }
                            }
                            result
                         }
                         _ => None
                     }
                 })
            )
    }

    pub fn mgetp(&self, map_handle: &Handle<TiledMap>, prop_by: pico8::PropBy, map_index: Option<usize>, layer_index: Option<usize>) -> Option<tiled::Properties> {
        self.tiled_maps
            .get(map_handle)
            .and_then(|tiled_map| {
                let tile_size = UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                tiled_map.map.get_layer(layer_index.unwrap_or(0)).and_then(|layer| {
                    match layer.layer_type() {
                        tiled::LayerType::Tiles(tile_layer) => {

                            match prop_by {
                                PropBy::Pos(pos) => {
                                    tile_layer.get_tile(pos.x as i32, pos.y as i32)
                                      .and_then(|layer_tile| layer_tile.get_tile().map(|tile| tile.properties.clone()))
                                }
                                PropBy::Name(name) => {
                                    warn!("Cannot look up by name {name:?} on a tile layer.");
                                    None
                                }
                            }
                        }
                        tiled::LayerType::Objects(object_layer) => {
                            match prop_by {
                                PropBy::Pos(pos) => {
                                    let posf = pos * tile_size.as_vec2();
                                    for object in object_layer.objects() {
                                        /// The tiles in Tiled are positioned by their bottom left.
                                        let obj_rect = match object.shape {
                                            tiled::ObjectShape::Rect { width, height } => {
                                                Rect::new(object.x,
                                                          object.y,
                                                          object.x + width,
                                                          object.y + height)
                                            }
                                            _ => {
                                                Rect::new(object.x,
                                                          object.y - tile_size.y as f32,
                                                          object.x + tile_size.x as f32,
                                                          object.y)
                                            }
                                        };
                                        if obj_rect.contains(posf) {
                                            let mut properties = object.properties.clone();

                                            insert_object_fields(&mut properties, &object);
                                            return Some(properties);
                                        }
                                    }
                                    None
                                }
                                PropBy::Name(name) => {
                                    for object in object_layer.objects() {
                                        if object.name == name {
                                            let mut properties = object.properties.clone();
                                            insert_object_fields(&mut properties, &object);
                                            return Some(properties);
                                        }
                                    }
                                    None
                                }

                            }
                        }
                        _ => None
                    }
                })
            })
    }

    pub fn mset(&mut self, map_handle: &Handle<TiledMap>, pos: Vec2, sprite_index: usize, map_index: Option<usize>, layer_index: Option<usize>)
                -> Result<(), pico8::Error> {
        self.tiled_maps
            .get(map_handle)
            .ok_or(pico8::Error::NoSuch("map".into()))
            .and_then(|tiled_map| {
                let tile_size = UVec2::new(tiled_map.map.tile_width, tiled_map.map.tile_width);
                tiled_map.map.get_layer(layer_index.unwrap_or(0))
                    .ok_or(pico8::Error::NoSuch("layer".into()))
                    .and_then(|layer| {
                    match layer.layer_type() {
                        tiled::LayerType::Tiles(tile_layer) => {
                            // tile_layer.get_tile(pos.x as i32, pos.y as i32)
                            //           .and_then(|layer_tile| layer_tile.get_tile().map(|tile| tile.properties.clone()))
                            Ok(())
                        }
                        tiled::LayerType::Objects(object_layer) => {
                            // dbg!(pos);
                            // dbg!(tile_size);
                            let posf = pos * tile_size.as_vec2();
                            // dbg!(posf);
                            for object in object_layer.objects() {
                                /// The tiles in Tiled are positioned by their bottom left.
                                let obj_rect = match object.shape {
                                    tiled::ObjectShape::Rect { width, height } => {
                                        Rect::new(object.x,
                                                    object.y,
                                                    object.x + width,
                                                    object.y + height)
                                    }
                                    _ => {
                                        Rect::new(object.x,
                                                    object.y - tile_size.y as f32,
                                                    object.x + tile_size.x as f32,
                                                    object.y)
                                    }
                                };
                                // dbg!(obj_rect);
                                if obj_rect.contains(posf) {
                                    let mut sprite_id = None;
                                    for (tiled_id_storage, handle) in &self.tiled_id_storage {
                                        if handle.0 == *map_handle {
                                            // This is probably the one.
                                            if let Some(id) = tiled_id_storage.objects.get(&object.id()) {
                                                sprite_id = Some(id);
                                            }
                                        }
                                    }
                                    return if let Some(id) = sprite_id {
                                        self.sprites
                                            .get_mut(*id)
                                            .map_err(|_| pico8::Error::NoSuch("object sprite".into()))
                                            .and_then(|mut sprite| {
                                                if let Some(ref mut atlas) = &mut sprite.texture_atlas {
                                                    atlas.index = sprite_index;
                                                    Ok(())
                                                } else {
                                                    Err(pico8::Error::NoSuch("sprite atlas".into()))
                                                }
                                            })
                                    } else {
                                        Err(pico8::Error::NoSuch("sprite entity".into()))
                                    }
                                }
                            }
                            Err(pico8::Error::NoSuch("tile".into()))
                        }
                        _ => Err(pico8::Error::Unsupported("setting tile and object layers in map".into()))
                    }
                })
            })
    }
}

fn insert_object_fields(properties: &mut tiled::Properties, object: &tiled::Object) {
    properties.insert("x".to_owned(), tiled::PropertyValue::FloatValue(object.x));
    properties.insert("y".to_owned(), tiled::PropertyValue::FloatValue(object.y));
    match object.shape {
        tiled::ObjectShape::Rect { width, height } => {
            properties.insert("width".to_owned(), tiled::PropertyValue::FloatValue(width));
            properties.insert("height".to_owned(), tiled::PropertyValue::FloatValue(height));
        }
        _ => {}
    }
    properties.insert("class".to_owned(),
                        tiled::PropertyValue::StringValue(object.user_type.clone()));
}


pub(crate) fn layout_from_tileset(tileset: &Tileset) -> TextureAtlasLayout {
    TextureAtlasLayout::from_grid(
        UVec2::new(tileset.tile_width, tileset.tile_height),
        tileset.columns,
        tileset.tilecount / tileset.columns,
        (tileset.spacing != 0).then_some(UVec2::new(tileset.spacing, tileset.spacing)),
        (tileset.offset_x != 0 || tileset.offset_y != 0).then_some(UVec2::new(tileset.offset_x as u32, tileset.offset_y as u32)))
}

pub(crate) fn layer_tile_properties(tile: &tiled::LayerTile) -> Option<tiled::Properties> {
    tile.get_tile().map(|t| t.properties.clone())
}



pub(crate) fn flags_from_tileset(tileset: &Tileset) -> Vec<u8> {
    let mut flags: Vec<u8> = vec![0; tileset.tilecount as usize];
    for (id, tile) in tileset.tiles() {
        flags[id as usize] = tile.properties.get("p8flags").map(|value| match value {
            tiled::PropertyValue::IntValue(x) => *x as u8,
            v => panic!("Expected integer value not {v:?}"),
        }).unwrap_or(0);
    }
    flags
}

