#[cfg(feature = "scripting")]
use crate::{call, pico8::lua::with_system_param};
use bevy::{
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
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
    core::event::ScriptCallbackEvent,
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
}

/// Quick use plugin for Minibuffer, configured similarly to n9.
pub fn quick_plugin(app: &mut App) {
    let font: Handle<Font> = {
        if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
            asset_server.load(crate::pico8::PICO8_FONT)
        } else {
            default()
        }
    };
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            text_config: TextFont {
                // Here we define size of our overlay
                font_size: 24.0,
                // If we want, we can use a custom font
                font,
                // We could also disable font smoothing,
                font_smoothing: FontSmoothing::None,
                ..default()
            },
            // We can also change color of the overlay
            text_color: Color::WHITE,
            enabled: false,
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
            app.add_plugins(bevy_egui::EguiPlugin {
                enable_multipass_for_primary_context: false,
                ..default()
            });
        }
        app.add_acts((
            bevy_minibuffer_inspector::WorldActs::default(),
            bevy_minibuffer_inspector::StateActs::default().add::<crate::run::RunState>(),
            bevy_minibuffer_inspector::AssetActs::default()
                .add::<bevy::prelude::Image>()
                .add::<crate::pico8::Gfx>(),
        ));
    }
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
         mut writer: EventWriter<ScriptCallbackEvent>,
         mut commands: Commands| {
            if let Ok(input) = trigger.event_mut().take_result() {
                writer.write(ScriptCallbackEvent::new_for_all_contexts(
                    call::Eval,
                    vec![ScriptValue::String(input.into()), ScriptValue::Bool(true)],
                ));
            } else {
                commands.entity(trigger.event().entity).despawn();
            }
        },
    );
}
