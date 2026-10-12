use super::*;
use crate::translate::Position;
use crate::{
    CanvasRenderTarget, Headless,
    headless::{TEXT2D_LAYOUT_SCALE, setup_render_target},
};
#[cfg(target_arch = "wasm32")]
use bevy::render::view::Msaa;
use bevy::{
    camera::{
        visibility::RenderLayers, CameraOutputMode, ClearColorConfig, CompositingSpace,
        ImageRenderTarget, RenderTarget, Viewport,
    },
    render::render_resource::BlendState,
    window::{PrimaryWindow, WindowResized},
};

#[derive(Debug, Clone, Resource, Reflect)]
pub struct N9Canvas {
    pub size: UVec2,
    pub bit_depth: u8,
}

impl Default for N9Canvas {
    fn default() -> Self {
        Self {
            size: UVec2::splat(128),
            bit_depth: 4,
        }
    }
}

/// Screen-sized Gfx written by `pset`. Spawned on demand as a hashless Clearable.
#[derive(Component, Debug, Reflect)]
pub struct PixelCanvas;

const MAX_PIXEL_CANVASES: usize = 8;

#[derive(Component, Debug, Reflect)]
pub struct OneColorBackground;

bobtail::define! {
    #[doc(hidden)]
    pub __cls => fn cls(&mut self, #[tail] color: Option<PColor>) -> Result<(), Error>;
    #[doc(hidden)]
    pub __pget => fn pget(&self, x: f32, y: f32) -> Option<usize>;
    #[doc(hidden)]
    pub __pset => fn pset(&mut self, pos: UVec2, #[tail] color: Option<PColor>) -> Result<(), Error>;
}
pub use __cls as cls;
pub use __pget as pget;
pub use __pset as pset;

pub(crate) fn plugin(app: &mut App) {
    // app.register_type::<OneColorBackground>()
    //     .register_type::<Background>()
    //     .register_type::<N9Canvas>()
    //     // .add_systems(PreStartup, (spawn_camera, setup_canvas).chain())
    //     ;

    app.add_systems(PreUpdate, (spawn_camera, setup_canvas).chain());
    if app.is_plugin_added::<WindowPlugin>() {
        app.add_systems(Update, sync_window_size);
    }
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

pub fn setup_canvas(
    canvas: Option<Res<N9Canvas>>,
    mut assets: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Nano9Camera>>,
    mut commands: Commands,
) {
    let Some(canvas) = canvas else {
        return;
    };
    if canvas.is_added() {
        trace!("setup canvas");
        let camera_id = camera.into_inner();

        let mut image = Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0xffu8, 0xffu8, 0xffu8, 0xffu8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        commands
            .spawn((
                Name::new("1x1 canvas"),
                Sprite {
                    image: assets.add(image),
                    color: Color::BLACK,
                    custom_size: Some(canvas.size.as_vec2()),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, -101.0),
                OneColorBackground,
            ))
            .insert(ChildOf(camera_id));
    } else if canvas.is_changed() {
        trace!("sync canvas");
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy)]
struct Dolly;

fn spawn_camera(
    mut commands: Commands,
    canvas: Option<Res<N9Canvas>>,
    headless: Option<Res<Headless>>,
    canvas_target: Option<Res<CanvasRenderTarget>>,
    mut images: ResMut<Assets<Image>>,
    mut dolly: Query<&mut Transform, With<Dolly>>,
) {
    let Some(canvas) = canvas else {
        return;
    };
    if canvas_target.is_some() {
        if canvas.is_changed()
            && let Ok(mut transform) = dolly.single_mut()
        {
            *transform = Transform::from_xyz(
                canvas.size.x as f32 / 2.0,
                -(canvas.size.y as f32) / 2.0,
                0.0,
            );
        }
        return;
    }
    if canvas.is_added() {
        let image_target = headless.is_some().then(|| {
            let physical = (canvas.size.as_vec2() * TEXT2D_LAYOUT_SCALE)
                .round()
                .as_uvec2();
            let handle = setup_render_target(&mut images, physical);
            commands.insert_resource(CanvasRenderTarget {
                handle: handle.clone(),
                scale_factor: TEXT2D_LAYOUT_SCALE,
            });
            ImageRenderTarget {
                handle,
                scale_factor: TEXT2D_LAYOUT_SCALE,
            }
        });
        commands
            .spawn((
                Name::new("dolly"),
                Transform::from_xyz(
                    canvas.size.x as f32 / 2.0,
                    -(canvas.size.y as f32) / 2.0,
                    0.0,
                ),
                Dolly,
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                let mut camera = parent.spawn((
                    Name::new("camera"),
                    Camera2d,
                    // Native: gamma-encoded writes, bit-exact with palette PNGs.
                    // Wasm/WebGL2 cannot create those sRGB view formats
                    // (`DownlevelFlags::VIEW_FORMATS`).
                    #[cfg(not(target_arch = "wasm32"))]
                    CompositingSpace::Srgb,
                    #[cfg(target_arch = "wasm32")]
                    CompositingSpace::Linear,
                    #[cfg(target_arch = "wasm32")]
                    Msaa::Off,
                    Projection::Orthographic(OrthographicProjection::default_2d()),
                    InheritedVisibility::default(),
                    Nano9Camera,
                    Position::default(),
                ));
                if let Some(target) = image_target {
                    camera.insert((
                        Camera {
                            order: 0,
                            ..default()
                        },
                        RenderTarget::Image(target),
                        // Headless carts have no separate window camera for UI.
                        IsDefaultUiCamera,
                    ));
                } else if cfg!(target_arch = "wasm32") {
                    // Wasm already composites in linear space, so UI can stay on this camera.
                    camera.insert(IsDefaultUiCamera);
                }
            });
        // The game camera composites in sRGB so palette pixels stay bit-exact.
        // Feathers colors are linear and turn nearly black on that camera, so UI
        // gets a later camera that encodes them on the way to the window.
        if headless.is_none() && cfg!(not(target_arch = "wasm32")) {
            commands.spawn((
                Name::new("ui camera"),
                Camera2d,
                Camera {
                    order: 1,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    output_mode: CameraOutputMode::Write {
                        blend_state: Some(BlendState::ALPHA_BLENDING),
                        clear_color: ClearColorConfig::None,
                    },
                    ..default()
                },
                RenderLayers::none(),
                IsDefaultUiCamera,
            ));
        }
    } else if canvas.is_changed()
        && let Ok(mut transform) = dolly.single_mut()
    {
        *transform = Transform::from_xyz(
            canvas.size.x as f32 / 2.0,
            -(canvas.size.y as f32) / 2.0,
            0.0,
        );
    }
}

pub fn sync_window_size(
    mut resize_event: MessageReader<WindowResized>,
    canvas: Option<Res<N9Canvas>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    mut projection_query: Query<&mut Projection, With<Nano9Camera>>,
    mut camera_query: Query<&mut Camera, With<Nano9Camera>>,
    mut pending: Local<bool>,
) {
    if resize_event
        .read()
        .any(|e| primary_windows.get(e.window).is_ok())
    {
        *pending = true;
    }
    if !*pending {
        return;
    }
    // `WindowResized` expires after two frames; keep the latch until the cart's
    // canvas (and camera) exist so late-loading assets still get a scale.
    let Some(canvas) = canvas else {
        return;
    };
    if camera_query.is_empty() {
        return;
    }
    let Ok(primary_window) = primary_windows.single() else {
        return;
    };

    let window_scale = primary_window.scale_factor();
    let window_size = Vec2::new(
        primary_window.physical_width() as f32,
        primary_window.physical_height() as f32,
    ) / window_scale;

    let canvas_size = canvas.size.as_vec2();
    // `new_scale` is the number of logical window pixels per canvas pixel.
    let new_scale = (window_size.y / canvas_size.y).min(window_size.x / canvas_size.x);

    for mut projection in projection_query.iter_mut() {
        match &mut *projection {
            Projection::Orthographic(orthographic) => {
                trace!(
                    "oldscale {} new_scale {new_scale} window_scale {window_scale}",
                    &orthographic.scale
                );
                orthographic.scale = 1.0 / new_scale;
            }
            x => warn_once!("Nano9Camera is not an orthographic camera: {:?}", x),
        }
    }

    let viewport_size = canvas_size * new_scale * window_scale;
    let start = (window_size * window_scale - viewport_size) / 2.0;
    trace!("viewport size {} start {}", &viewport_size, &start);

    for mut camera in camera_query.iter_mut() {
        camera.viewport = Some(Viewport {
            physical_position: UVec2::new(start.x as u32, start.y as u32),
            physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
            ..default()
        });
    }
    *pending = false;
}

/// Floor a Pico-8 coordinate. Non-finite or out-of-`i32` values are out of range.
fn floor_pixel(x: f32, y: f32) -> Option<(u32, u32)> {
    let x = floor_coord(x)?;
    let y = floor_coord(y)?;
    if x < 0 || y < 0 {
        return None;
    }
    Some((x as u32, y as u32))
}

fn floor_coord(value: f32) -> Option<i32> {
    if !value.is_finite() {
        return None;
    }
    let floored = value.floor();
    if floored < i32::MIN as f32 || floored > i32::MAX as f32 {
        return None;
    }
    Some(floored as i32)
}

impl super::Pico8<'_, '_> {
    // cls([n])
    pub fn cls(&mut self, color: Option<PColor>) -> Result<(), Error> {
        // trace!("cls");
        let c = color.unwrap_or(PColor::Palette(self.defaults.clear_color));
        // let image = self
        //     .images
        //     .get_mut(&self.canvas.handle)
        //     .ok_or(Error::NoAsset("canvas".into()))?;
        // for i in 0..image.width() {
        //     for j in 0..image.height() {
        //         image.set_color_at(i, j, c)?;
        //     }
        // }
        self.commands
            .run_system_cached_with(crate::pico8::clear::clear_screen, c);
        Ok(())
    }

    pub fn pset(&mut self, pos: UVec2, color: Option<PColor>) -> Result<(), Error> {
        match color.unwrap_or(self.state.draw_state.pen) {
            PColor::Palette(p) => {
                let p = self.draw_pal_map().map_or_mod(p) as u8;
                let gfx_handle = self.ensure_pixel_canvas_gfx()?;
                let mut gfx = self
                    .gfxs
                    .get_mut(&gfx_handle)
                    .ok_or(Error::NoAsset("gfx".into()))?;
                if gfx.set(pos.x as usize, pos.y as usize, p) {
                    Ok(())
                } else {
                    Err(Error::InvalidArgument(
                        format!(
                            "Could not set gfx color {} at ({:.1}, {:.1}).",
                            p, pos.x, pos.y
                        )
                        .into(),
                    ))
                }
            }
            _ => {
                todo!("Write an RGBA color.")
                // let c = self.get_color(color.into())?;
                // let image = self
                //     .images
                //     .get_mut(&self.canvas.handle)
                //     .ok_or(Error::NoAsset("canvas".into()))?;
                // image.set_color_at(pos.x, pos.y, c)?;
                // Ok(())
            }
        }
    }

    /// Write target for `pset`: the latest visible PixelCanvas if it is still
    /// the last Clearable issued, otherwise a resurrected or newly spawned one.
    fn ensure_pixel_canvas_gfx(&mut self) -> Result<Handle<Gfx>, Error> {
        let mut latest_visible: Option<(Entity, usize)> = None;
        let mut hidden: Option<Entity> = None;
        let mut count = 0usize;
        for (entity, clearable, _) in self.pixel_canvases.iter() {
            count += 1;
            match clearable.state {
                ClearState::Visible => {
                    if latest_visible.is_none_or(|(_, dc)| clearable.draw_count >= dc) {
                        latest_visible = Some((entity, clearable.draw_count));
                    }
                }
                ClearState::Hidden { .. } => {
                    hidden = Some(entity);
                }
            }
        }

        let needs_new = match latest_visible {
            Some((_, draw_count)) => draw_counter() != draw_count + 1,
            None => true,
        };

        if !needs_new {
            let entity = latest_visible.expect("visible pixel canvas").0;
            return self.pixel_canvas_gfx(entity);
        }

        if let Some(entity) = hidden {
            if let Ok((_, mut clearable, mut visibility)) = self.pixel_canvases.get_mut(entity) {
                clearable.resurrect();
                *visibility = Visibility::Inherited;
            }
            let handle = self.pixel_canvas_gfx(entity)?;
            if let Some(mut gfx) = self.gfxs.get_mut(&handle) {
                gfx.clear_canvas();
            }
            return Ok(handle);
        }

        if count >= MAX_PIXEL_CANVASES {
            warn!("pixel canvas cap ({MAX_PIXEL_CANVASES}) reached; writing existing canvas");
            if let Some((entity, _)) = latest_visible {
                return self.pixel_canvas_gfx(entity);
            }
            return Err(Error::NoAsset("pixel canvas".into()));
        }

        let camera = self
            .n9_cameras
            .single()
            .map_err(|_| Error::NoSuch("camera".into()))?;
        let gfx = Gfx::new_pixel_canvas(
            self.canvas.bit_depth as usize,
            self.canvas.size.x as usize,
            self.canvas.size.y as usize,
        );
        let gfx_handle = self.gfxs.add(gfx);
        let material = self.gfx_material();
        let clearable = Clearable::new(self.defaults.time_to_live);
        self.commands.spawn((
            Name::new("pixel canvas"),
            PixelCanvas,
            GfxSprite {
                image: gfx_handle.clone(),
                material,
                pal_map_lut: self.state.pal_map_lut.clone(),
            },
            GfxDirty::default(),
            Position::default(),
            clearable,
            Visibility::Inherited,
            ChildOf(camera),
        ));
        Ok(gfx_handle)
    }

    fn pixel_canvas_gfx(&self, entity: Entity) -> Result<Handle<Gfx>, Error> {
        self.gfx_sprites
            .get(entity)
            .map(|sprite| sprite.image.clone())
            .map_err(|_| Error::NoAsset("pixel canvas gfx".into()))
    }

    /// Color index at a screen pixel, or `None` when no canvas has written it.
    ///
    /// Walks visible pixel canvases from the top down and returns the first
    /// occupied pixel. An unset pixel is transparent, so a hole in a newer
    /// canvas shows an older one. Out of range is `None`. `pset` of color 0 is
    /// occupied. Sprites, rects, maps, and print are not in these buffers.
    pub fn pget(&self, x: f32, y: f32) -> Option<usize> {
        let (x, y) = floor_pixel(x, y)?;
        if x >= self.canvas.size.x || y >= self.canvas.size.y {
            return None;
        }
        let x = x as usize;
        let y = y as usize;
        let mut canvases = Vec::new();
        for (entity, clearable, _) in self.pixel_canvases.iter() {
            if matches!(clearable.state, ClearState::Visible) {
                canvases.push((clearable.draw_count, entity));
            }
        }
        canvases.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        for (_, entity) in canvases {
            let Ok(sprite) = self.gfx_sprites.get(entity) else {
                continue;
            };
            let Some(gfx) = self.gfxs.get(&sprite.image) else {
                continue;
            };
            if gfx.occupied(x, y)
                && let Some(color) = gfx.get(x, y)
            {
                return Some(color as usize);
            }
        }
        None
    }

    /// Return the size of the canvas
    ///
    /// This is not the window dimensions, which are physical pixels. Instead it
    /// is the number of "logical" pixels, which may be comprised of many
    /// physical pixels.
    pub fn canvas_size(&self) -> UVec2 {
        self.canvas.size
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register("cls", |ctx: FunctionCallContext, c: Option<PColor>| {
                with_pico8(&ctx, |pico8| pico8.cls(c))
            })
            .register("pget", |ctx: FunctionCallContext, x: f32, y: f32| {
                with_pico8(&ctx, |pico8| Ok(pico8.pget(x, y).unwrap_or(0)))
            })
            .register(
                "pset",
                |ctx: FunctionCallContext, x: u32, y: u32, color: Option<PColor>| {
                    with_pico8(&ctx, |pico8| {
                        // We want to ignore out of bounds errors specifically but possibly not others.
                        // Ok(pico8.pset(x, y, color)?)
                        let _ = pico8.pset(UVec2::new(x, y), color);
                        Ok(())
                    })
                },
            );
    }
}
