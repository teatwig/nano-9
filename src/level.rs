use bevy::prelude::*;

use bevy_ecs_ldtk::prelude::*;

#[derive(Debug, Clone)]
pub struct Map {
    pub handle: Handle<LdtkProject>,
}

pub(crate) fn plugin(app: &mut App) {
    app.add_plugins(LdtkPlugin)
        // .register_ldtk_entity::<Slime>("Slime")
        .insert_resource(LevelSelection::index(0))
        // .add_systems(PostUpdate, process_entities)
        ;

}

