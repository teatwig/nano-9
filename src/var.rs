use crate::{api::*, *};
use bevy_mod_scripting::lua::prelude::tealr::mlu::mlua::{Variadic};
use bevy_mod_scripting::{core::event::ScriptLoaded, prelude::*};
use std::{borrow::Cow, sync::Arc};

#[derive(Component, Reflect)]
pub struct N9Var(Cow<'static, str>);

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<N9Var>()
        .add_systems(PostStartup, set_vars)
        .add_systems(PreUpdate, set_vars.run_if(on_event::<ScriptLoaded>));
}

impl N9Var {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        N9Var(name.into())
    }
}

/// Sends initialization event
fn set_vars(mut events: PriorityEventWriter<LuaEvent<N9Args>>, query: Query<(Entity, &N9Var)>) {
    for (id, var) in &query {
        events.send(
            LuaEvent {
                hook_name: "_set_global".to_owned(),
                args: {
                    let mut args = Variadic::new();
                    args.push(N9Arg::String(var.0.to_string()));
                    // args.push(N9Arg::Entity(id));
                    args.push(N9Arg::N9Entity(Arc::new(N9Entity {
                        entity: id,
                        drop: DropPolicy::Nothing,
                    })));
                    args
                },
                recipients: Recipients::All,
            },
            0,
        );
    }
}
