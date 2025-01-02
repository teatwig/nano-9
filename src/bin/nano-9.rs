use bevy::prelude::*;
use bevy_mod_scripting::prelude::*;
use nano_9::*;
use std::env;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_minibuffer::prelude::*;

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
                   // acts::universal::UniversalActs::default(),
                   // acts::tape::TapeActs::default()
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
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .run();
    Ok(())
}
