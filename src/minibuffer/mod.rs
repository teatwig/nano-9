use crate::{call, error::ErrorState};
use bevy::{
    core::FrameCount,
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_minibuffer::prelude::*;
use bevy_mod_scripting::core::event::ScriptCallbackEvent;

use bevy_mod_scripting::{
    core::{
        asset::{AssetPathToLanguageMapper, Language, ScriptAsset, ScriptAssetSettings},
        bindings::{
            access_map::ReflectAccessId,
            function::{
                from::FromScript,
                into_ref::IntoScriptRef,
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
            ReflectReference, WorldAccessGuard,
        },
        error::InteropError,
    },
    lua::mlua::prelude::LuaError,
};
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
        NamespaceBuilder::<World>::new_unregistered(world).register(
            "message",
            |ctx: FunctionCallContext, s: String| {
                with_minibuffer(&ctx, |minibuffer| Ok(minibuffer.message(s)))
            },
        );
    }
}

fn with_minibuffer<X>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Minibuffer) -> Result<X, Error>,
) -> Result<X, InteropError> {
    let world_guard = ctx.world()?;
    let raid = ReflectAccessId::for_global();
    if world_guard.claim_global_access() {
        let world = world_guard.as_unsafe_world_cell()?;
        let world = unsafe { world.world_mut() };
        let mut system_state: SystemState<Minibuffer> = SystemState::new(world);
        let mut minibuffer = system_state.get_mut(world);
        let r = f(&mut minibuffer);
        system_state.apply(world);
        unsafe { world_guard.release_global_access() };
        r.map_err(|e| InteropError::external_error(Box::new(e)))
    } else {
        Err(InteropError::cannot_claim_access(
            raid,
            world_guard.get_access_location(raid),
            "with_minibuffer",
        ))
    }
}

pub fn toggle_pause(
    state: Res<State<ErrorState>>,
    mut next_state: ResMut<NextState<ErrorState>>,
    frame_count: Res<FrameCount>,
) {
    next_state.set(match **state {
        ErrorState::None => ErrorState::Messages {
            frame: frame_count.0,
        },
        ErrorState::Messages { .. } => ErrorState::None,
    });
}

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
