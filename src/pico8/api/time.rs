use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn time(&self) -> f32 {
        self.time.elapsed_secs()
    }

    pub fn exit(&mut self, error: Option<u8>) {
        self.commands.send_event(match error {
            Some(n) => std::num::NonZero::new(n)
                .map(AppExit::Error)
                .unwrap_or(AppExit::Success),
            None => AppExit::Success,
        });
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::core::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register("exit", |ctx: FunctionCallContext, error: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.exit(error);
                    Ok(())
                })
            })
            .register("time", |ctx: FunctionCallContext| {
                with_pico8(&ctx, move |pico8| Ok(pico8.time()))
            });
    }
}
