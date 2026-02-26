use crate::Nano9Plugin;
use bevy::{
    app::{PluginGroup, PluginGroupBuilder},
    asset::AssetPath,
    audio::{AudioPlugin, Volume},
    image::ImagePlugin,
    prelude::*,
    window::ExitCondition,
};

/// Nano-9 plugins
#[derive(Debug, Default)]
pub struct Nano9Plugins;

// impl Nano9Plugins {
//     pub fn new(config: Config) -> Self {
//         Nano9Plugins {
//             config,
//             config_path: None,
//         }
//     }
// }

impl PluginGroup for Nano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>();
        // TODO: Get rid of this n9mem directory.
        // let group = group.add(MemoryDir::new("n9mem"));
        let nano9_plugin = Nano9Plugin::default();
        // {
        //     config: self.config,
        //     config_path: self.config_path,
        // };
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
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::OnPrimaryClosed,
                    ..default()
                }),
        );

        group.add(nano9_plugin)
    }
}

/// Headless plugin set for tests: no window, no winit event loop.
/// Use this in tests to avoid "EventLoop must be created on the main thread" on macOS.
#[derive(Debug, Default)]
pub struct HeadlessNano9Plugins;

impl PluginGroup for HeadlessNano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(MinimalPlugins)
            .add(bevy::state::app::StatesPlugin)
            .add(AssetPlugin::default())
            .add(ImagePlugin::default_nearest())
            .add(Nano9Plugin::default())
    }
}
