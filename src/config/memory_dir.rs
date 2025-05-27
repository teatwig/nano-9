use bevy::{
    asset::io::{
        memory::{Dir, MemoryAssetReader},
        AssetSource, AssetSourceId,
    },
    audio::AudioPlugin,
    prelude::*,
};

/// This plugin provides a "memory://path.ext" asset source.
///
/// It is not actually Nano-9 specific.
///
/// ```
/// use bevy::prelude::*;
/// use nano9::{config::{Config, MemoryDir}, pico8::{Pico8Asset, Pico8Handle}};
/// use std::path::Path;
///
/// fn plugin(app: &mut App) {
///     let config = Config::gameboy();
///     // Make our config readable by the Bevy AssetServer.
///     let mut memory_dir = MemoryDir::default();
///     let config_string = toml::to_string(&config).unwrap();
///     memory_dir
///         .insert_asset(Path::new("Nano9.toml"), config_string.into_bytes());
///     app
///        .add_plugins(memory_dir)
///        .add_systems(
///         Startup,
///         move |asset_server: Res<AssetServer>,
///             mut commands: Commands| {
///             let pico8_asset: Handle<Pico8Asset> = asset_server.load("memory://Nano9.toml");
///             commands.insert_resource(Pico8Handle::from(pico8_asset));
///         });
/// }
/// ```
#[derive(Debug, Deref, DerefMut)]
pub struct MemoryDir {
    /// The name of the asset source
    ///
    /// It is "memory" by default.
    pub source: &'static str,
    #[deref]
    pub dir: Dir,
}

impl Default for MemoryDir {
    fn default() -> Self {
        Self {
            source: "memory",
            dir: Dir::default()
        }
    }
}

impl Plugin for MemoryDir {
    fn build(&self, app: &mut App) {
        let reader = MemoryAssetReader {
            root: self.dir.clone(),
        };
        app.register_asset_source(
            AssetSourceId::from_static(self.source),
            AssetSource::build().with_reader(move || Box::new(reader.clone())),
        );
    }
}
