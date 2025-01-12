use bevy::{core::FrameCount, prelude::*};
use bevy_minibuffer::prelude::*;
use crate::error::ErrorState;

mod count;
pub use count::*;

#[derive(Debug)]
pub struct Nano9Acts {
    /// Set of acts
    pub acts: Acts,
}

impl Default for Nano9Acts {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                Act::new(toggle_pause).bind(keyseq! { Space N P }),
            ]),
        }
    }
}


impl ActsPlugin for Nano9Acts {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}

impl Plugin for Nano9Acts {
    fn build(&self, _app: &mut App) {
        self.warn_on_unused_acts();
    }
}


pub fn toggle_pause(state: Res<State<ErrorState>>, mut next_state: ResMut<NextState<ErrorState>>,
    frame_count: Res<FrameCount>,
) {
    next_state.set(match **state {
        ErrorState::None => ErrorState::Messages { frame: frame_count.0 },
        ErrorState::Messages { .. } => ErrorState::None
    });
}

