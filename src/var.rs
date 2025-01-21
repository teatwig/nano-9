use bevy::prelude::*;
use bevy_mod_scripting::{
        lua::mlua::{prelude::LuaValue, AnyUserData},
        core::{bindings::script_value::ScriptValue, asset::ScriptAsset,
                                event::{ScriptCallbackEvent, OnScriptLoaded}}};
use std::{borrow::Cow, sync::{Mutex, Arc}};
use crate::{on_asset_change, call, N9Entity, DropPolicy};

#[derive(Component, Reflect)]
pub struct N9Var(Cow<'static, str>);

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<N9Var>()
        // .add_systems(PostStartup, set_vars)
        .add_systems(PreUpdate, set_vars.run_if(on_asset_change::<ScriptAsset>()));
        // .add_systems(PreUpdate, set_vars.run_if(on_event::<OnScriptLoaded>));
}

impl N9Var {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        N9Var(name.into())
    }
}

/// Sends initialization event
fn set_vars(mut writer: EventWriter<ScriptCallbackEvent>,
            query: Query<(Entity, &N9Var)>) {
        for (id, var) in &query {
            warn!("Need to impl set_vars().");
            // todo!();
        // writer.send(ScriptCallbackEvent::new_for_all(
        //     call::SetGlobal,
        //     vec![ScriptValue::String(var.0.clone()),
        //          // ScriptValue::Reference(Arc::new(Mutex::new(N9Entity { entity: id,
        //          //                                          drop: DropPolicy::Nothing }))).into()
        //          ScriptValue::Reference(LuaValue::UserData(AnyUserData::wrap(N9Entity { entity: id,
        //                                                   drop: DropPolicy::Nothing })))
        //     ]));
        // events.send(
        //     LuaEvent {
        //         hook_name: "_set_global".to_owned(),
        //         args: {
        //             let mut args = Variadic::new();
        //             args.push(N9Arg::String(var.0.to_string()));
        //             // args.push(N9Arg::Entity(id));
        //             args.push(N9Arg::N9Entity(Arc::new(N9Entity {
        //                 entity: id,
        //                 drop: DropPolicy::Nothing,
        //             })));
        //             args
        //         },
        //         recipients: Recipients::All,
        //     },
        //     0,
        // );
    }
}
