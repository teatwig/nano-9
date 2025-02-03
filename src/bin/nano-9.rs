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
use nano_9::{minibuffer::*, pico8::*, *, config::Config};
use std::{fs, ffi::OsStr, env, io, path::{Path, PathBuf}, borrow::Cow};

fn main() -> io::Result<()> {
    let args = env::args();
    let script: String = args
        .skip(1)
        .next()
        // .map(|s| format!("../{s}"))
        .unwrap_or("scripts/main.lua".into());
    dbg!(&script);
    let script_path = {
        let mut path = PathBuf::from(&script);
    dbg!(&path);
        if path.is_dir() {
            path.push("nano9.toml")
        }
    dbg!(&path);
        path
    };
    let mut app = App::new();
    let pwd = AssetSourceId::Name("pwd".into());
    let mut builder = AssetSourceBuilder::platform_default(env::current_dir()?.to_str().expect("pwd dir"), None);
    builder.watcher = None;

    app.register_asset_source(&pwd,
                              builder);
    // let source = AssetSourceId::Name("nano9".into());
    let source = AssetSourceId::Default;
    // if let Ok(cwd) = env::current_dir() {
    //     app.register_asset_source("cwd",
    //                               AssetSourceBuilder::platform_default

    // }
    let nano9_plugin;
    if script_path.extension() == Some(OsStr::new("toml")) {
        eprintln!("loading config");
        let path = &script_path;
        if let Some(parent) = path.parent() {
            app.register_asset_source(&source,
                                      AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None));

        }
        let content = fs::read_to_string(path)?;

        let config: Config = toml::from_str::<Config>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?
            .inject_template();
        let cmd = config.clone();
        app.add_systems(
            PostStartup,
            move |asset_server: Res<AssetServer>, mut commands: Commands, mut pico8: Pico8| {
                // let asset_path = AssetPath::from_path(&code_path).with_source(&source);
                // pico8.state.code = asset_server.load(&asset_path);
                // dbg!(&asset_path);
                // commands.spawn(ScriptComponent(vec![script_path.clone().into()]));
                // commands.spawn(ScriptComponent(vec![asset_path.to_string().into()]));
                commands.queue(cmd.clone())
            },
        );
        nano9_plugin = Nano9Plugin {
            config
        };
    } else if script_path.ends_with(".p8") {
        eprintln!("loading cart");
        let path = PathBuf::from(script_path.clone());
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let asset_path = AssetPath::from_path(&path).with_source(&pwd);
                let cart: Handle<Cart> = asset_server.load(&asset_path);
                commands.send_event(LoadCart(cart));
                // commands.spawn(ScriptComponent(vec![asset_path.path().to_str().unwrap().to_string().into()]));
                commands.spawn(ScriptComponent(
                    // vec![format!("{}#lua", &script_path).into()],
                    vec![format!("{}#lua", &asset_path.path().to_str().unwrap()).into()],
                ));
            },
        );
        nano9_plugin = Nano9Plugin { config: Config::pico8() };
    } else {
        eprintln!("loading lua");
        let path = PathBuf::from(script_path.clone());
        let asset_path = AssetPath::from_path(&path).with_source(&source);
        if let Some(parent) = path.parent().map(Cow::Borrowed).or_else(|| env::current_dir().ok().map(Cow::Owned)) {
            app.register_asset_source(&source,
                                      AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None));

        }
        nano9_plugin = Nano9Plugin {
            config: Config {
                code: Some(asset_path.to_string().into()),
                ..Config::default()
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
