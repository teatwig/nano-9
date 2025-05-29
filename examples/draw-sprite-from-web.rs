use bevy::prelude::*;
use nano9::{config::*, error::RunState, pico8::*, *};
use std::{io, path::Path};

fn init(mut pico8: Pico8) {
    pico8.cls(Some(0)).unwrap();
    // pico8.color(Some(1)).unwrap();
    pico8.spr(0, Vec2::ZERO, None, None, None).unwrap();
}

fn update(pico8: Pico8, x: Local<u32>) {
    // let _ = pico8.pset(UVec2::new(*x, *x), Some(2));
    // let _ = pico8.pset(UVec2::new(*x, *x), None);
    // *x += 1;
}

fn main() -> io::Result<()> {
    let mut app = App::new();
    app.add_systems(OnEnter(RunState::Init), init);
    app.add_systems(Update, update.run_if(in_state(RunState::Run)));

    let mut config = Config::pico8();
    config.sprite_sheets.push(config::SpriteSheet {
        path: "https://img.itch.zone/aW1hZ2UvNzQ0NDEyLzQxNDk3MzQucG5n/original/ESTrjK.png".into(),
        ..default()
    });
    {
        // Make our config readable by the Bevy AssetServer.
        let memory_dir = MemoryDir::default();
        let config_string = toml::to_string(&config).unwrap();
        memory_dir
            .insert_asset(Path::new("Nano9.toml"), config_string.into_bytes());
        app.add_plugins(memory_dir);
    }
    app
        .add_systems(
            Startup,
            move |asset_server: Res<AssetServer>,
            mut commands: Commands| {
                let pico8_asset: Handle<Pico8Asset> = asset_server.load("memory://Nano9.toml");
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            })
        .add_systems(PreUpdate, run_pico8_when_loaded);
    app.add_plugins(Nano9Plugins { config });
    app.run();
    Ok(())
}

fn show_asset_changes<T: Asset>(mut reader: EventReader<AssetEvent<T>>) {
    reader.read().inspect(|e| info!("asset event {e:?}"));
}
