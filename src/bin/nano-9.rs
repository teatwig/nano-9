use bevy::prelude::*;
use bevy_minibuffer::prelude::*;
// use bevy_minibuffer_inspector as inspector;
use bevy_mod_scripting::prelude::*;
use nano_9::{*, pico8::*, minibuffer::*, error::*};
use bevy_ecs_tilemap::prelude::{TilePos, TilemapType};
use std::env;

fn main() -> std::io::Result<()> {
    let args = env::args();
    let script_path: String = args
        .skip(1)
        .next()
        .map(|s| format!("../{s}"))
        .unwrap_or("scripts/main.lua".into());
    let nano9_plugin = Nano9Plugin::default();
    let mut app = App::new();
    app
        .add_plugins(nano9_plugin.default_plugins())
        .add_plugins(nano9_plugin)
        .add_plugins(nano_9::pico8::plugin)
        .add_plugins(MinibufferPlugins)
        .add_acts((
            BasicActs::default(),
            acts::universal::UniversalArgActs::default(),
            acts::tape::TapeActs::default(),
            bevy_minibuffer_inspector::WorldActs::default(),
            crate::minibuffer::Nano9Acts::default(),
            CountComponentsActs::default()
                .add::<Text>("text")
                .add::<TilemapType>("map")
                .add::<TilePos>("tile")
                .add::<Sprite>("sprite")
                ,
            // inspector::AssetActs::default().add::<Image>(),
        ))
        .add_systems(Startup,
                     |mut state: ResMut<NextState<ErrorState>>| {
                         state.set(ErrorState::Messages { frame: 1 });
                     })
        // .insert_state(ErrorState::Messages { frame: 0 })
        ;
    if script_path.ends_with(".p8") {
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let cart: Handle<Cart> = asset_server.load(&script_path);
                commands.spawn(LoadCart(cart));
            },
        );
    } else {
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(ScriptCollection::<LuaFile> {
                    scripts: vec![Script::new(
                        script_path.clone(),
                        asset_server.load(&script_path),
                    )],
                });
            },
        );
    }

    app
        .run();
    Ok(())
}
