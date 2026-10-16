#[cfg(feature = "scripting")]
use crate::{call, pico8::lua::with_system_param};
use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
    prelude::*,
    text::FontSmoothing,
};
use bevy_minibuffer::prelude::*;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    bindings::{
        InteropError,
        function::{namespace::NamespaceBuilder, script_function::FunctionCallContext},
        script_value::ScriptValue,
    },
    core::event::ScriptCallbackMessage,
};
// mod count;
// pub use count::*;

#[derive(Debug)]
pub struct Nano9Acts {
    /// Set of acts
    pub acts: Acts,
}

impl Default for Nano9Acts {
    fn default() -> Self {
        Self {
            acts: Acts::new([
                Act::new(crate::action::toggle_pause).bind(keyseq! { Space N P }),
                Act::new(crate::action::one_step).bind(keyseq! { Space N S }),
                #[cfg(feature = "scripting")]
                Act::new(lua_eval).bind(keyseq! { Space N E }),
            ]),
        }
    }
}

fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
    config.frame_time_graph_config.enabled = config.enabled;
}

/// Quick use plugin for Minibuffer, configured similarly to n9.
pub fn quick_plugin(app: &mut App) {
    let font: Handle<Font> = {
        if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
            asset_server.load(crate::config::pico8::FONT)
        } else {
            default()
        }
    };
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            text_config: TextFont::from_font_size(24.0)
                .with_font(font)
                .with_font_smoothing(FontSmoothing::None),
            text_color: Color::WHITE,
            enabled: false,
            frame_time_graph_config: FrameTimeGraphConfig {
                enabled: false,
                ..default()
            },
            ..default()
        },
    });
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        // acts::universal::UniversalArgActs::default(),
        // acts::tape::TapeActs::default(),
        crate::minibuffer::Nano9Acts::default(),
        // CountComponentsActs::default()
        //     .add::<Text>("text")
        //     .add::<TilemapType>("map")
        //     .add::<TilePos>("tile")
        //     .add::<Sprite>("sprite")
        //     .add::<Clearable>("clearables"),
        Act::new(toggle_fps).bind(keyseq! { Space N F }),
    ));

    #[cfg(feature = "inspector")]
    {
        if !app.is_plugin_added::<bevy_egui::EguiPlugin>() {
            info!("bevy_minibuffer_inspector requires EguiPlugin, adding it.");
            app.add_plugins(bevy_egui::EguiPlugin::default());
        }
        // The Pico-8 camera uses `CompositingSpace::Srgb` (`Rgba8Unorm`).
        // bevy_egui's pipeline is `Rgba8UnormSrgb`, so attaching the primary
        // context to that camera crashes when the inspector is shown.
        if let Some(mut settings) = app
            .world_mut()
            .get_resource_mut::<bevy_egui::EguiGlobalSettings>()
        {
            settings.auto_create_primary_context = false;
        }
        app.add_systems(Update, spawn_inspector_overlay).add_acts((
            bevy_minibuffer_inspector::WorldActs::default(),
            bevy_minibuffer_inspector::StateActs::default().add::<crate::run::RunState>(),
            bevy_minibuffer_inspector::AssetActs::default()
                .add::<bevy::prelude::Image>()
                .add::<crate::pico8::Gfx>(),
        ));
    }
}

/// Overlay camera for the in-game world inspector. Lives on the primary window
/// but has no `CompositingSpace`, so its pass matches `egui_pipeline`.
#[cfg(feature = "inspector")]
#[derive(Component)]
struct InspectorOverlayCamera;

#[cfg(feature = "inspector")]
fn spawn_inspector_overlay(
    mut commands: Commands,
    existing: Query<(), With<InspectorOverlayCamera>>,
    windows: Query<Entity, With<bevy::window::PrimaryWindow>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    use bevy::{
        camera::{ClearColorConfig, RenderTarget, visibility::RenderLayers},
        window::WindowRef,
    };

    commands.spawn((
        Name::new("inspector-overlay-camera"),
        InspectorOverlayCamera,
        ChildOf(window),
        Camera3d::default(),
        Camera {
            order: 100,
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(window)),
        RenderLayers::none(),
        bevy_egui::PrimaryEguiContext,
        bevy_egui::EguiContextSettings {
            enable_ime: false,
            ..default()
        },
    ));
}

impl ActsPlugin for Nano9Acts {
    fn acts(&self) -> &Acts {
        &self.acts
    }
    fn acts_mut(&mut self) -> &mut Acts {
        &mut self.acts
    }
}

impl Plugin for Nano9Acts {
    fn build(&self, app: &mut App) {
        self.warn_on_unused_acts();
        let world = app.world_mut();
        #[cfg(feature = "scripting")]
        NamespaceBuilder::<World>::new_unregistered(world).register(
            "message",
            |ctx: FunctionCallContext, s: String| {
                with_minibuffer(&ctx, |minibuffer| {
                    minibuffer.message(s);
                    Ok(())
                })
            },
        );
    }
}

#[cfg(feature = "scripting")]
fn with_minibuffer<T>(
    ctx: &FunctionCallContext,
    f: impl FnOnce(&mut Minibuffer) -> Result<T, Error>,
) -> Result<T, InteropError> {
    with_system_param::<Minibuffer, T, Error>(ctx, f)
}

#[cfg(feature = "scripting")]
pub fn lua_eval(mut minibuffer: Minibuffer) {
    minibuffer.prompt::<TextField>("Lua Eval: ").observe(
        |mut trigger: On<Submit<String>>,
         mut writer: MessageWriter<ScriptCallbackMessage>,
         mut commands: Commands| {
            if let Ok(input) = trigger.event_mut().take_result() {
                writer.write(ScriptCallbackMessage::new_for_all_contexts(
                    call::Eval,
                    vec![ScriptValue::String(input.into()), ScriptValue::Bool(true)],
                ));
            } else {
                commands.entity(trigger.event().entity).despawn();
            }
        },
    );
}
