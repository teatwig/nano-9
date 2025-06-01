use bevy::prelude::*;

#[derive(Event, Debug)]
pub(crate) struct UpdateCameraPos(pub(crate) Vec2);
