use bevy::{
    audio::AudioPlugin,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
    text::FontSmoothing,
};
use bevy_ecs_tilemap::prelude::{TilePos, TilemapType};
use bevy_minibuffer::prelude::*;
use bevy_mod_scripting::core::{asset::ScriptAsset, script::ScriptComponent};
use nano_9::{minibuffer::*, pico8::*, *, config::N9Config};
use std::{fs, env, io};

#[derive(Resource)]
struct MyScript(Handle<ScriptAsset>);

fn main() -> io::Result<()> {
    let args = env::args();
    let script_path: String = args
        .skip(1)
        .next()
        .map(|s| format!("../{s}"))
        .unwrap_or("scripts/main.lua".into());
    let mut app = App::new();
    let nano9_plugin;
    if script_path.ends_with(".toml") {
        let content = fs::read_to_string(script_path)?;
        let config: N9Config = toml::from_str::<N9Config>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?
            .inject_template();
        nano9_plugin = Nano9Plugin {
            config
        };
    } else if script_path.ends_with(".p8") {
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
        nano9_plugin = Nano9Plugin { config: N9Config::pico8() };
    } else {
        nano9_plugin = Nano9Plugin {
            config: N9Config {
                code: Some(script_path.clone().into()),
                ..N9Config::pico8()
            }
        };
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands, mut pico8: Pico8| {
                pico8.state.code = asset_server.load(script_path.clone());
                commands.spawn(ScriptComponent(vec![script_path.clone().into()]));
            },
        );
    }

    app
        .add_plugins(DefaultPlugins
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.4),
                ..default()
            })
                     .set(nano9_plugin.window_plugin()))
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
    app.run();
    Ok(())
}

fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
}
