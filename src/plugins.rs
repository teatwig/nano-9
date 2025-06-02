use crate::{
    config::{Config, MemoryDir},
    Nano9Plugin,
};
use bevy::{
    app::{PluginGroup, PluginGroupBuilder},
    audio::AudioPlugin,
    prelude::*,
};
/// Nano-9 plugins
pub struct Nano9Plugins {
    pub config: Config,
}

impl PluginGroup for Nano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>();
        #[cfg(feature = "web-asset")]
        let group = group.add(bevy_web_asset::WebAssetPlugin);
        let group = group.add(MemoryDir::new("n9mem"));
        let nano9_plugin = Nano9Plugin {
            config: self.config,
        };
        let group = group.add_group(
            DefaultPlugins
                // .set(AssetPlugin {
                //     mode: AssetMode::Processed,
                //     ..default()
                // })
                .set(AudioPlugin {
                    global_volume: GlobalVolume::new(0.4),
                    ..default()
                })
                .set(nano9_plugin.window_plugin()),
        );

        group.add(nano9_plugin)
    }
}
