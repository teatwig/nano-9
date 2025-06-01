

use super::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        error::InteropError,
    };

use crate::pico8::{
        Gfx,
    };

use std::any::TypeId;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub(crate) fn palette(&self, index: Option<usize>) -> Result<&Palette, Error> {
        self
            .pico8_asset()?
            .palettes
            .get(index.unwrap_or(self.state.palette))
            .ok_or(Error::NoSuch("palette".into()))
    }

    pub fn pal_map(&mut self, original_to_new: Option<(usize, usize)>, mode: Option<PalModify>) {
        let mode = mode.unwrap_or_default();
        assert!(matches!(mode, PalModify::Following));
        if let Some((old, new)) = original_to_new {
            self.state.pal_map.remap(old, new);
        } else {
            // Reset the pal_map.
            self.state.pal_map.reset();
        }
    }

    /// Return the number of colors in the current palette.
    pub fn paln(&self, palette_index: Option<usize>) -> Result<usize, Error> {
        self.palette(palette_index).map(|pal| pal.data.len())
    }

    pub fn palt(&mut self, color_index: Option<usize>, transparent: Option<bool>) {
        if let Some(color_index) = color_index {
            self.state
                .pal_map
                .transparency
                .set(color_index, transparent.unwrap_or(false));
        } else {
            // Reset the pal_map.
            self.state.pal_map.reset_transparency();
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{N9Entity, DropPolicy, pico8::lua::with_pico8};

use bevy_mod_scripting::core::{
    bindings::{
        access_map::ReflectAccessId,
        function::{
            from::FromScript,
            into_ref::IntoScriptRef,
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        script_value::ScriptValue,
        IntoScript, ReflectReference,
    },
    error::InteropError,
};
pub(crate) fn plugin(app: &mut App) {
    // callbacks can receive any `ToLuaMulti` arguments, here '()' and
    // return any `FromLuaMulti` arguments, here a `usize`
    // check the Rlua documentation for more details
    let world = app.world_mut();

    NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
        .register(
            "pal",
            |ctx: FunctionCallContext, old: Option<usize>, new: Option<usize>, mode: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    if old.is_some() && new.is_none() && mode.is_none() {
                        // Set the palette.
                        pico8.state.palette = old.unwrap();
                    } else {
                        pico8.pal_map(
                            old.zip(new),
                            mode.map(|i| match i {
                                0 => PalModify::Following,
                                1 => PalModify::Present,
                                2 => PalModify::Secondary,
                                x => panic!("No such palette modify mode {x}"),
                            }),
                        );
                    }
                    Ok(())
                })
            },
        )
        .register(
            "palt",
            |ctx: FunctionCallContext, color: Option<usize>, transparency: Option<bool>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.palt(color, transparency);
                    Ok(())
                })
            },
        )

        ;
}

}
