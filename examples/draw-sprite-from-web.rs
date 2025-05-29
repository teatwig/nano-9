use bevy::prelude::*;
use nano9::{config::*, error::RunState, pico8::*, *};
use std::{io, path::Path};

fn init(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
    // pico8.color(Some(1)).unwrap();
    pico8.spr(0, Vec2::ZERO, None, None, None).unwrap();
}

fn update(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
    let t = pico8.time();

    let n = t % 8.0 + 8.0;

    pico8.spr(n as usize, Vec2::ZERO, None, None, None).unwrap();
}

fn main() {
    let mut app = App::new();
    app.add_systems(OnEnter(RunState::Init), init);
    app.add_systems(Update, update.run_if(in_state(RunState::Run)));

    let mut config = Config::pico8();
    config.sprite_sheets.push(config::SpriteSheet {
        path: "BirdSprite.png".into(),
        sprite_size: Some(UVec2::splat(16)),
        ..default()
    });
    app
        .add_systems(PreUpdate, run_pico8_when_loaded);
    app
        .add_plugins(Nano9Plugins { config })
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .run();
}

fn show_asset_changes<T: Asset>(mut reader: EventReader<AssetEvent<T>>) {
    reader.read().inspect(|e| info!("asset event {e:?}"));
}
