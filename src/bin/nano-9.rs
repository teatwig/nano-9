use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    text::FontSmoothing,
   prelude::*,
};
use bevy_ecs_tilemap::prelude::{TilePos, TilemapType};
use bevy_minibuffer::prelude::*;
use bevy_mod_scripting::core::{asset::ScriptAsset, script::ScriptComponent};
use nano_9::{minibuffer::*, pico8::*, *};
use std::env;

#[derive(Resource)]
struct MyScript(Handle<ScriptAsset>);

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
        .add_plugins(FpsOverlayPlugin {
                config: FpsOverlayConfig {
                    text_config: TextFont {
                        // Here we define size of our overlay
                        font_size: 24.0,
                        // If we want, we can use a custom font
                        font: default(),
                        // We could also disable font smoothing,
                        font_smoothing: FontSmoothing::None,
                    },
                    // We can also change color of the overlay
                    text_color: Color::WHITE,
                    enabled: false,
                },
            })
        .add_acts((
            BasicActs::default(),
            acts::universal::UniversalArgActs::default(),
            acts::tape::TapeActs::default(),
            // bevy_minibuffer_inspector::WorldActs::default(),
            crate::minibuffer::Nano9Acts::default(),
            CountComponentsActs::default()
                .add::<Text>("text")
                .add::<TilemapType>("map")
                .add::<TilePos>("tile")
                .add::<Sprite>("sprite")
                .add::<Clearable>("clearables"),
            toggle_fps
            // inspector::AssetActs::default().add::<Image>(),
        ))
        // .insert_state(ErrorState::Messages { frame: 0 })
        ;
    if script_path.ends_with(".p8") {
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let cart: Handle<Cart> = asset_server.load(&script_path);
                commands.send_event(LoadCart(cart));
                commands.spawn(ScriptComponent(
                    vec![format!("{}#lua", &script_path).into()],
                ));
            },
        );
    } else {
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                commands.insert_resource(MyScript(asset_server.load(script_path.clone())));
                commands.spawn(ScriptComponent(vec![script_path.clone().into()]));
            },
        );
    }
    app.run();
    Ok(())
}

fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
}
