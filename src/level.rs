use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use crate::pico8::Clearable;
use bevy_ecs_tiled::{TiledMapPluginConfig, prelude::*};
use std::path::Path;
// pub mod ldtk;
// use ldtk::*;
pub(crate) mod tiled;

#[derive(Debug, Clone)]
pub struct Map {
    pub handle: Handle<TiledMap>,
    // pub handle: LdtkMapHandle,
}

impl Map {
    pub fn map(&self, screen_start: Vec2, level: usize, mut commands: &mut Commands) -> Entity {
        // commands.insert_resource(LevelSelection::index(level));
        let clearable = Clearable::default();

        // let mut transform =
        //     get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z());
        // transform.translation += screen_start.extend(0.0);
        commands.spawn((TiledMapHandle(self.handle.clone()),
            // ldtk_map: self.handle.clone(),
                        Transform::from_xyz(screen_start.x, screen_start.y, clearable.suggest_z()),
                        TiledMapSettings {
                            layer_positioning: LayerPositioning::Anchor(TilemapAnchor::TopLeft),
                            ..default()
                        },
                        Name::new("level"),
                        clearable,
                        InheritedVisibility::default(),
        )).id()
    }
}

pub(crate) fn plugin(app: &mut App) {
    app//.add_plugins(LdtkPlugin)
        .add_plugins(TilemapPlugin)
        .add_plugins(TiledMapPlugin(TiledMapPluginConfig { tiled_types_export_file: None }))
        // .add_plugins(ldtk::LdtkPlugin)
        // .register_ldtk_entity::<Slime>("Slime")
        // .insert_resource(LevelSelection::index(0))
        // .add_systems(PostUpdate, process_entities)
        ;

}

