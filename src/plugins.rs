use crate::{Nano9Plugin, headless};
#[cfg(target_arch = "wasm32")]
use bevy::asset::AssetMetaCheck;
#[cfg(target_arch = "wasm32")]
use bevy::render::{
    RenderPlugin,
    settings::{Backends, RenderCreation, WgpuSettings, WgpuSettingsPriority},
};
use bevy::{
    app::{PluginGroup, PluginGroupBuilder, ScheduleRunnerPlugin},
    audio::{AudioPlugin, Volume},
    image::ImagePlugin,
    prelude::*,
    window::ExitCondition,
    winit::WinitPlugin,
};
use std::time::Duration;

/// Nano-9 plugins
#[derive(Debug, Default)]
pub struct Nano9Plugins;

/// Canvas selector Bevy binds on wasm. Must exist in `web/index.html` before GPU init.
pub const WASM_CANVAS_SELECTOR: &str = "#nano9-canvas";

fn wasm_primary_window() -> Option<Window> {
    #[cfg(target_arch = "wasm32")]
    {
        // Keep the GPU buffer at Pico-8 screen size. `fit_canvas_to_parent`
        // copies the CSS layout size (often >2048 on a wide display) into the
        // swapchain, which exceeds WebGL2's max texture dimension.
        Some(Window {
            canvas: Some(WASM_CANVAS_SELECTOR.into()),
            fit_canvas_to_parent: false,
            prevent_default_event_handling: false,
            visible: true,
            resolution: crate::config::DEFAULT_SCREEN_SIZE.into(),
            ..default()
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

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
        let nano9_plugin = Nano9Plugin;
        // {
        //     config: self.config,
        //     config_path: self.config_path,
        // };
        #[allow(unused_mut)]
        let mut default_plugins = DefaultPlugins
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
                // Native: spawn the window later from Nano9.toml.
                // Wasm/WebGL: wgpu only enumerates a GPU if a canvas exists
                // when RenderPlugin asks for an adapter.
                primary_window: wasm_primary_window(),
                exit_condition: ExitCondition::OnPrimaryClosed,
                ..default()
            });
        #[cfg(target_arch = "wasm32")]
        {
            default_plugins = default_plugins
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                        backends: Some(Backends::GL),
                        priority: WgpuSettingsPriority::WebGL2,
                        ..default()
                    })),
                    ..default()
                });
        }
        let group = group.add_group(default_plugins);

        group.add(nano9_plugin)
    }
}

/// GPU headless: no Winit window. Camera renders to a canvas-sized image.
///
/// Use with `n9 run --headless`. Drive the loop with [`ScheduleRunnerPlugin`].
#[derive(Debug, Default)]
pub struct HeadlessNano9Plugins;

impl PluginGroup for HeadlessNano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(headless::plugin)
            .add_group(
                DefaultPlugins
                    .set(ImagePlugin::default_nearest())
                    .set(AudioPlugin {
                        global_volume: GlobalVolume {
                            volume: Volume::Linear(0.0),
                        },
                        ..default()
                    })
                    .set(WindowPlugin {
                        primary_window: None,
                        exit_condition: ExitCondition::DontExit,
                        ..default()
                    })
                    .disable::<WinitPlugin>(),
            )
            .add(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            )))
            .add(Nano9Plugin)
    }
}

/// Headless plugin set for tests: no window, no winit event loop, no GPU.
/// Use this in tests to avoid "EventLoop must be created on the main thread" on macOS.
#[cfg(test)]
#[derive(Debug, Default)]
pub struct TestHeadlessNano9Plugins;

#[cfg(test)]
impl PluginGroup for TestHeadlessNano9Plugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add_group(MinimalPlugins)
            .add(bevy::state::app::StatesPlugin)
            .add(AssetPlugin::default())
            .add(ImagePlugin::default_nearest())
            .add(Nano9Plugin)
    }
}
