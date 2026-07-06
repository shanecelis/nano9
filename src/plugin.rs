#![allow(deprecated)]
use bevy::{prelude::*, reflect::Reflect, window::PresentMode};
use std::time::Duration;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    BMSPlugin,
    asset::ScriptAsset,
    bindings::{CoreScriptGlobalsPlugin, InteropError, function::namespace::NamespaceBuilder},
    core::{
        callback_labels,
        event::{CallbackLabel, IntoCallbackLabel, ScriptCallbackEvent},
        handler::event_handler,
        script::{ContextPolicy, ScriptContexts},
    },
    lua::LuaScriptingPlugin,
    prelude::ScriptAttachment,
};

use crate::{
    PColor,
    config::Config,
    pico8::{self, FillPat, Pico8Handle, input::fill_input},
    run::RunState,
    schedule,
};

#[derive(Clone, Debug, Reflect)]
pub struct DrawState {
    pub pen: PColor,
    pub camera_position: Vec2,
    pub camera_position_delta: Option<Vec2>,
    pub print_cursor: Vec2,
    pub fill_pat: Option<FillPat>,
    pub is_clear: bool,
}

impl DrawState {
    /// Mark ourselves as having drawn something this frame.
    pub fn mark_drawn(&mut self) {
        self.is_clear = false;
        if self.camera_position_delta.is_none() {
            self.camera_position_delta = Some(Vec2::ZERO);
        }
    }

    #[inline]
    pub fn apply_camera_delta(&self, a: Vec2) -> Vec2 {
        a
        // self.camera_position_delta.map(|d| a + d).unwrap_or(a)
    }

    #[inline]
    pub fn apply_camera_delta_ivec2(&self, a: IVec2) -> IVec2 {
        a
        // self.camera_position_delta
        //     .map(|d| a + d.as_ivec2())
        //     .unwrap_or(a)
    }

    pub fn clear_screen(&mut self) {
        self.print_cursor = Vec2::ZERO;
        self.is_clear = true;
    }
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            pen: PColor::Palette(1),
            camera_position: Vec2::ZERO,
            print_cursor: Vec2::ZERO,
            camera_position_delta: None,
            fill_pat: None,
            is_clear: true,
        }
    }
}

#[cfg(feature = "scripting")]
pub mod call {
    use super::*;
    callback_labels!(
    SetGlobal => "_set_global",
    Update => "_update",
    Update60 => "_update60",
    Init => "_init",
    Eval => "_eval",
    Draw => "_draw");
}

#[cfg(feature = "scripting")]
pub fn send(
    label: impl Into<CallbackLabel>,
) -> impl Fn(MessageWriter<ScriptCallbackEvent>, Option<Res<Pico8Handle>>) {
    let label = label.into();
    move |mut writer: MessageWriter<ScriptCallbackEvent>,
          maybe_pico8_handle: Option<Res<Pico8Handle>>| {
        let maybe_recipients =
            maybe_pico8_handle.and_then(|pico8_handle| pico8_handle.main_script.clone());

        match maybe_recipients {
            Some(recipients) => {
                writer.write(ScriptCallbackEvent::new(
                    label.clone(),
                    vec![],
                    recipients,
                    None,
                ));
            }
            None => {
                writer.write(ScriptCallbackEvent::new_for_all_contexts(
                    label.clone(),
                    vec![],
                ));
            }
        }
    }
}

#[derive(Default)]
pub struct Nano9Plugin;

impl Nano9Plugin {
    // pub fn new(config: Config) -> Self {
    //     Nano9Plugin {
    //         config,
    //         config_path: None,
    //     }
    // }

    pub fn window_plugin(config: &Config) -> WindowPlugin {
        use crate::config::*;
        let screen_size = config
            .screen
            .as_ref()
            .and_then(|s| s.screen_size)
            .unwrap_or(DEFAULT_SCREEN_SIZE);

        let decorations = config
            .screen
            .as_ref()
            .and_then(|s| s.decorations)
            .unwrap_or(DEFAULT_DECORATIONS);

        let resize_constraints = config
            .screen
            .as_ref()
            .and_then(|s| s.resize_constraints.clone())
            .unwrap_or(ResizeConstraints::MatchScreen {
                match_screen: false,
            });
        let resolution: bevy::window::WindowResolution = screen_size.into();
        let resize_constraints = match resize_constraints {
            ResizeConstraints::MatchScreen { match_screen: true } => WindowResizeConstraints {
                min_width: resolution.width(),
                max_width: resolution.width(),
                min_height: resolution.height(),
                max_height: resolution.height(),
            },
            ResizeConstraints::MatchScreen {
                match_screen: false,
            } => WindowResizeConstraints::default(),
            ResizeConstraints::Rect { rect } => WindowResizeConstraints {
                min_width: rect.min.x as f32,
                max_width: rect.max.x as f32,
                min_height: rect.min.y as f32,
                max_height: rect.max.y as f32,
            },
        };
        WindowPlugin {
            primary_window: Some(Window {
                title: config.name.as_deref().unwrap_or("Nano-9").into(),
                // Turn off vsync to maximize CPU/GPU usage
                present_mode: PresentMode::AutoVsync,
                // Let's not allow resizing.
                resize_constraints,
                decorations,
                // decorations: false,
                // resolution: resolution.with_scale_factor_override(1.0),
                resolution,
                ..default()
            }),
            ..default()
        }
    }
}

#[cfg(feature = "scripting")]
fn add_logging(app: &mut App) {
    let world = app.world_mut();
    NamespaceBuilder::<World>::new_unregistered(world)
        .register("info", |s: String| {
            bevy::log::info!("{}", s);
        })
        .register("warn", |s: String| {
            bevy::log::warn!("{}", s);
        })
        .register("error", |s: String| {
            bevy::log::error!("{}", s);
        })
        .register("debug", |s: String| {
            bevy::log::debug!("{}", s);
        })
        .register("trace", |s: String| {
            bevy::log::trace!("{}", s);
        });
}
// use bevy_mod_scripting::bindings::InteropError;

// #[derive(Event)]
// pub struct MyTrigger(usize);

// pub(crate) fn plugin(app: &mut App) {
//     let world = app
//         .world_mut();
//     NamespaceBuilder::<World>::new_unregistered(world)
//         .register("my_trigger", |
//                   ctx: FunctionCallContext,
//                  number: Option<usize>| {
//                      let world_guard = ctx.world()?;
//                      let raid = ReflectAccessId::for_global();
//                      if world_guard.claim_global_access() {
//                          let world = world_guard.as_unsafe_world_cell()?;
//                          let world = unsafe { world.world_mut() };
//                          world.trigger(MyTrigger(number.unwrap_or(0)));
//                          unsafe { world_guard.release_global_access() };
//                          Ok(())
//                      } else {
//                          Err(InteropError::cannot_claim_access(
//                              raid,
//                              world_guard.get_access_location(raid),
//                              "my_trigger",
//                          ))
//                      }
//                  });
// }
//
#[cfg(feature = "scripting")]
fn context_initializer(
    _script_attachment: &ScriptAttachment,
    context: &mut bevy_mod_scripting::lua::LuaContext,
) -> Result<(), InteropError> {
    use bevy_mod_scripting::lua::IntoInteropError;
    context
        .globals()
        .set(
            "_eval_string",
            context
                .create_function(|ctx, arg: String| {
                    ctx.load(format!("tostring({arg})")).eval::<String>()
                })
                .map_err(IntoInteropError::to_bms_error)?,
        )
        .map_err(IntoInteropError::to_bms_error)?;
    context
        .load(include_str!("builtin.lua"))
        .exec()
        .expect("Problem in builtin.lua");
    Ok(())
}

impl Plugin for Nano9Plugin {
    fn build(&self, app: &mut App) {
        // app.register_type::<DrawState>();
        // How do you enable shared context since it eats the plugin?
        // let canvas_size: UVec2 = self
        //     .config
        //     .screen
        //     .as_ref()
        //     .map(|s| s.canvas_size)
        //     .unwrap_or(DEFAULT_CANVAS_SIZE);

        // let asset_path: AssetPath<'static> = self.config_path.clone().unwrap_or_else(|| {
        //     // Make our config readable by the Bevy AssetServer.
        //     //
        //     // I kind of hate this because we have to serialize just to
        //     // deserialize. It also breaks the ability to use bevy/file_watcher.
        //     let config_string = toml::to_string(&self.config).unwrap();
        //     if let Some(memory_dir) = app.world_mut().get_resource_mut::<MemoryDir>() {
        //         memory_dir.insert_asset(
        //             std::path::Path::new("Nano9.toml"),
        //             config_string.into_bytes(),
        //         );
        //         AssetPath::<'static>::from("n9mem://Nano9.toml")
        //     } else {
        //         panic!("No 'n9mem://' asset source configured.");
        //     }
        // });
        // app.add_supported_script_extensions(&[".p8"], Language::Lua);

        // TODO: Add this functionality somewhere.
        // if let Some(asset_path) = self.config_path.clone() {
        //     app.add_systems(
        //         Startup,
        //         move |asset_server: Res<AssetServer>, mut commands: Commands| {
        //             let pico8_asset: Handle<Pico8Asset> = asset_server.load(&asset_path);
        //             commands.insert_resource(Pico8Handle::from(pico8_asset));
        //         },
        //     );
        // }
        #[cfg(feature = "scripting")]
        {
            app.insert_resource(ScriptContexts::<LuaScriptingPlugin>::new(
                ContextPolicy::shared(),
            ));
            let mut lua_scripting_plugin = LuaScriptingPlugin::default();
            lua_scripting_plugin
                .scripting_plugin
                .add_context_initializer(context_initializer);

            // TODO: Add this another day.
            //
            // https://rhai.rs/book/rust/modules/ast.html
            // let mut rhai_scripting_plugin = bevy_mod_scripting::rhai::RhaiScriptingPlugin::default().enable_context_sharing();
            // rhai_scripting_plugin
            //     .scripting_plugin
            //     .add_context_initializer(
            //         |_script_id: &str, context: &mut bevy_mod_scripting::rhai::RhaiScriptContext| {
            //             // context.globals().set(
            //             //     "_eval_string",
            //             //     context.create_function(|ctx, arg: String| {
            //             //         ctx.load(format!("tostring({arg})")).eval::<String>()
            //             //     })?,
            //             // )?;

            //             // context
            //             //     .load(include_str!("builtin.lua"))
            //             //     .exec()
            //             //     .expect("Problem in builtin.lua");
            //             Ok(())
            //         },
            //     );

            // Filter out config types and other duplicates so only one type per short name is registered.
            let globals_plugin = CoreScriptGlobalsPlugin {
                filter: |reg| {
                    let path = reg.type_info().type_path_table().path();
                    // Exclude config types (keep pico8/bevy runtime types for scripts).
                    if path == crate::config::SpriteSheet::type_path()
                        || path == crate::config::Palette::type_path()
                        || path == crate::config::SpriteMap::type_path()
                        || path == crate::config::Mesh::type_path()
                        || path == crate::config::AudioBank::type_path()
                    {
                        return false;
                    }
                    // Exclude core::ops::Range so std::ops::Range is the one registered.
                    if path.starts_with("core::ops::Range") {
                        return false;
                    }
                    // Exclude Arc<StrongHandle> duplicates (alloc vs std); keep neither as global.
                    if path.contains("StrongHandle") {
                        return false;
                    }
                    // Exclude Vec<config::Palette> so only Vec<pico8::Palette> is in the types global.
                    if path.contains("Vec<")
                        && (path.contains("config::Palette") || path.contains("config::SpriteMap"))
                    {
                        return false;
                    }
                    true
                },
                ..Default::default()
            };
            app.add_plugins(BMSPlugin.set(globals_plugin).set(lua_scripting_plugin));
        }
        // let resolution = settings.canvas_size.as_vec2() * settings.pixel_scale;
        app.insert_resource(bevy::winit::WinitSettings {
            // focused_mode: bevy::winit::UpdateMode::Continuous,
            focused_mode: bevy::winit::UpdateMode::reactive(Duration::from_millis(16)),
            unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(Duration::from_millis(
                // We could run it slower here, but that feels bad actually.
                // 16 * 4,
                16,
            )),
        })
        .init_resource::<pico8::Defaults>()
        .add_plugins(crate::plugin);

        #[cfg(feature = "framepace")]
        app.add_plugins(bevy_framepace::FramepacePlugin);

        #[cfg(feature = "scripting")]
        app.add_plugins(add_logging);
        #[cfg(feature = "scripting")]
        app.add_systems(
            Update,
            (
                fill_input,
                (send(call::Init), schedule::run_schedule(schedule::Init))
                    .run_if(init_when::<ScriptAsset>()),
                event_handler::<call::Init, LuaScriptingPlugin>,
                (send(call::Update), schedule::run_schedule(schedule::Update))
                    .run_if(in_state(RunState::Run)),
                event_handler::<call::Update, LuaScriptingPlugin>,
                event_handler::<call::Eval, LuaScriptingPlugin>,
                (send(call::Draw), schedule::run_schedule(schedule::Draw))
                    .run_if(in_state(RunState::Run)),
                event_handler::<call::Draw, LuaScriptingPlugin>,
            )
                .chain(),
        );
        app.add_systems(
            OnEnter(RunState::Init),
            schedule::run_schedule(schedule::Init),
        );
        #[cfg(not(feature = "scripting"))]
        {
            app.add_systems(
                Update,
                (
                    fill_input,
                    schedule::run_schedule(schedule::Update),
                    schedule::run_schedule(schedule::Draw),
                )
                    .chain()
                    .run_if(in_state(RunState::Run)),
            );
        }
        // bevy_ecs_ldtk will add this plugin, so let's not add that if it's
        // present.
        #[cfg(not(feature = "level"))]
        app.add_plugins(bevy_ecs_tilemap::TilemapPlugin);
    }
}

pub fn init_when<T: Asset>()
-> impl FnMut(MessageReader<AssetEvent<T>>, Local<bool>, Res<State<RunState>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: MessageReader<AssetEvent<T>>,
          mut asset_change: Local<bool>,
          state: Res<State<RunState>>| {
        let asset_just_changed = reader
            .read()
            // .inspect(|e| info!("asset event {e:?}"))
            .any(|e| matches!(e, AssetEvent::Added { .. } | AssetEvent::Modified { .. }));
        match **state {
            RunState::Run => {
                // Return true once if the script asset has changed.
                let result = *asset_change | asset_just_changed;
                *asset_change = false;
                result
            }
            _ => {
                *asset_change |= asset_just_changed;
                false
            }
        }
    }
}

pub fn on_asset_change<T: Asset>() -> impl FnMut(MessageReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: MessageReader<AssetEvent<T>>| {
        reader
            .read()
            // .inspect(|e| info!("asset event {e:?}"))
            .any(|e| {
                matches!(
                    e, //AssetEvent::LoadedWithDependencies { .. } |
                    AssetEvent::Added { .. } | AssetEvent::Modified { .. }
                )
            })
    }
}

pub fn on_asset_loaded<T: Asset>() -> impl FnMut(MessageReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: MessageReader<AssetEvent<T>>| {
        reader
            .read()
            .any(|e| matches!(e, AssetEvent::LoadedWithDependencies { .. }))
    }
}

pub fn on_asset_modified<T: Asset>() -> impl FnMut(MessageReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: MessageReader<AssetEvent<T>>| {
        reader
            .read()
            .any(|e| matches!(e, AssetEvent::Modified { .. }))
    }
}

pub fn info_on_asset_event<T: Asset>() -> impl FnMut(MessageReader<AssetEvent<T>>) {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: MessageReader<AssetEvent<T>>| {
        for event in reader.read() {
            match event {
                AssetEvent::Modified { .. } => (),
                _ => {
                    info!("ASSET EVENT {:?}", &event);
                }
            }
        }
    }
}

#[cfg(feature = "scripting")]
fn run_script_event_handler<L: IntoCallbackLabel>(
    world: &mut World,
    state: &mut SystemState<Local<MessageCursor<ScriptCallbackEvent>>>,
) -> Result<(), BevyError> {
    let _ = event_handler::<L, LuaScriptingPlugin>(world, state)?;
    Ok(())
}
