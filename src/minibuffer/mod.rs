use crate::error::RunState;
#[cfg(feature = "scripting")]
use crate::{call, pico8::lua::with_system_param};
use bevy::{core::FrameCount, prelude::*};
use bevy_minibuffer::prelude::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
    bindings::{
        function::{namespace::NamespaceBuilder, script_function::FunctionCallContext},
        script_value::ScriptValue,
    },
    error::InteropError,
    event::ScriptCallbackEvent,
};
// mod count;
// pub use count::*;

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
                #[cfg(feature = "scripting")]
                Act::new(lua_eval).bind(keyseq! { Space N E }),
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
    fn build(&self, app: &mut App) {
        self.warn_on_unused_acts();
        let world = app.world_mut();
        #[cfg(feature = "scripting")]
        NamespaceBuilder::<World>::new_unregistered(world).register(
            "message",
            |ctx: FunctionCallContext, s: String| {
                with_minibuffer(&ctx, |minibuffer| {
                    minibuffer.message(s);
                    Ok(())
                })
            },
        );
    }
}

#[cfg(feature = "scripting")]
fn with_minibuffer<T>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Minibuffer) -> Result<T, Error>,
) -> Result<T, InteropError> {
    with_system_param::<Minibuffer, T, Error>(ctx, f)
}

pub fn toggle_pause(
    state: Res<State<RunState>>,
    mut next_state: ResMut<NextState<RunState>>,
    frame_count: Res<FrameCount>,
) {
    next_state.set(match **state {
        RunState::Run => RunState::Pause,
        RunState::Pause => RunState::Run,
        _ => RunState::Pause,
    });
}

#[cfg(feature = "scripting")]
pub fn lua_eval(mut minibuffer: Minibuffer) {
    minibuffer.prompt::<TextField>("Lua Eval: ").observe(
        |mut trigger: Trigger<Submit<String>>,
         mut writer: EventWriter<ScriptCallbackEvent>,
         mut commands: Commands| {
            if let Ok(input) = trigger.event_mut().take_result() {
                writer.send(ScriptCallbackEvent::new_for_all(
                    call::Eval,
                    vec![ScriptValue::String(input.into()), ScriptValue::Bool(true)],
                ));
            } else {
                commands.entity(trigger.entity()).despawn_recursive();
            }
        },
    );
}
