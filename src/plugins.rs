use crate::{
    Nano9Plugin,
    config::{Config, MemoryDir},
};
use bevy::{
    app::{PluginGroup, PluginGroupBuilder},
    asset::AssetPath,
    audio::{AudioPlugin, Volume},
    image::ImagePlugin,
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
            config_path: None,
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
                // Preserve crisp pixel art by default.
                //
                // TODO: I don't necessarily want to do this because it's a
                // global setting. But currently the images that I use with
                // bevy_ecs_tilemap do not seem to be using nearest, so this is
                // the fix for now.
                .set(ImagePlugin::default_nearest())
                .set(AudioPlugin {
                    global_volume: GlobalVolume {
                        volume: Volume::Linear(0.4),
                    },
                    ..default()
                })
                .set(nano9_plugin.window_plugin()),
        );

        group.add(nano9_plugin)
    }
}
