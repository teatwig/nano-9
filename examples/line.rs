use bevy::{
    asset::io::{
        memory::{Dir, MemoryAssetReader},
        AssetSource, AssetSourceId,
    },
    audio::AudioPlugin,
    prelude::*,
};
use nano9::{config::{run_pico8_when_loaded, Config}, error::RunState, pico8::*, *};
use std::{io, path::Path};

#[cfg(feature = "minibuffer")]
use bevy_minibuffer::prelude::*;

fn init(mut pico8: Pico8) {
    pico8.cls(Some(0)).unwrap();
    pico8.color(Some(1)).unwrap();
}

fn update(mut pico8: Pico8, mut x: Local<u32>) {
    let _ = pico8.pset(UVec2::new(*x, *x), Some(2));
    // let _ = pico8.pset(UVec2::new(*x, *x), None);
    *x += 1;
}

fn main() -> io::Result<()> {
    let mut app = App::new();

    app.add_systems(OnExit(RunState::Uninit), init);
    app.add_systems(Update, update.run_if(in_state(RunState::Run)));
    // let config = Config::pico8();
    let config = Config::gameboy();
    {
        let config_string = toml::to_string(&config).unwrap();
        let memory_dir = Dir::default();
        memory_dir
            .insert_asset(Path::new("Nano9.toml"), config_string.into_bytes());
        let reader = MemoryAssetReader {
            root: memory_dir.clone(),
        };
        app.register_asset_source(
            AssetSourceId::from_static("memory"),
            AssetSource::build().with_reader(move || Box::new(reader.clone())),
        );
    }
    let nano9_plugin = Nano9Plugin { config };
    app.add_systems(
        PostStartup,
        move |asset_server: Res<AssetServer>,
              mut commands: Commands,
              next_state: ResMut<NextState<RunState>>| {
            let pico8_asset: Handle<Pico8Asset> = asset_server.load("memory://Nano9.toml");
            commands.insert_resource(Pico8Handle::from(pico8_asset));
        },
    )
    .add_systems(PreUpdate, run_pico8_when_loaded)
        ;
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
