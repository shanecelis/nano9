#![allow(deprecated)]
use bevy::{
    asset::AssetPath,
    prelude::*,
    reflect::Reflect,
    utils::Duration,
    window::{PresentMode, PrimaryWindow, WindowMode},
};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    core::{
        ConfigureScriptAssetSettings,
        asset::{Language, ScriptAsset},
        bindings::{function::namespace::NamespaceBuilder},
        callback_labels,
        event::{Recipients, ScriptCallbackEvent, CallbackLabel},
        handler::event_handler,
        ConfigureScriptPlugin,
        script::{ScriptContext, ContextKey},
    },
    lua::LuaScriptingPlugin,
    BMSPlugin,
};

use crate::{
    config::*,
    run::RunState,
    pico8::{self, input::fill_input, FillPat, Pico8Asset, Pico8Handle, canvas::N9Canvas},
    PColor,
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
        self.camera_position_delta.map(|d| a + d).unwrap_or(a)
    }

    #[inline]
    pub fn apply_camera_delta_ivec2(&self, a: IVec2) -> IVec2 {
        self.camera_position_delta
            .map(|d| a + d.as_ivec2())
            .unwrap_or(a)
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

pub fn fullscreen_key(
    input: Res<ButtonInput<KeyCode>>,
    mut primary_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if input.just_pressed(KeyCode::Enter)
        && input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight])
    {
        use WindowMode::*;
        let mut primary_window = primary_windows.single_mut();
        primary_window.mode = match primary_window.mode {
            Windowed => Fullscreen(MonitorSelection::Current),
            _ => Windowed,
        }
    }
}


#[cfg(feature = "scripting")]
pub fn send(label: impl Into<CallbackLabel>) -> impl Fn(EventWriter<ScriptCallbackEvent>,
                                                        Option<Res<Pico8Handle>>) {
    let label = label.into();
    move |mut writer: EventWriter<ScriptCallbackEvent>,
    maybe_pico8_handle: Option<Res<Pico8Handle>>| {
        let maybe_id = maybe_pico8_handle.and_then(|pico8_handle| pico8_handle.main_script.clone());

        match maybe_id {
            Some(id) => {
                writer.send(ScriptCallbackEvent::new(
                    label.clone(),
                    vec![],
                    Recipients::All,
                    // Recipients::Script("main.lua".into())
                ));
            }
            None => {
                writer.send(ScriptCallbackEvent::new_for_all(
                    label.clone(),
                    vec![],
                ));
            }
        }
    }
}
const DEFAULT_FRAMES_PER_SECOND: u8 = 60;

#[derive(Default)]
pub struct Nano9Plugin {
    pub config: Config,
    pub config_path: Option<AssetPath<'static>>,
}

impl Nano9Plugin {

    pub fn new(config: Config) -> Self {
        Nano9Plugin {
            config,
            config_path: None
        }
    }

    pub fn window_plugin(&self) -> WindowPlugin {
        let screen_size = self
            .config
            .screen
            .as_ref()
            .and_then(|s| s.screen_size)
            .unwrap_or(DEFAULT_SCREEN_SIZE);
        WindowPlugin {
            primary_window: Some(Window {
                resolution: screen_size.as_vec2().into(),
                title: self.config.name.as_deref().unwrap_or("Nano-9").into(),
                // Turn off vsync to maximize CPU/GPU usage
                present_mode: PresentMode::AutoVsync,
                // Let's not allow resizing.
                // resize_constraints: WindowResizeConstraints {
                //     min_width: resolution.x,
                //     max_width: resolution.x,
                //     min_height: resolution.y,
                //     max_height: resolution.y,
                // },
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
// use bevy_mod_scripting::core::error::InteropError;
// use bevy_mod_scripting::core::bindings::ReflectAccessId;
// use bevy_mod_scripting::core::bindings::FunctionCallContext;

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

impl Plugin for Nano9Plugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DrawState>();
        // How do you enable shared context since it eats the plugin?
        let canvas_size: UVec2 = self
            .config
            .screen
            .as_ref()
            .map(|s| s.canvas_size)
            .unwrap_or(DEFAULT_CANVAS_SIZE);

        let asset_path: AssetPath<'static> = self.config_path.clone().unwrap_or_else(|| {
            // Make our config readable by the Bevy AssetServer.
            //
            // I kind of hate this because we have to serialize just to
            // deserialize. It also breaks the ability to use bevy/file_watcher.
            let config_string = toml::to_string(&self.config).unwrap();
            if let Some(memory_dir) = app.world_mut().get_resource_mut::<MemoryDir>() {
                memory_dir.insert_asset(
                    std::path::Path::new("Nano9.toml"),
                    config_string.into_bytes(),
                );
                AssetPath::<'static>::from("n9mem://Nano9.toml")
            } else {
                panic!("No 'n9mem://' asset source configured.");
            }
        });
        // app.add_supported_script_extensions(&[".p8"], Language::Lua);
        app.add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                let pico8_asset: Handle<Pico8Asset> =
                    asset_server.load(&asset_path);
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            },
        );
        #[cfg(feature = "scripting")]
        {
            app.insert_resource(ScriptContext::<LuaScriptingPlugin>::shared());
            let mut lua_scripting_plugin = LuaScriptingPlugin::default();
            lua_scripting_plugin
                .scripting_plugin
                .add_context_initializer(
                    |_context_key: &ContextKey, context: &mut bevy_mod_scripting::lua::mlua::Lua| {
                        context.globals().set(
                            "_eval_string",
                            context.create_function(|ctx, arg: String| {
                                ctx.load(format!("tostring({arg})")).eval::<String>()
                            })?,
                        )?;
                        context
                            .load(include_str!("builtin.lua"))
                            .exec()
                            .expect("Problem in builtin.lua");
                        Ok(())
                    },
                );

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

            app.add_plugins(BMSPlugin.set(lua_scripting_plugin));
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
        .insert_resource(
            self.config
                .defaults
                .as_ref()
                .map(pico8::Defaults::from_config)
                .unwrap_or_default(),
        )
        .insert_resource(N9Canvas {
            size: canvas_size,
            ..default()
        })
        .add_plugins(crate::plugin);

        if let Some(fps) = self.config.frames_per_second {
            info!("Set FPS {}", &fps);
            let limiter = bevy_framepace::Limiter::from_framerate(fps as f64);
            app.add_plugins(bevy_framepace::FramepacePlugin)
               .insert_resource(bevy_framepace::FramepaceSettings::default().with_limiter(limiter));
            // app.insert_resource(Time::<Fixed>::from_seconds(
            //     1.0 / fps as f64,
            // ));
        }

        #[cfg(feature = "scripting")]
        app.add_plugins(add_logging);
        #[cfg(feature = "scripting")]
        app.add_systems(
            Update,
            (
                fill_input,
                send(call::Init).run_if(init_when::<ScriptAsset>()),
                event_handler::<call::Init, LuaScriptingPlugin>,
                send(call::Update).run_if(in_state(RunState::Run)),
                event_handler::<call::Update, LuaScriptingPlugin>,
                event_handler::<call::Eval, LuaScriptingPlugin>,
                send(call::Draw).run_if(in_state(RunState::Run)),
                event_handler::<call::Draw, LuaScriptingPlugin>,
            )
                .chain(),
        );

        #[cfg(not(feature = "scripting"))]
        app.add_systems(Update, fill_input);
        // bevy_ecs_ldtk will add this plugin, so let's not add that if it's
        // present.
        #[cfg(not(feature = "level"))]
        app.add_plugins(bevy_ecs_tilemap::TilemapPlugin);

    }
}

pub fn init_when<T: Asset>(
) -> impl FnMut(EventReader<AssetEvent<T>>, Local<bool>, Res<State<RunState>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>,
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

pub fn on_asset_change<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
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

pub fn on_asset_loaded<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
        reader
            .read()
            .any(|e| matches!(e, AssetEvent::LoadedWithDependencies { .. }))
    }
}

pub fn on_asset_modified<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) -> bool + Clone {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
        reader
            .read()
            .any(|e| matches!(e, AssetEvent::Modified { .. }))
    }
}

pub fn info_on_asset_event<T: Asset>() -> impl FnMut(EventReader<AssetEvent<T>>) {
    // The events need to be consumed, so that there are no false positives on subsequent
    // calls of the run condition. Simply checking `is_empty` would not be enough.
    // PERF: note that `count` is efficient (not actually looping/iterating),
    // due to Bevy having a specialized implementation for events.
    move |mut reader: EventReader<AssetEvent<T>>| {
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
