use bevy::{
    asset::{
        io::{AssetSourceBuilder, AssetSourceId},
        AssetPath,
    },
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
    text::FontSmoothing,
};
#[cfg(feature = "minibuffer")]
use bevy_minibuffer::prelude::*;
use nano9::{config::{Config, front_matter, run_pico8_when_loaded}, pico8::{Pico8Handle, Pico8Asset}, *};
use std::{env, ffi::OsStr, fs, io, path::PathBuf, process::ExitCode};

fn usage(mut output: impl io::Write) -> io::Result<()> {
    writeln!(output, "usage: n9 <FILE>")?;
    // XXX: Rewrite this to show what it accepts based on its feature flags.
    writeln!(
        output,
        "Nano-9 accepts cart.p8, cart.p8.png, code.lua, or game[/Nano9.toml] files."
    )
}

fn main() -> io::Result<ExitCode> {
    let example_files = [
        "cart.p8",
        "cart.p8.png",
        "code.lua", // Lua
        // "code.pua", // Pico-8 dialect
        "game-dir",
        "game-dir/Nano9.toml",
        "code.n9",
    ];
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
    let cwd = AssetSourceId::Name("cwd".into());
    let mut builder =
        AssetSourceBuilder::platform_default(dbg!(env::current_dir()?.to_str().expect("current dir")), None);
    builder.watcher = None;
    builder.processed_watcher = None;

    app.register_asset_source(&cwd, builder);
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
        // OLD SHANE: Get rid of this.
        //
        // NEW SHANE: No. We use part of Config to configure the App and can't
        // do that at load time.
        let content = fs::read_to_string(path)?;
        let mut config: Config = toml::from_str::<Config>(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;

        if let Some(template) = config.template.take() {
            if let Err(e) = config.inject_template(&template) {
                eprintln!("error: {e}");
                return Ok(ExitCode::from(2));
            }
        }
        nano9_plugin = Nano9Plugin { config };
    } else if script_path.extension() == Some(OsStr::new("p8"))
        || script_path.extension() == Some(OsStr::new("png"))
    {
        eprintln!("loading cart");
        let mut config = Config::pico8();

        // let asset_path = AssetPath::from_path(&script_path).into_owned().with_source(&cwd).with_label("lua");
        // config.code = Some(asset_path.to_string());
        let path = script_path.clone();
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let asset_path = AssetPath::from_path(&path).with_source(&cwd);
                let pico8_asset: Handle<Pico8Asset> = asset_server.load(&asset_path);
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            },
        );
        nano9_plugin = Nano9Plugin {
            config,
        };
    } else if script_path.extension() == Some(OsStr::new("lua"))
    {
        eprintln!("loading lua");
        let mut content = fs::read_to_string(&script_path)?;

        let mut config = if let Some(front_matter) = front_matter::LUA.parse_in_place(&mut content) {
            let mut config: Config = toml::from_str::<Config>(&front_matter)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            if let Some(template) = config.template.take() {
                config.inject_template(&template)
                      .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
            }
            config
        } else {
            Config::pico8()
        };
        config.code = Some(AssetPath::from_path(&script_path).with_source(&cwd).to_string());
        nano9_plugin = Nano9Plugin { config };
    } else {
        eprintln!("Only accepts .p8, .png, .lua, and .toml files.");

        return Ok(ExitCode::from(1));
    }

    app.add_plugins(
        Nano9Plugins { config: nano9_plugin.config }
    )
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
    .add_systems(PreUpdate, run_pico8_when_loaded);

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
