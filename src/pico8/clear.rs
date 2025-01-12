use bevy::prelude::*;
use std::{fmt, sync::atomic::{AtomicUsize, AtomicBool, Ordering}};

static DRAW_COUNTER: DrawCounter = DrawCounter::new(1);
///
const MAX_EXPECTED_CLEARABLES: f32 = 1000.0;

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
            r + 1
        } else {
            r
        }
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
pub struct ClearEvent { draw_ceiling: usize }

impl Default for ClearEvent {
    fn default() -> Self {
        ClearEvent { draw_ceiling: DRAW_COUNTER.get() }
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub struct Clearable { draw_count: usize }

impl Default for Clearable {
    fn default() -> Self {
        Clearable { draw_count: DRAW_COUNTER.increment() }
    }
}

impl Clearable {
    /// Suggest a z value based on the draw count.
    pub fn suggest_z(&self) -> f32 {
        1.0 + self.draw_count as f32 / MAX_EXPECTED_CLEARABLES
    }
}

pub(crate) fn plugin(app: &mut App) {
    app
        .add_event::<ClearEvent>()
        .add_systems(Last, (handle_overflow, handle_clear_event).chain());
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

fn handle_clear_event(mut events: EventReader<ClearEvent>,
                      mut query: Query<(Entity, &mut Clearable, &mut Transform)>,
                      mut commands: Commands) {
    if let Some(ceiling) = events.read().map(|e| e.draw_ceiling).max() {
        let (less_than, mut greater_than): (Vec<_>, Vec<_>) = query.iter_mut().partition(|(_, clearable, _)| clearable.draw_count < ceiling);
        for (id, _, _) in less_than {
            commands.entity(id).despawn_recursive();
        }

        let mut i = 1;
        greater_than.sort_by(|(_,_,a), (_, _, b)| a.translation.z.partial_cmp(&b.translation.z).unwrap_or(std::cmp::Ordering::Equal));
        for (id, mut clearable, mut transform) in greater_than {
            clearable.draw_count = 0;
            transform.translation.z = i as f32 / MAX_EXPECTED_CLEARABLES;
            i += 1;
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
