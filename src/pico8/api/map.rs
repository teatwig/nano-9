use super::*;
use bevy::utils::hashbrown::hash_map::DefaultHashBuilder;
use std::hash::{BuildHasher, Hash, Hasher};

impl super::Pico8<'_, '_> {
    fn sprite_map(&self, map_index: Option<usize>) -> Result<&Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset()?
            .maps
            .get(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    fn sprite_map_mut(&mut self, map_index: Option<usize>) -> Result<&mut Map, Error> {
        let index = map_index.unwrap_or(0);
        self.pico8_asset_mut()?
            .maps
            .get_mut(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    pub fn map(
        &mut self,
        map_pos: UVec2,
        mut screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        map_index: Option<usize>,
    ) -> Result<Entity, Error> {
        screen_start = self.state.draw_state.apply_camera_delta(screen_start);
        if cfg!(feature = "negate-y") {
            screen_start.y = -screen_start.y;
        }
        let hash = {
            let mut hasher = DefaultHashBuilder::default().build_hasher();
            map_pos.hash(&mut hasher);
            size.hash(&mut hasher);
            mask.inspect(|m| m.hash(&mut hasher));
            map_index.inspect(|i| i.hash(&mut hasher));
            hasher.finish()
        };
        // See if there's already an entity here.
        if let Some(id) = self.clear_cache.get(&hash) {
            let id = *id;
            self.commands.queue(move |world: &mut World| {
                if let Some(mut clearable) = world.get_mut::<Clearable>(id) {
                    clearable.time_to_live = 2;
                }
                if let Some(mut visibility) = world.get_mut::<Visibility>(id) {
                    *visibility = Visibility::Inherited;
                }
                if let Some(_transform) = world.get_mut::<Transform>(id) {
                    // TODO: Need to update the transform.
                    // transform.
                }
            });
            return Ok(id);
        }

        match self.sprite_map(map_index)?.clone() {
            Map::P8(map) => {
                let palette = self.palette(None)?.clone();

                let sprite_sheets = &self.pico8_asset()?.sprite_sheets.clone();
                map.map(
                    map_pos,
                    screen_start,
                    size,
                    mask,
                    sprite_sheets,
                    Some(hash),
                    &mut self.commands,
                    |handle| {
                        self.gfx_handles.get_or_create(
                            &palette,
                            &self.state.pal_map,
                            None,
                            handle,
                            &self.gfxs,
                            &mut self.images,
                        )
                    },
                )
            }
            #[cfg(feature = "level")]
            Map::Level(map) => Ok(map.map(screen_start, 0, &mut self.commands)),
        }
    }

    pub fn mget(
        &self,
        pos: Vec2,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Option<usize> {
        let map: &Map = self.sprite_map(map_index).ok()?;
        match *map {
            Map::P8(ref map) => {
                Some(map[(pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize] as usize)
            }

            #[cfg(feature = "level")]
            Map::Level(ref map) => self.tiled.mget(map, pos, map_index, layer_index),
        }
    }

    pub fn mset(
        &mut self,
        pos: Vec2,
        sprite_index: usize,
        map_index: Option<usize>,
        _layer_index: Option<usize>,
    ) -> Result<(), Error> {
        let map = self.sprite_map_mut(map_index)?;
        match map {
            Map::P8(ref mut map) => map
                .get_mut((pos.x as u32 + pos.y as u32 * MAP_COLUMNS) as usize)
                .map(|value| *value = sprite_index as u8)
                .ok_or(Error::NoSuch("map entry".into())),
            #[cfg(feature = "level")]
            Map::Level(ref mut map) => {
                todo!()
                // self.tiled
                //     .mset(map, pos, sprite_index, map_index, layer_index)
            }
        }
    }
}
