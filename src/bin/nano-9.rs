use bevy::prelude::*;
use bevy_mod_scripting::prelude::*;
use nano_9::*;
use std::env;
use bevy_minibuffer::prelude::*;
use bevy_minibuffer_inspector as inspector;

fn main() -> std::io::Result<()> {
    let args = env::args();
    let script_path: String = args
        .skip(1)
        .next()
        .map(|s| format!("../{s}"))
        .unwrap_or("scripts/main.lua".into());
    let nano9_plugin = Nano9Plugin::default();
    App::new()
        .add_plugins(nano9_plugin.default_plugins())
        .add_plugins(nano9_plugin)
        .add_plugins(MinibufferPlugins)
        .add_acts((BasicActs::default(),
                   acts::universal::UniversalArgActs::default(),
                   acts::tape::TapeActs::default(),
                   inspector::WorldActs::default(),
                   // inspector::AssetActs::default().add::<Image>(),
        ))
        .add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.spawn(ScriptCollection::<LuaFile> {
                    scripts: vec![Script::new(
                        script_path.clone(),
                        asset_server.load(&script_path),
                    )],
                });
            },
        )
        .run();
    Ok(())
}
