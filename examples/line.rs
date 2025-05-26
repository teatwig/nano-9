use bevy::{
    audio::AudioPlugin,
    prelude::*,
};
use nano9::{config::{MemoryDir, run_pico8_when_loaded, Config}, error::RunState, pico8::*, *};
use std::{io, path::Path};

fn init(mut pico8: Pico8) {
    pico8.cls(Some(0)).unwrap();
    pico8.color(Some(1)).unwrap();
}

fn update(mut pico8: Pico8, mut x: Local<u32>) {
    // let _ = pico8.pset(UVec2::new(*x, *x), Some(2));
    let _ = pico8.pset(UVec2::new(*x, *x), None);
    *x += 1;
}

fn main() -> io::Result<()> {
    let mut app = App::new();
    app.add_systems(OnEnter(RunState::Init), init);
    app.add_systems(Update, update.run_if(in_state(RunState::Run)));

    // let config = Config::pico8();
    let config = Config::gameboy();
    {
        // Make our config readable by the Bevy AssetServer.
        let mut memory_dir = MemoryDir::default();
        let config_string = toml::to_string(&config).unwrap();
        memory_dir
            .insert_asset(Path::new("Nano9.toml"), config_string.into_bytes());
        app.add_plugins(memory_dir);
    }
    let nano9_plugin = Nano9Plugin { config };
    app
        .add_systems(
            Startup,
            move |asset_server: Res<AssetServer>,
            mut commands: Commands| {
                let pico8_asset: Handle<Pico8Asset> = asset_server.load("memory://Nano9.toml");
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            })
        .add_systems(PreUpdate, run_pico8_when_loaded);
    app.add_plugins((
        DefaultPlugins
            .set(AudioPlugin {
                global_volume: GlobalVolume::new(0.4),
                ..default()
            })
            .set(nano9_plugin.window_plugin()),
        nano9_plugin));
    app.run();
    Ok(())
}

fn show_asset_changes<T: Asset>(mut reader: EventReader<AssetEvent<T>>) {
    reader.read().inspect(|e| info!("asset event {e:?}"));
}
