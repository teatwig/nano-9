use bevy::{
    asset::{AssetPath, io::{AssetSourceId, AssetSourceBuilder}},
    audio::AudioPlugin,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
    text::FontSmoothing,
};
use bevy_ecs_tilemap::prelude::{TilePos, TilemapType};
use bevy_minibuffer::prelude::*;
use bevy_mod_scripting::core::{asset::ScriptAsset, script::ScriptComponent};
use nano_9::{minibuffer::*, pico8::*, *, config::N9Config};
use std::{fs, env, io, path::{Path, PathBuf}, borrow::Cow};

fn main() -> io::Result<()> {
    let args = env::args();
    let script_path: String = args
        .skip(1)
        .next()
        // .map(|s| format!("../{s}"))
        .unwrap_or("scripts/main.lua".into());
    let source = AssetSourceId::Name("nano9".into());
    let mut app = App::new();
    // if let Ok(cwd) = env::current_dir() {
    //     app.register_asset_source("cwd",
    //                               AssetSourceBuilder::platform_default

    // }
    let nano9_plugin;
    if script_path.ends_with(".toml") {
        let path = PathBuf::from(script_path);
        if let Some(parent) = path.parent() {
            app.register_asset_source(&source,
                                      AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None));

        }
        let content = fs::read_to_string(path)?;

        let config: N9Config = toml::from_str::<N9Config>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?
            .inject_template();
        if let Some(ref code_path) = config.code {
            // let code_asset_path = AssetPath::from_path(&code_path).with_source(source);
            // let code_path: PathBuf = code_asset_path.into();
            let code_path = code_path.to_owned();


        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands, mut pico8: Pico8| {
                let asset_path = AssetPath::from_path(&code_path).with_source(&source);
                pico8.state.code = asset_server.load(&asset_path);
                dbg!(&asset_path);
                // commands.spawn(ScriptComponent(vec![script_path.clone().into()]));
                commands.spawn(ScriptComponent(vec![asset_path.to_string().into()]));
            },
        );
        }
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
        let path = PathBuf::from(script_path.clone());
        let asset_path = AssetPath::from_path(&path).with_source(&source);
        if let Some(parent) = path.parent().map(Cow::Borrowed).or_else(|| env::current_dir().ok().map(Cow::Owned)) {
            app.register_asset_source(&source,
                                      AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None));

        }
        nano9_plugin = Nano9Plugin {
            config: N9Config {
                code: Some(asset_path.to_string().into()),
                ..N9Config::default()
            }.with_default_font()
        };
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands, mut pico8: Pico8| {
                let asset_path = AssetPath::from_path(&path).with_source(&source);
                pico8.state.code = asset_server.load(&asset_path);
                // commands.spawn(ScriptComponent(vec![asset_path.to_string().into()]));
                commands.spawn(ScriptComponent(vec![asset_path.path().to_str().unwrap().to_string().into()]));
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
