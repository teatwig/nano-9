use bevy::{
    asset::{
        io::{AssetSourceBuilder, AssetSourceId},
        AssetPath,
    },
    audio::AudioPlugin,
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
    text::FontSmoothing,
};
#[cfg(feature = "minibuffer")]
use bevy_minibuffer::prelude::*;
use bevy_mod_scripting::core::script::ScriptComponent;
use nano9::{config::Config, error::RunState, pico8::*, *};
use std::{borrow::Cow, env, ffi::OsStr, fs, io, path::PathBuf, process::ExitCode};

#[allow(dead_code)]
#[derive(Resource)]
struct InitState(Handle<Pico8State>);

fn usage(mut output: impl io::Write) -> io::Result<()> {
    writeln!(output, "usage: n9 <FILE>")?;
    writeln!(
        output,
        "Nano-9 accepts cart.p8, cart.p8.png, or game[/Nano9.toml] files."
    )
}

fn main() -> io::Result<ExitCode> {
    let mut args = env::args();
    let Some(arg) = args.nth(1) else {
        usage(std::io::stderr())?;
        return Ok(ExitCode::from(2));
    };
    if arg == "--help" || arg == "-h" {
        usage(std::io::stdout())?;
        return Ok(ExitCode::from(0));
    }
    let script = arg;
    let script_path = {
        let mut path = PathBuf::from(&script);
        if path.is_dir() {
            path.push("Nano9.toml")
        }
        path
    };
    let mut app = App::new();
    let pwd = AssetSourceId::Name("pwd".into());
    let mut builder =
        AssetSourceBuilder::platform_default(env::current_dir()?.to_str().expect("pwd dir"), None);
    builder.watcher = None;
    builder.processed_watcher = None;

    app.register_asset_source(&pwd, builder);
    let source = AssetSourceId::Default;
    let nano9_plugin;
    if script_path.extension() == Some(OsStr::new("toml")) {
        eprintln!("loading config");
        let path = &script_path;
        if let Some(parent) = path.parent() {
            app.register_asset_source(
                &source,
                AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None),
            );
        }
        // Get rid of this.
        let content = fs::read_to_string(path)?;

        let mut config: Config = toml::from_str::<Config>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;

        if let Some(template) = config.template.take() {
            if let Err(e) = config.inject_template(&template) {
                eprintln!("error: {e}");
                return Ok(ExitCode::from(2));
            }
        }
        // let cmd = config.clone();
        app.add_systems(
            PostStartup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let pico8state: Handle<Pico8State> = asset_server.load("nano9.toml");
                commands.insert_resource(InitState(pico8state));
            },
        );
        nano9_plugin = Nano9Plugin { config };
    } else if script_path.extension() == Some(OsStr::new("p8"))
        || script_path.extension() == Some(OsStr::new("png"))
    {
        eprintln!("loading cart");
        let path = script_path.clone();
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let asset_path = AssetPath::from_path(&path).with_source(&pwd);
                let pico8state: Handle<Pico8State> = asset_server.load(&asset_path);
                commands.insert_resource(InitState(pico8state));
            },
        );
        nano9_plugin = Nano9Plugin {
            config: Config::pico8(),
        };
    } else if script_path.extension() == Some(OsStr::new("lua")) {
        eprintln!("loading lua");
        let path = script_path.clone();
        let asset_path = AssetPath::from_path(&path).with_source(&source);
        if let Some(parent) = path
            .parent()
            .map(Cow::Borrowed)
            .or_else(|| env::current_dir().ok().map(Cow::Owned))
        {
            app.register_asset_source(
                &source,
                AssetSourceBuilder::platform_default(parent.to_str().expect("parent dir"), None),
            );
        }
        nano9_plugin = Nano9Plugin {
            config: Config {
                code: Some(asset_path.to_string().into()),
                ..Config::default()
            }
            .with_default_font(),
        };
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands, mut pico8: Pico8| {
                //, script_settings: Res<ScriptAssetSettings>| {
                let asset_path = AssetPath::from_path(&path).with_source(&source);
                pico8.state.code = asset_server.load(&asset_path);
                // let script_path = script_settings.script_id_mapper.map(pico8.state.code.path());
                // commands.spawn(ScriptComponent(vec![asset_path.to_string().into()]));
                commands.spawn(ScriptComponent(vec![asset_path
                    .path()
                    .to_str()
                    .unwrap()
                    .to_string()
                    .into()]));
            },
        );
    } else {
        eprintln!("Only accepts .p8, .lua, and .toml files.");

        return Ok(ExitCode::from(1));
    }

    app.add_plugins(
        DefaultPlugins
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.4),
                ..default()
            })
            .set(nano9_plugin.window_plugin()),
    )
    .add_plugins(nano9_plugin)
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
    .add_systems(PreUpdate, run_pico8_when_ready);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        acts::universal::UniversalArgActs::default(),
        acts::tape::TapeActs::default(),
        crate::minibuffer::Nano9Acts::default(),
        // CountComponentsActs::default()
        //     .add::<Text>("text")
        //     .add::<TilemapType>("map")
        //     .add::<TilePos>("tile")
        //     .add::<Sprite>("sprite")
        //     .add::<Clearable>("clearables"),
        toggle_fps, // inspector::AssetActs::default().add::<Image>(),
    ));

    #[cfg(all(feature = "minibuffer", feature = "inspector"))]
    app.add_acts((
        bevy_minibuffer_inspector::WorldActs::default(),
        bevy_minibuffer_inspector::StateActs::default().add::<RunState>(),
    ));
    #[cfg(all(feature = "level", feature = "user_properties"))]
    app.add_systems(Startup, |reg: Res<AppTypeRegistry>| {
        bevy_ecs_tiled::map::export_types(&reg, "all-export-types.json", |name| true);
        bevy_ecs_tiled::map::export_types(&reg, "export-types.json", |name| {
            name.contains("bevy_ecs_tilemap::tiles") || name.contains("nano9")
        });
    });
    app.run();

    Ok(ExitCode::from(0))
}

#[cfg(feature = "minibuffer")]
fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
}
