use bevy::{
    ecs::system::SystemParam,
    math::bounding::{Aabb2d, AabbCast2d, IntersectsVolume, RayCast2d},
    prelude::*,
};

use crate::pico8::negate_y;

pub struct RaycastPlugin;

impl Plugin for RaycastPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Place>().register_type::<Cover>();
    }
}
/// A `ray`-able object.
#[derive(Debug, Component, Reflect)]
pub struct Cover {
    pub aabb: Aabb2d,
    pub flags: u32,
}

#[derive(Debug, Component, Reflect)]
pub struct Place(pub String);

#[derive(SystemParam)]
pub struct Rays<'w, 's> {
    covers: Query<'w, 's, (Entity, &'static Cover, &'static GlobalTransform)>,
    places: Query<'w, 's, (&'static Place, &'static GlobalTransform)>,
}

impl Rays<'_, '_> {
    pub fn raydown(&self, mut pos: Vec2, mask: Option<u32>, shape: Option<Aabb2d>) -> Vec<Entity> {
        pos.y = negate_y(pos.y);
        self.covers
            .iter()
            .filter_map(|(id, cover, transform)| {
                if let Some(mask) = mask {
                    if cover.flags & mask == 0 {
                        return None;
                    }
                }
                let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                // let min = cover.aabb.min;
                if let Some(mut shape) = shape {
                    shape.min += pos;
                    shape.max += pos;
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    // dbg!(id);
                    shape.intersects(&other).then_some(id)
                } else {
                    (min.x <= pos.x && min.y <= pos.y && {
                        let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                        // let max = cover.aabb.max;
                        max.x > pos.x && max.y > pos.y
                    })
                    .then_some(id)
                }
            })
            .collect()
    }

    // Cast a "ray" either at pos or from pos in direction dir.
    pub fn raycast(
        &self,
        mut pos: Vec2,
        mut dir: Dir2,
        mask: Option<u32>,
        shape: Option<Aabb2d>,
    ) -> Vec<(Entity, f32)> {
        let mut v = dir.as_vec2();
        if cfg!(feature = "negate-y") {
            v.y = -v.y;
            pos.y = -pos.y;
        }
        // dir = Dir2::new_unchecked(v);
        dir = Dir2::new(v).unwrap();
        if let Some(shape) = shape {
            let aabb_cast = AabbCast2d::new(shape, pos, dir, f32::MAX);
            self.covers
                .iter()
                .filter_map(|(id, cover, transform)| {
                    if let Some(mask) = mask {
                        if cover.flags & mask == 0 {
                            return None;
                        }
                    }
                    let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    aabb_cast
                        .aabb_collision_at(other)
                        .map(|distance| (id, distance))
                })
                .collect()
        } else {
            let ray_cast = RayCast2d::new(pos, dir, f32::MAX);
            self.covers
                .iter()
                .filter_map(|(id, cover, transform)| {
                    if let Some(mask) = mask {
                        if cover.flags & mask == 0 {
                            return None;
                        }
                    }
                    let min = (*transform * cover.aabb.min.extend(0.0)).xy();
                    let max = (*transform * cover.aabb.max.extend(0.0)).xy();
                    let other = Aabb2d { min, max };
                    ray_cast.aabb_intersection_at(&other).map(|distance| {
                        dbg!(&other);
                        dbg!(&ray_cast);
                        dbg!((id, distance))
                    })
                })
                .collect()
        }
    }

    pub fn place(&self, name: &str) -> Option<Vec2> {
        for (place, transform) in &self.places {
            if place.0 == name {
                let mut r = transform.translation().xy();
                if cfg!(feature = "negate-y") {
                    r.y = -r.y;
                }
                return Some(r);
            }
        }
        None
    }
}
