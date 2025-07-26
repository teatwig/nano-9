use crate::pico8::Pico8State;
use bevy::utils::HashMap;
use bevy::{ecs::component::ComponentId, ecs::world::DeferredWorld, prelude::*};
use std::{
    fmt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

static DRAW_COUNTER: DrawCounter = DrawCounter::new(1);
///
const MAX_EXPECTED_CLEARABLES: f32 = 1000.0;

pub(crate) fn plugin(app: &mut App) {
    app.add_event::<ClearEvent>()
        .init_resource::<ClearCache>()
        .add_systems(Last, (handle_overflow, handle_clear_event).chain());
}

// Define a newtype around AtomicUsize
struct DrawCounter {
    counter: AtomicUsize,
    overflowed: AtomicBool,
}

impl DrawCounter {
    // Create a new DrawCounter with an initial value
    pub const fn new(initial: usize) -> Self {
        Self {
            counter: AtomicUsize::new(initial),
            overflowed: AtomicBool::new(false),
        }
    }

    // Increment the counter and return the previous value
    pub fn increment(&self) -> usize {
        let r = self.counter.fetch_add(1, Ordering::Relaxed);
        if r == 0 {
            warn!("draw counter over flowed.");
            self.overflowed.store(true, Ordering::Relaxed);
        }
        r
    }

    fn overflowed(&self) -> bool {
        self.overflowed.load(Ordering::Relaxed)
    }

    fn reset_overflowed(&self) {
        self.overflowed.store(false, Ordering::Relaxed)
    }

    // Get the current value of the counter
    pub fn get(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    pub fn set(&self, value: usize) {
        self.counter.store(value, Ordering::Relaxed);
    }
}

impl Default for DrawCounter {
    fn default() -> Self {
        DrawCounter::new(1)
    }
}

impl fmt::Debug for DrawCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DrawCounter({})", self.get())
    }
}

#[derive(Debug, Event, Clone, Copy)]
pub struct ClearEvent {
    draw_ceiling: usize,
}

impl Default for ClearEvent {
    fn default() -> Self {
        ClearEvent {
            draw_ceiling: DRAW_COUNTER.get(),
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub(crate) struct ClearCache(HashMap<u64, Entity>);

#[derive(Debug, Component, Clone, Copy)]
#[component(on_add = on_insert_hook)]
#[component(on_insert = on_insert_hook)]
#[component(on_remove = on_remove_hook)]
pub struct Clearable {
    draw_count: usize,
    pub time_to_live: u8,
    pub hash: Option<u64>,
}

fn on_insert_hook(mut world: DeferredWorld, id: Entity, _comp_id: ComponentId) {
    let Some(hash) = world
        .get::<Clearable>(id)
        .and_then(|clearable| clearable.hash)
    else {
        return;
    };
    let Some(mut cache) = world.get_resource_mut::<ClearCache>() else {
        return;
    };
    cache.insert(hash, id);
}

fn on_remove_hook(mut world: DeferredWorld, id: Entity, _comp_id: ComponentId) {
    let Some(hash) = world
        .get::<Clearable>(id)
        .and_then(|clearable| clearable.hash)
    else {
        return;
    };
    let Some(mut cache) = world.get_resource_mut::<ClearCache>() else {
        return;
    };
    cache.remove(&hash);
}

impl Default for Clearable {
    fn default() -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live: 0,
            hash: None,
        }
    }
}

impl Clearable {
    pub fn new(time_to_live: u8) -> Self {
        Clearable {
            draw_count: DRAW_COUNTER.increment(),
            time_to_live,
            hash: None,
        }
    }

    pub fn with_hash(mut self, hash: u64) -> Self {
        // That's _some_ hash!
        self.hash = Some(hash);
        self
    }

    /// Suggest a z value based on the draw count.
    pub fn suggest_z(&self) -> f32 {
        1.0 + self.draw_count as f32 / MAX_EXPECTED_CLEARABLES
    }

    /// Update the draw count, changes the suggest_z() to be current.
    pub fn update(&mut self) {
        self.draw_count = DRAW_COUNTER.increment();
    }
}

fn handle_overflow(mut query: Query<&mut Clearable>) {
    if DRAW_COUNTER.overflowed() {
        for mut clearable in &mut query {
            // It will normally never be zero.
            clearable.draw_count = 0;
        }
        DRAW_COUNTER.reset_overflowed()
    }
}

fn handle_clear_event(
    mut events: EventReader<ClearEvent>,
    mut query: Query<(Entity, &mut Clearable, &mut Transform, &mut Visibility)>,
    mut commands: Commands,
    mut state: ResMut<Pico8State>,
) {
    if let Some(ceiling) = events.read().map(|e| e.draw_ceiling).max() {
        let (less_than, mut greater_than): (Vec<_>, Vec<_>) = query
            .iter_mut()
            .partition(|(_, clearable, _, _)| clearable.draw_count < ceiling);
        for (id, mut clearable, _, mut visibility) in less_than {
            if clearable.time_to_live <= 0 {
                commands.entity(id).despawn_recursive();
            } else {
                clearable.time_to_live -= 1;
                *visibility = Visibility::Hidden;
            }
        }

        let mut i = 1;
        greater_than.sort_by(|(_, _, a, _), (_, _, b, _)| {
            a.translation
                .z
                .partial_cmp(&b.translation.z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (_id, mut clearable, mut transform, _) in greater_than {
            clearable.draw_count = 0;
            transform.translation.z = i as f32 / MAX_EXPECTED_CLEARABLES;
            i += 1;
        }

        if i == 1 {
            // If there aren't any more clearables, we can let the camera
            // move.
            state.draw_state.camera_position_delta = None;
        }
        DRAW_COUNTER.set(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test0() {
        static COUNTER: DrawCounter = DrawCounter::new(0);
        assert_eq!(COUNTER.increment(), 0);
        assert_eq!(COUNTER.increment(), 1);
        assert_eq!(COUNTER.get(), 2);
    }
}
