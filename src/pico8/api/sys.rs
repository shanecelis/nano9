use super::canvas::N9Canvas;
use super::*;
use crate::run::RunState;
use crate::{CanvasRenderTarget, Headless};
use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::PrimaryWindow;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CartArgs>()
        .init_resource::<ReadyQueue>()
        .add_message::<ExtcmdRequest>()
        .add_message::<LoadCartRequest>()
        .add_observer(queue_startup_window)
        .add_systems(Last, pump_ready_queue)
        .add_systems(Update, handle_load_cart);
    if app.world().contains_resource::<Headless>() {
        app.add_systems(Startup, queue_startup_headless);
    }
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

/// Parameter string and breadcrumb from `load()` or the CLI `-p` flag.
///
/// Pico-8 `stat(6)` is the parameter string; `stat(100)` is the breadcrumb.
#[derive(Resource, Clone, Debug, Default, Reflect)]
#[reflect(Resource)]
pub struct CartArgs {
    /// Arbitrary string from `load(..., param)` or `-p`. Pico-8 `stat(6)`.
    pub param: String,
    /// Breadcrumb from `load(filename, breadcrumb, ...)`. Pico-8 `stat(100)`.
    pub breadcrumb: String,
}

/// Pico-8 `load(filename)` — swap in another cart.
#[derive(Message, Clone, Debug)]
pub(crate) struct LoadCartRequest {
    pub filename: String,
}

/// Sequential `extcmd` work processed one item at a time in the last-frame pump.
#[derive(Message, Clone, Debug)]
pub enum ExtcmdRequest {
    WaitTilReady,
    RevealWindow(Entity),
    StartScreenshot(PathBuf),
    WaitForScreenshot(Arc<Mutex<bool>>),
    Shutdown,
}

#[derive(Resource, Default)]
struct ReadyQueue(VecDeque<ExtcmdRequest>);

fn queue_startup_window(
    add: On<Add, PrimaryWindow>,
    mut writer: MessageWriter<ExtcmdRequest>,
    mut commands: Commands,
) {
    writer.write(ExtcmdRequest::WaitTilReady);
    writer.write(ExtcmdRequest::RevealWindow(add.entity));
    commands.entity(add.observer()).despawn();
}

fn queue_startup_headless(mut writer: MessageWriter<ExtcmdRequest>) {
    writer.write(ExtcmdRequest::WaitTilReady);
}

fn handle_load_cart(
    mut reader: MessageReader<LoadCartRequest>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<RunState>>,
    #[cfg(feature = "scripting")] scripts: Query<
        Entity,
        With<bevy_mod_scripting::core::script::ScriptComponent>,
    >,
) {
    let Some(req) = reader.read().last().cloned() else {
        return;
    };
    #[cfg(feature = "scripting")]
    for entity in &scripts {
        commands.entity(entity).despawn();
    }
    let handle: Handle<Pico8Asset> = asset_server.load(req.filename.clone());
    commands.insert_resource(Pico8Handle::from(handle));
    next_state.set(RunState::Uninit);
}

fn pump_ready_queue(
    mut incoming: MessageReader<ExtcmdRequest>,
    mut queue: ResMut<ReadyQueue>,
    mut current: Local<Option<ExtcmdRequest>>,
    run_state: Res<State<RunState>>,
    mut windows: Query<&mut Window>,
    mut commands: Commands,
    canvas: Option<Res<N9Canvas>>,
    headless: Option<Res<Headless>>,
    canvas_target: Option<Res<CanvasRenderTarget>>,
) {
    queue.0.extend(incoming.read().cloned());
    loop {
        if current.is_none() {
            *current = queue.0.pop_front();
        }
        let Some(item) = current.clone() else {
            return;
        };
        let done = match item {
            ExtcmdRequest::WaitTilReady => matches!(**run_state, RunState::Run | RunState::Pause),
            ExtcmdRequest::RevealWindow(entity) => match windows.get_mut(entity) {
                Ok(mut window) => {
                    if window.visible {
                        true
                    } else {
                        window.visible = true;
                        false
                    }
                }
                Err(_) => {
                    warn!("RevealWindow({entity}) has no Window; skipping");
                    true
                }
            },
            ExtcmdRequest::StartScreenshot(path) => match canvas.as_ref() {
                None => false,
                Some(canvas) => {
                    let image_target = canvas_target.as_ref().map(|t| ImageRenderTarget {
                        handle: t.handle.clone(),
                        scale_factor: t.scale_factor,
                    });
                    if headless.is_some() {
                        if image_target.is_none() {
                            false
                        } else {
                            let written = Arc::new(Mutex::new(false));
                            start_queued_screenshot(
                                &mut commands,
                                path,
                                canvas.size,
                                written.clone(),
                                image_target,
                            );
                            *current = Some(ExtcmdRequest::WaitForScreenshot(written));
                            false
                        }
                    } else if !windows.iter().any(|window| window.visible) {
                        false
                    } else {
                        let written = Arc::new(Mutex::new(false));
                        start_queued_screenshot(
                            &mut commands,
                            path,
                            canvas.size,
                            written.clone(),
                            None,
                        );
                        *current = Some(ExtcmdRequest::WaitForScreenshot(written));
                        false
                    }
                }
            },
            ExtcmdRequest::WaitForScreenshot(written) => {
                written.lock().map(|g| *g).unwrap_or(false)
            }
            ExtcmdRequest::Shutdown => {
                commands.write_message(AppExit::Success);
                true
            }
        };
        if !done {
            return;
        }
        *current = None;
    }
}

/// Spawn Bevy's screenshot and set `written` after the PNG is on disk.
fn start_queued_screenshot(
    commands: &mut Commands,
    path: PathBuf,
    canvas_size: UVec2,
    written: Arc<Mutex<bool>>,
    image_target: Option<ImageRenderTarget>,
) {
    let screenshot = match image_target {
        Some(target) => Screenshot(RenderTarget::Image(target)),
        None => Screenshot::primary_window(),
    };
    commands.spawn(screenshot).observe(
        move |captured: On<ScreenshotCaptured>, cameras: Query<&Camera, With<Nano9Camera>>| {
            if let Err(e) = save_captured_screenshot(&captured.image, cameras, canvas_size, &path) {
                error!("extcmd(\"screen\") failed: {e}");
            } else {
                info!("Screenshot saved to {}", path.display());
            }
            if let Ok(mut done) = written.lock() {
                *done = true;
            }
        },
    );
}

impl super::Pico8<'_, '_> {
    pub fn time(&self) -> f32 {
        self.time.elapsed_secs()
    }

    pub fn delta_time(&self) -> f32 {
        self.time.delta_secs()
    }

    pub fn exit(&mut self, error: Option<u8>) {
        self.commands
            .write_message(match error.and_then(std::num::NonZero::new) {
                Some(n) => AppExit::Error(n),
                None => AppExit::Success,
            });
    }

    /// Pico-8 `extcmd(cmd, [p1], [p2])`.
    ///
    /// Supported: `set_filename`, `screen`, `audio_rec`, `audio_end`, `shutdown`.
    pub fn extcmd(&mut self, cmd: &str, p1: Option<&str>, p2: Option<f32>) -> Result<(), Error> {
        match cmd {
            "set_filename" => {
                if let Some(name) = p1 {
                    self.state.screenshot_filename = Some(name.to_string());
                }
                Ok(())
            }
            "screen" => {
                let _scale = p1.and_then(|s| s.parse::<f32>().ok()).or(p2);
                let _save_to_folder = p2;
                let path = self.capture_path("png");
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                self.commands
                    .write_message(ExtcmdRequest::StartScreenshot(path));
                Ok(())
            }
            "audio_rec" => {
                self.commands.queue(|world: &mut World| {
                    let frame = world
                        .get_resource::<bevy::diagnostic::FrameCount>()
                        .map(|f| f.0)
                        .unwrap_or(0);
                    if let Some(mut rec) = world.get_resource_mut::<crate::pico8::audio::AudioRecorder>()
                    {
                        rec.start(frame);
                    }
                });
                Ok(())
            }
            "audio_end" => {
                let path = self.capture_path("wav");
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                self.commands.queue(move |world: &mut World| {
                    let end_frame = world
                        .get_resource::<bevy::diagnostic::FrameCount>()
                        .map(|f| f.0)
                        .unwrap_or(0);
                    let Some(recording) = world
                        .get_resource_mut::<crate::pico8::audio::AudioRecorder>()
                        .and_then(|mut rec| rec.take())
                    else {
                        warn!("extcmd(\"audio_end\") with no active audio_rec");
                        return;
                    };
                    let pcm = recording.render(end_frame);
                    match crate::pico8::audio::write_wav(&path, &pcm) {
                        Ok(()) => info!("Audio saved to {}", path.display()),
                        Err(e) => error!("extcmd(\"audio_end\") failed: {e}"),
                    }
                });
                Ok(())
            }
            "shutdown" => {
                self.commands.queue(|world: &mut World| {
                    if let Some(mut next) = world.get_resource_mut::<NextState<RunState>>() {
                        next.set(RunState::Pause);
                    }
                });
                self.commands.write_message(ExtcmdRequest::Shutdown);
                Ok(())
            }
            other => {
                warn!("extcmd({other:?}) is not implemented");
                Ok(())
            }
        }
    }

    /// Pico-8 `load(filename, [breadcrumb], [param])`.
    ///
    /// Stores the breadcrumb (`stat(100)`) and parameter string (`stat(6)`), then
    /// queues a cart swap. A missing filename (Pico-8's load dialog) is not supported.
    pub fn load(
        &mut self,
        filename: Option<&str>,
        breadcrumb: Option<&str>,
        param: Option<&str>,
    ) -> Result<(), Error> {
        self.cart_args.breadcrumb = breadcrumb.unwrap_or("").to_string();
        self.cart_args.param = param.unwrap_or("").to_string();
        let Some(filename) = filename.filter(|name| !name.is_empty()) else {
            warn!("load() with no filename is not implemented (file picker)");
            return Ok(());
        };
        self.commands.write_message(LoadCartRequest {
            filename: filename.to_string(),
        });
        Ok(())
    }

    fn capture_path(&self, ext: &str) -> PathBuf {
        let stem = self
            .state
            .screenshot_filename
            .clone()
            .unwrap_or_else(|| "nano9".to_string());
        let mut path = std::env::var_os("NANO9_SCREENSHOT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        path.push(format!("{stem}.{ext}"));
        path
    }
}

fn save_captured_screenshot(
    image: &Image,
    cameras: Query<&Camera, With<Nano9Camera>>,
    canvas_size: UVec2,
    path: &Path,
) -> Result<(), String> {
    let src_size = image.size();
    let data = image
        .data
        .as_ref()
        .ok_or_else(|| "screenshot image has no CPU data".to_string())?;
    let bpp = 4usize;
    if data.len() < src_size.x as usize * src_size.y as usize * bpp {
        return Err(format!(
            "screenshot buffer too small: {} bytes for {}x{}",
            data.len(),
            src_size.x,
            src_size.y
        ));
    }
    let bgra = matches!(
        image.texture_descriptor.format,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
    );

    let crop = cameras
        .iter()
        .find_map(|camera| {
            camera.viewport.as_ref().map(|vp| {
                (
                    vp.physical_position.x.min(src_size.x.saturating_sub(1)),
                    vp.physical_position.y.min(src_size.y.saturating_sub(1)),
                    vp.physical_size.x.max(1).min(src_size.x),
                    vp.physical_size.y.max(1).min(src_size.y),
                )
            })
        })
        .unwrap_or((0, 0, src_size.x, src_size.y));

    let (cx, cy, cw, ch) = crop;
    let cw = cw.min(src_size.x.saturating_sub(cx)).max(1);
    let ch = ch.min(src_size.y.saturating_sub(cy)).max(1);
    let dst_w = canvas_size.x.max(1);
    let dst_h = canvas_size.y.max(1);

    let mut rgb = vec![0u8; dst_w as usize * dst_h as usize * 3];
    for y in 0..dst_h {
        let sy = cy + y * ch / dst_h;
        for x in 0..dst_w {
            let sx = cx + x * cw / dst_w;
            let si = ((sy * src_size.x + sx) as usize) * bpp;
            let di = ((y * dst_w + x) as usize) * 3;
            if bgra {
                rgb[di] = data[si + 2];
                rgb[di + 1] = data[si + 1];
                rgb[di + 2] = data[si];
            } else {
                rgb[di] = data[si];
                rgb[di + 1] = data[si + 1];
                rgb[di + 2] = data[si + 2];
            }
        }
    }
    write_rgb_png(path, dst_w, dst_h, &rgb)
}

fn write_rgb_png(path: &Path, width: u32, height: u32, rgb: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgb).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::bindings::ScriptValue;
    use bevy_mod_scripting::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };

    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();
        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register("exit", |ctx: FunctionCallContext, error: Option<u8>| {
                with_pico8(&ctx, move |pico8| {
                    pico8.exit(error);
                    Ok(())
                })
            })
            .register("time", |ctx: FunctionCallContext| {
                with_pico8(&ctx, move |pico8| Ok(pico8.time()))
            })
            .register("delta_time", |ctx: FunctionCallContext| {
                with_pico8(&ctx, move |pico8| Ok(pico8.delta_time()))
            })
            .register(
                "extcmd",
                |ctx: FunctionCallContext,
                 cmd: String,
                 p1: Option<ScriptValue>,
                 p2: Option<ScriptValue>| {
                    let p1s = p1.as_ref().and_then(script_value_to_string);
                    let p2n = p2.as_ref().and_then(script_value_to_f32);
                    with_pico8(&ctx, move |pico8| pico8.extcmd(&cmd, p1s.as_deref(), p2n))
                },
            )
            .register(
                "_n9_load",
                |ctx: FunctionCallContext,
                 filename: Option<String>,
                 breadcrumb: Option<ScriptValue>,
                 param: Option<ScriptValue>| {
                    let breadcrumb = breadcrumb.as_ref().and_then(script_value_to_string);
                    let param = param.as_ref().and_then(script_value_to_string);
                    with_pico8(&ctx, move |pico8| {
                        pico8.load(filename.as_deref(), breadcrumb.as_deref(), param.as_deref())
                    })
                },
            );
    }

    fn script_value_to_string(value: &ScriptValue) -> Option<String> {
        match value {
            ScriptValue::String(s) => Some(s.to_string()),
            ScriptValue::Integer(i) => Some(i.to_string()),
            ScriptValue::Float(f) => Some(f.to_string()),
            _ => None,
        }
    }

    fn script_value_to_f32(value: &ScriptValue) -> Option<f32> {
        match value {
            ScriptValue::Integer(i) => Some(*i as f32),
            ScriptValue::Float(f) => Some(*f as f32),
            ScriptValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }
}
