use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;
use crate::pico8::Clearable;

#[derive(Debug, Clone)]
pub struct Map {
    pub handle: Handle<LdtkProject>,
}

impl Map {
    pub fn map(&self, screen_start: Vec2, level: usize, mut commands: &mut Commands) -> Entity {
        commands.insert_resource(LevelSelection::index(level));
        let clearable = Clearable::default();

        let mut transform =
            get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z());
        transform.translation += screen_start.extend(0.0);
        commands.spawn((LdtkWorldBundle {
            ldtk_handle: self.handle.clone().into(),
            transform: Transform::from_xyz(screen_start.x, screen_start.y, clearable.suggest_z()),
            ..default()
        },
                        Name::new("level"),
                        clearable,
        )).id()
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(LdtkPlugin)
        // .register_ldtk_entity::<Slime>("Slime")
        .insert_resource(LevelSelection::index(0))
        // .add_systems(PostUpdate, process_entities)
        ;

}

