use super::*;
use crate::translate::Position;
use bevy::{
    camera::Viewport,
    window::{PrimaryWindow, WindowResized},
};

#[derive(Debug, Clone, Resource, Default, Reflect)]
pub struct N9Canvas {
    pub size: UVec2,
    pub background: Option<Entity>,
    pub handle: Handle<Image>,
    pub gfx_handle: Handle<Gfx>,
    pub bit_depth: u8,
}

#[derive(Component, Debug, Reflect)]
pub struct Background;

#[derive(Component, Debug, Reflect)]
pub struct OneColorBackground;

bobtail::define! {

    #[doc(hidden)]
    pub __cls => fn cls(&mut self, #[tail] color: Option<PColor>) -> Result<(), Error>;
}
pub use __cls as cls;

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

#[allow(clippy::too_many_arguments)]
pub fn setup_canvas(
    canvas: Option<ResMut<N9Canvas>>,
    mut assets: ResMut<Assets<Image>>,
    mut gfxs: ResMut<Assets<Gfx>>,
    camera: Single<Entity, With<Nano9Camera>>,
    mut state: ResMut<Pico8State>,
    defaults: Res<Defaults>,
    mut gfx_materials: ResMut<Assets<GfxMaterial>>,
    mut commands: Commands,
) {
    let Some(mut canvas) = canvas else {
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

        // What should the bitdepth be configurable?
        let gfx_image = Gfx::new(
            defaults.canvas_bit_depth.into(),
            canvas.size.x as usize,
            canvas.size.y as usize,
        );
        let gfx_handle = gfxs.add(gfx_image);
        let material = state.gfx_material(&mut gfx_materials);
        canvas.gfx_handle = gfx_handle.clone();
        canvas.background = Some(
            commands
                .spawn((
                    Name::new("canvas"),
                    GfxSprite {
                        image: gfx_handle,
                        material,
                    },
                    GfxDirty::default(),
                    Transform::from_xyz(0.0, 0.0, -100.0),
                    Background,
                ))
                .insert(ChildOf(camera_id))
                .id(),
        );
    } else if canvas.is_changed() {
        trace!("sync canvas");
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy)]
struct Dolly;

fn spawn_camera(mut commands: Commands,
                canvas: Option<Res<N9Canvas>>,
                mut dolly: Query<&mut Transform, With<Dolly>>,
) {

    let Some(mut canvas) = canvas else {
        return;
    };
    if canvas.is_added() {
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
                parent.spawn((
                    Name::new("camera"),
                    Camera2d,
                    Projection::Orthographic(OrthographicProjection::default_2d()),
                    IsDefaultUiCamera,
                    InheritedVisibility::default(),
                    Nano9Camera,
                    Position::default(),
                ));
            });
    } else if canvas.is_changed() {
        if let Ok(mut transform) = dolly.single_mut() {
        *transform = Transform::from_xyz(
                    canvas.size.x as f32 / 2.0,
                    -(canvas.size.y as f32) / 2.0,
                    0.0,
                );
        }
    }
}

pub fn sync_window_size(
    mut resize_event: MessageReader<WindowResized>,
    canvas: Option<Res<N9Canvas>>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    mut projection_query: Query<&mut Projection, With<Nano9Camera>>,
    mut camera_query: Query<&mut Camera, With<Nano9Camera>>,
) {
    let Some(canvas) = canvas else {
        return;
    };
    if let Some(e) = resize_event
        .read()
        .filter(|e| primary_windows.get(e.window).is_ok())
        .last()
    {
        let primary_window = primary_windows.get(e.window).unwrap();

        let window_scale = primary_window.scale_factor();
        let window_size = Vec2::new(
            primary_window.physical_width() as f32,
            primary_window.physical_height() as f32,
        ) / window_scale;

        let canvas_size = canvas.size.as_vec2();
        // `new_scale` is the number of physical pixels per logical pixels.
        let new_scale =
                // Canvas is longer than it is tall. Fit the width first.
                (window_size.y / canvas_size.y).min(window_size.x / canvas_size.x);

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
    }
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
                let gfx = self
                    .gfxs
                    .get_mut(&self.canvas.gfx_handle)
                    .ok_or(Error::NoAsset("gfx".into()))?;
                if gfx.set(pos.x as usize, pos.y as usize, p as u8) {
                    // if let Some(background) = self.canvas.background {
                    //     self.commands
                    //         .entity(background)
                    //         .insert(Background);
                    // }
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
                todo!()
            }
        }
        // todo!()
        // let c = self.get_color(color.into())?;
        // let image = self
        //     .images
        //     .get_mut(&self.canvas.handle)
        //     .ok_or(Error::NoAsset("canvas".into()))?;
        // image.set_color_at(pos.x, pos.y, c)?;
        // Ok(())
    }

    // XXX: pget needed
    // pub fn pget()

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
