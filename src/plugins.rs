use crate::{
    config::{Config, MemoryDir},
    Nano9Plugin,
};
use bevy::{
    asset::AssetPath,
    app::{PluginGroup, PluginGroupBuilder},
    audio::AudioPlugin,
    prelude::*,
};

/// Nano-9 plugins
#[derive(Debug, Default)]
pub struct Nano9Plugins {
    pub config: Config,
    pub config_path: Option<AssetPath<'static>>,
}

impl Nano9Plugins {
    pub fn new(config: Config) -> Self {
        Nano9Plugins {
            config,
            config_path: None
        }
    }
}

impl PluginGroup for Nano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>();
        #[cfg(feature = "web-asset")]
        let group = group.add(bevy_web_asset::WebAssetPlugin);
        let group = group.add(MemoryDir::new("n9mem"));
        let nano9_plugin = Nano9Plugin {
            config: self.config,
            config_path: self.config_path,
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
