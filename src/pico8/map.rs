use crate::pico8::{self, Clearable, Gfx, SprAsset};
use bevy::prelude::*;

#[cfg(feature = "level")]
use crate::level;
use bevy_ecs_tilemap::prelude::*;

#[derive(Clone, Debug, Reflect)]
pub enum Map {
    P8(P8Map),
    #[cfg(feature = "level")]
    Level(level::Tiled),
}

#[derive(Clone, Debug, Deref, DerefMut, Reflect)]
pub struct P8Map {
    #[deref]
    pub entries: Vec<u8>,
    pub sheet_index: usize,
}

impl From<P8Map> for Map {
    fn from(map: P8Map) -> Self {
        Map::P8(map)
    }
}

impl P8Map {
    pub fn map(
        &self,
        map_pos: UVec2,
        screen_start: Vec2,
        size: UVec2,
        mask: Option<u8>,
        sprite_sheets: &[pico8::SpriteSheet],
        commands: &mut Commands,
        mut gfx_to_image: impl FnMut(&Handle<Gfx>) -> Handle<Image>,
    ) -> Result<Entity, pico8::Error> {
        let map_size = TilemapSize::from(size);
        // Create a tilemap entity a little early.
        // We want this entity early because we need to tell each tile which tilemap entity
        // it is associated with. This is done with the TilemapId component on each tile.
        // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
        // will contain various necessary components, such as `TileStorage`.

        // To begin creating the map we will need a `TileStorage` component.
        // This component is a grid of tile entities and is used to help keep track of individual
        // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
        // per layer, each with their own `TileStorage` component.

        // Spawn the elements of the tilemap.
        // Alternatively, you can use helpers::filling::fill_tilemap.
        let clearable = Clearable::default();
        let mut tile_storage = TileStorage::empty(map_size);
        let tilemap_entity = commands.spawn(Name::new("map")).id();
        commands.entity(tilemap_entity).with_children(|builder| {
            for x in 0..map_size.x {
                for y in 0..map_size.y {
                    let texture_index = self
                        .entries
                        .get((map_pos.x + x + (map_pos.y + y) * pico8::MAP_COLUMNS) as usize)
                        .and_then(|index| {
                            if let Some(mask) = mask {
                                sprite_sheets
                                    .get(self.sheet_index)
                                    .and_then(|sprite_sheet| {
                                        (sprite_sheet.flags[*index as usize] & mask == mask)
                                            .then_some(index)
                                    })
                                // (cart.flags[*index as usize] & mask == mask)
                                //     .then_some(index)
                            } else {
                                Some(index)
                            }
                        })
                        .copied()
                        .unwrap_or(0);
                    if texture_index != 0 {
                        let tile_pos = TilePos {
                            x,
                            y: map_size.y - y - 1,
                        };
                        let tile_entity = builder
                            .spawn((
                                TileBundle {
                                    position: tile_pos,
                                    tilemap_id: TilemapId(tilemap_entity),
                                    texture_index: TileTextureIndex(texture_index as u32),
                                    ..Default::default()
                                },
                                // clearable.clone(),
                            ))
                            .id();
                        tile_storage.set(&tile_pos, tile_entity);
                    }
                }
            }
        });

        let sprites = &sprite_sheets[self.sheet_index];
        let tile_size: TilemapTileSize = sprites.sprite_size.as_vec2().into();
        let grid_size = tile_size.into();
        let map_type = TilemapType::default();
        let mut transform =
            get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z());
        transform.translation += screen_start.extend(0.0);

        commands.entity(tilemap_entity).insert((
            TilemapBundle {
                grid_size,
                map_type,
                size: map_size,
                storage: tile_storage,
                texture: TilemapTexture::Single(match &sprites.handle {
                    SprAsset::Image(handle) => handle.clone(),
                    SprAsset::Gfx(ref handle) => gfx_to_image(handle),
                    // self.gfx_handles.get_or_create(&self.state.pal, handle, &self.gfxs, &mut self.images)
                }),
                tile_size,
                // transform: Transform::from_xyz(screen_start.x, -screen_start.y, 0.0),//get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
                transform,
                ..Default::default()
            },
            clearable,
        ));
        Ok(tilemap_entity)
    }
}

/// Calculates a [`Transform`] for a tilemap that places it so that its center is at
/// `(0.0, 0.0, 0.0)` in world space.
pub(crate) fn get_tilemap_top_left_transform(
    size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    z: f32,
) -> Transform {
    assert_eq!(map_type, &TilemapType::Square);
    let y = size.y as f32 * grid_size.y;
    Transform::from_xyz(grid_size.x / 2.0, -y + grid_size.y / 2.0, z)
}

#[cfg(feature = "level")]
impl From<level::Tiled> for Map {
    fn from(map: level::Tiled) -> Self {
        Map::Level(map)
    }
}
