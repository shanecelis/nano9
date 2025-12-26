use crate::{
    Nano9Plugin,
    config::{Config, MemoryDir},
};
use bevy::{
    app::{PluginGroup, PluginGroupBuilder},
    asset::AssetPath,
    audio::{AudioPlugin, Volume},
    prelude::*,
};
#[cfg(feature = "sdl")]
use bevy_window_sdl2_backend::Sdl2WindowBackendPlugin;
#[cfg(feature = "sdl")]
use bevy::winit::WinitPlugin;
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

        let default_plugins = DefaultPlugins
                // .set(AssetPlugin {
                //     mode: AssetMode::Processed,
                //     ..default()
                // })
                .set(AudioPlugin {
                    global_volume: GlobalVolume {
                        volume: Volume::Linear(0.4),
                    },
                    ..default()
                })
                .set(nano9_plugin.window_plugin());

        #[cfg(feature = "sdl")]
        let default_plugins = default_plugins.disable::<WinitPlugin>();

        let group = group.add_group(
            default_plugins
        );
        #[cfg(feature = "sdl")]
        let group = group.add(Sdl2WindowBackendPlugin);

        group.add(nano9_plugin)
    }
}
