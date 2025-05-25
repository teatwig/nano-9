use bevy::{
    asset::io::{
        memory::{Dir, MemoryAssetReader},
        AssetSource, AssetSourceId,
    },
    audio::AudioPlugin,
    prelude::*,
};
use nano9::{config::Config, error::RunState, pico8::*, *};
use std::{io, path::Path};

#[cfg(feature = "minibuffer")]
use bevy_minibuffer::prelude::*;
#[allow(dead_code)]
#[derive(Resource)]
struct InitState(Handle<Pico8State>);
#[derive(Resource)]
struct MemoryDir {
    dir: Dir,
}

fn init(mut pico8: Pico8) {
    pico8.cls(Some(0)).unwrap();
    let n = pico8.paln(None).unwrap();

    let UVec2 {
        x: width,
        y: height,
    } = pico8.canvas_size();
    let dw = width as f32 / n as f32;

    for i in 0..n {
        pico8
            .rectfill(
                Vec2::new(i as f32 * dw, 0.0),
                Vec2::new((i + 1) as f32 * dw, height as f32),
                Some(i),
            )
            .unwrap();
    }
}

// fn update(mut pico8: Pico8, mut x: Local<u32>) {
//     let _ = pico8.pset(UVec2::new(*x, *x), None);
//     *x += 1;
// }

fn main() -> io::Result<()> {
    let mut app = App::new();

    app.add_systems(OnExit(RunState::Uninit), init);
    // app.add_systems(Update, update);
    // let config = Config::pico8();
    let config = Config::gameboy();
    {
        let config_string = toml::to_string(&config).unwrap();
        let mut memory_dir = MemoryDir {
            dir: Dir::default(),
        };
        memory_dir
            .dir
            .insert_asset(Path::new("Nano9.toml"), config_string.into_bytes());
        let reader = MemoryAssetReader {
            root: memory_dir.dir.clone(),
        };
        app.register_asset_source(
            AssetSourceId::from_static("memory"),
            AssetSource::build().with_reader(move || Box::new(reader.clone())),
        );
    }
    let nano9_plugin = Nano9Plugin { config: config };
    app.add_systems(
        PostStartup,
        move |asset_server: Res<AssetServer>,
              mut commands: Commands,
              mut next_state: ResMut<NextState<RunState>>| {
            let pico8_state: Handle<Pico8State> = asset_server.load("memory://Nano9.toml");
            commands.insert_resource(InitState(pico8_state));
        },
    )
    .add_systems(PreUpdate, run_pico8_when_ready);
    app.add_plugins(
        DefaultPlugins
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.4),
                ..default()
            })
            .set(nano9_plugin.window_plugin()),
    )
    .add_plugins(nano9_plugin);
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
        // toggle_fps, // inspector::AssetActs::default().add::<Image>(),
    ));

    #[cfg(all(feature = "minibuffer", feature = "inspector"))]
    app.add_acts((
        bevy_minibuffer_inspector::WorldActs::default(),
        bevy_minibuffer_inspector::StateActs::default().add::<RunState>(),
    ));

    app.run();
    Ok(())
}

fn show_asset_changes<T: Asset>(mut reader: EventReader<AssetEvent<T>>) {
    reader.read().inspect(|e| info!("asset event {e:?}"));
}
