#![allow(deprecated)]
use bevy::{
    image::ImageSampler,
    prelude::*,
    reflect::Reflect,
    render::{
        camera::{ScalingMode, Viewport},
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    utils::Duration,
    window::{PresentMode, PrimaryWindow, WindowMode, WindowResized},
};

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    core::{
        asset::{Language, ScriptAsset, ScriptAssetSettings},
        bindings::{function::namespace::NamespaceBuilder, script_value::ScriptValue},
        callback_labels,
        event::ScriptCallbackEvent,
        handler::event_handler,
        ConfigureScriptPlugin,
    },
    lua::LuaScriptingPlugin,
    BMSPlugin,
};

use crate::{
    config::*,
    error::RunState,
    pico8::{self, input::fill_input, FillPat, Pico8Asset, Pico8Handle},
    PColor,
};

#[cfg(feature = "scripting")]
use crate::N9Var;

#[derive(Component)]
pub struct Nano9Sprite;

#[derive(Clone, Debug, Reflect)]
pub struct DrawState {
    pub pen: PColor,
    pub camera_position: Vec2,
    pub camera_position_delta: Option<Vec2>,
    pub print_cursor: Vec2,
    pub fill_pat: Option<FillPat>,
}

impl DrawState {
    /// Mark ourselves as having drawn something this frame.
    pub fn mark_drawn(&mut self) {
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
    }
}

#[derive(Debug, Clone, Resource, Default)]
pub struct N9Canvas {
    pub size: UVec2,
    pub handle: Handle<Image>,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState {
            // XXX: Pico-8 should be 6 here, but that's not true in general.
            pen: PColor::Palette(1),
            camera_position: Vec2::ZERO,
            print_cursor: Vec2::ZERO,
            camera_position_delta: None,
            fill_pat: None,
        }
    }
}

// fn reset_camera_delta(mut events: EventReader<ClearEvent>, mut state: ResMut<Pico8State>) {
//     for _ in events.read() {
//         // info!("reset camera delta");
//         state.draw_state.camera_position_delta = None;
//     }
// }

pub fn setup_canvas(mut canvas: Option<ResMut<N9Canvas>>, mut assets: ResMut<Assets<Image>>) {
    trace!("setup_canvas");
    if let Some(ref mut canvas) = canvas {
        let mut image = Image::new_fill(
            Extent3d {
                width: canvas.size.x,
                height: canvas.size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0u8, 0u8, 0u8, 0u8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        canvas.handle = assets.add(image);
    }
}

#[cfg(feature = "scripting")]
pub mod call {
    use super::*;
    callback_labels!(
    SetGlobal => "_set_global",
    Update => "_update",
    // Update60 => "_update60",
    Init => "_init",
    Eval => "_eval",
    Draw => "_draw"); // TODO: Should permit trailing comma
}

// pub fn set_camera(
//     camera: Query<Entity, With<Camera>>,
//     mut events: PriorityEventWriter<LuaEvent<N9Args>>,
// ) {
//     if let Ok(id) = camera.get_single() {
//         events.send(
//             LuaEvent {
//                 hook_name: "_set_global".to_owned(),
//                 args: {
//                     let mut args = Variadic::new();
//                     args.push(N9Arg::String("camera".into()));
//                     // args.push(N9Arg::Entity(id));
//                     args.push(N9Arg::Camera(N9Camera(id)));
//                     args
//                 },
//                 recipients: Recipients::All,
//             },
//             0,
//         )
//     }
// }

#[derive(Component, Debug, Reflect)]
pub struct Nano9Camera;

fn spawn_camera(mut commands: Commands, canvas: Option<Res<N9Canvas>>) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::WindowSize;
    let handle = canvas.as_ref().map(|c| c.handle.clone());
    let canvas_size: UVec2 = canvas.map(|c| c.size).unwrap_or_default();
    commands
        .spawn((
            Name::new("dolly"),
            Transform::from_xyz(
                canvas_size.x as f32 / 2.0,
                -(canvas_size.y as f32) / 2.0,
                0.0,
            ),
            InheritedVisibility::default(),
        ))
        .with_children(|parent| {
            let mut camera_commands = parent.spawn((
                Name::new("camera"),
                Camera2d,
                Msaa::Off,
                projection,
                IsDefaultUiCamera,
                InheritedVisibility::default(),
                Nano9Camera,
                #[cfg(feature = "scripting")]
                N9Var::new("camera"),
            ));
            if let Some(handle) = handle {
                camera_commands.with_children(|parent| {
                    parent.spawn((
                        Name::new("canvas"),
                        Sprite::from_image(handle),
                        Transform::from_xyz(0.0, 0.0, -100.0),
                        Nano9Sprite,
                        #[cfg(feature = "scripting")]
                        N9Var::new("canvas"),
                    ));
                });
            }
        });
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

pub fn sync_window_size(
    mut resize_event: EventReader<WindowResized>,
    canvas: Res<N9Canvas>,
    // mut query: Query<&mut Sprite, With<Nano9Sprite>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    orthographic_camera: Single<(&mut OrthographicProjection, &mut Camera), With<Nano9Camera>>,
) {
    if let Some(e) = resize_event
        .read()
        .filter(|e| primary_windows.get(e.window).is_ok())
        .last()
    {
        let primary_window = primary_windows.get(e.window).unwrap();

        //let window_size = primary_window.physical_size().as_vec2();
        let window_scale = primary_window.scale_factor();
        let window_size = Vec2::new(
            primary_window.physical_width() as f32,
            primary_window.physical_height() as f32,
        ) / window_scale;
        // let mut orthographic = orthographic.single_mut();

        let canvas_size = canvas.size.as_vec2();
        // let canvas_aspect = canvas_size.x / canvas_size.y;
        // let window_aspect = window_size.x / window_size.y;

        // `new_scale` is the number of physical pixels per logical pixels.
        let new_scale =
                // Canvas is longer than it is tall. Fit the width first.
                (window_size.y / canvas_size.y).min(window_size.x / canvas_size.x);
        // info!("window_size {window_size}");

        let (mut orthographic, mut camera) = orthographic_camera.into_inner();
        // match *orthographic.into_inner() {
        //     Projection::Orthographic(ref mut orthographic) => {

        info!(
            "oldscale {} new_scale {new_scale} window_scale {window_scale}",
            &orthographic.scale
        );
        orthographic.scale = 1.0 / new_scale;
        let viewport_size = canvas_size * new_scale * window_scale;
        let start = (window_size * window_scale - viewport_size) / 2.0;
        info!("viewport size {} start {}", &viewport_size, &start);
        camera.viewport = Some(Viewport {
            physical_position: UVec2::new(start.x as u32, start.y as u32),
            physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
            ..default()
        });

        // }
        //     _ => { panic!("Not expecting a perspective"); }

        // }
        // settings.pixel_scale = new_scale;
        // orthographic.scaling_mode = ScalingMode::WindowSize;
        // }
        // transform.scale = Vec3::splat(new_scale);

        // let scale = if settings.canvas_size.x > settings.canvas_size.y
        // {
        //     // horizontal is longer
        //     settings.resolution.1 as f32
        //         / settings.canvas_size.y as f32
        // } else {
        //     // vertical is longer
        //     settings.resolution.0 as f32
        //         / settings.canvas_size.x as f32
        // };

        //     sprite.custom_size = Some(Vec2::new(
        //         (settings.canvas_size.x as f32) * scale,
        //         (settings.canvas_size.y as f32) * scale,
        //     ));
    }
}

#[cfg(feature = "scripting")]
/// Sends events allowing scripts to drive update logic
pub fn send_update(mut writer: EventWriter<ScriptCallbackEvent>) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Update,
        vec![ScriptValue::Unit],
    ));
}

#[cfg(feature = "scripting")]
/// Sends initialization event
pub fn send_init(mut writer: EventWriter<ScriptCallbackEvent>) {
    info!("calling init");
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Init,
        vec![ScriptValue::Unit],
    ));
}

#[cfg(feature = "scripting")]
/// Sends draw event
pub fn send_draw(mut writer: EventWriter<ScriptCallbackEvent>) {
    writer.send(ScriptCallbackEvent::new_for_all(
        call::Draw,
        vec![ScriptValue::Unit],
    ));
}
const DEFAULT_FRAMES_PER_SECOND: u8 = 60;

#[derive(Default)]
pub struct Nano9Plugin {
    pub config: Config,
}

impl Nano9Plugin {
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
fn add_lua_logging(app: &mut App) {
    let world = app.world_mut();
    NamespaceBuilder::<World>::new_unregistered(world)
        .register("info", |s: String| {
            bevy::log::info!(s);
        })
        .register("warn", |s: String| {
            bevy::log::warn!(s);
        })
        .register("error", |s: String| {
            bevy::log::error!(s);
        })
        .register("debug", |s: String| {
            bevy::log::debug!(s);
        });
}

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

        {
            // Make our config readable by the Bevy AssetServer.
            //
            // I kind of hate this because we have to serialize just to
            // deserialize.
            let config_string = toml::to_string(&self.config).unwrap();
            if let Some(memory_dir) = app.world_mut().get_resource_mut::<MemoryDir>() {
                memory_dir.insert_asset(
                    std::path::Path::new("Nano9.toml"),
                    config_string.into_bytes(),
                );
                app.add_systems(
                    Startup,
                    |asset_server: Res<AssetServer>, mut commands: Commands| {
                        let pico8_asset: Handle<Pico8Asset> =
                            asset_server.load("n9mem://Nano9.toml");
                        commands.insert_resource(Pico8Handle::from(pico8_asset));
                    },
                );
            } else {
                warn!("No 'n9mem://' asset source configured.");
            }
        }

        #[cfg(feature = "scripting")]
        {
            let mut lua_scripting_plugin = LuaScriptingPlugin::default().enable_context_sharing();
            lua_scripting_plugin
                .scripting_plugin
                .add_context_initializer(
                    |_script_id: &str, context: &mut bevy_mod_scripting::lua::mlua::Lua| {
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

            app.add_plugins(BMSPlugin.set(lua_scripting_plugin))
                .insert_resource({
                    let mut settings = ScriptAssetSettings::default();
                    // settings
                    //     .extension_to_language_map
                    //     .insert("p8#lua", Language::Lua);
                    settings
                        .extension_to_language_map
                        .insert("p8", Language::Lua);

                    settings
                        .extension_to_language_map
                        .insert("p8lua", Language::Lua);

                    // settings
                    //     .extension_to_language_map
                    //     .insert("png#lua", Language::Lua);
                    settings
                        .extension_to_language_map
                        .insert("png", Language::Lua);
                    // settings.script_id_mapper = AssetPathToScriptIdMapper {
                    //     map: (|path: &AssetPath| path.to_string().into()),
                    // };
                    settings
                });
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
        // Insert the config as a resource.
        // TODO: Should we constrain it, if it wasn't provided as an option?
        .insert_resource(Time::<Fixed>::from_seconds(
            1.0 / self
                .config
                .frames_per_second
                .unwrap_or(DEFAULT_FRAMES_PER_SECOND) as f64,
        ))
        .insert_resource(N9Canvas {
            size: canvas_size,
            ..default()
        })
        .add_plugins(crate::plugin)
        .add_systems(PreStartup, (setup_canvas, spawn_camera).chain());

        #[cfg(feature = "scripting")]
        app.add_plugins(add_lua_logging);
        #[cfg(feature = "scripting")]
        app.add_systems(
            Update,
            (
                fill_input,
                send_init.run_if(init_when::<ScriptAsset>()),
                event_handler::<call::Init, LuaScriptingPlugin>,
                send_update.run_if(in_state(RunState::Run)),
                event_handler::<call::Update, LuaScriptingPlugin>,
                event_handler::<call::Eval, LuaScriptingPlugin>,
                send_draw.run_if(in_state(RunState::Run)),
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

        if app.is_plugin_added::<WindowPlugin>() {
            app.add_systems(Update, sync_window_size)
                .add_systems(Update, fullscreen_key);
        }
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
