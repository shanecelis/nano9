use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn ovalfill(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = pixel_snap(self.state.draw_state.apply_camera_delta(upper_left));
        let lower_right = pixel_snap(self.state.draw_state.apply_camera_delta(lower_right));
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        // let min = a.min(b);
        let size: UVec2 = ((lower_right.as_ivec2() - upper_left.as_ivec2()) + IVec2::ONE)
            .try_into()
            .unwrap();
        // // let size = UVec2::new((a.x - b.x).abs() + 1,
        // //                       (a.y - b.y).abs() + 1);
        // let size = UVec2::new(delta.x.abs() as u32, delta.y.abs() as u32) + UVec2::ONE;
        // dbg!(a, b, size);
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("ovalfill"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }

    pub fn oval(
        &mut self,
        upper_left: Vec2,
        lower_right: Vec2,
        color: Option<N9Color>,
    ) -> Result<Entity, Error> {
        let upper_left = pixel_snap(self.state.draw_state.apply_camera_delta(upper_left));
        let lower_right = pixel_snap(self.state.draw_state.apply_camera_delta(lower_right));
        let color = self.get_color(color.unwrap_or(N9Color::Pen))?;
        let size: UVec2 = ((lower_right.as_ivec2() - upper_left.as_ivec2()) + IVec2::ONE)
            .try_into()
            .unwrap();
        let mut pixmap = Pixmap::new(size.x, size.y).expect("pixmap");
        let oval =
            tiny_skia::Rect::from_ltrb(0.0, 0.0, size.x as f32, size.y as f32).expect("oval rect");
        let path = PathBuilder::from_oval(oval).expect("oval path");
        let mut paint = Paint::default();
        paint.anti_alias = false;
        paint.set_color_rgba8(255, 255, 255, 255);
        let mut stroke = Stroke::default();
        stroke.width = 0.0;
        pixmap.stroke_path(
            &path,
            &paint,
            &stroke,
            tiny_skia::Transform::identity(),
            None,
        );

        let mut image = Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixmap.take(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        image.sampler = ImageSampler::nearest();
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("oval"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Transform::from_xyz(upper_left.x, negate_y(upper_left.y), clearable.suggest_z()),
                clearable,
            ))
            .id();
        self.state.draw_state.mark_drawn();
        Ok(id)
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::core::bindings::function::{
        namespace::{GlobalNamespace, NamespaceBuilder},
        script_function::FunctionCallContext,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "ovalfill",
                |ctx: FunctionCallContext,
                 x0: Option<f32>,
                 y0: Option<f32>,
                 x1: Option<f32>,
                 y1: Option<f32>,
                 c: Option<N9Color>| {
                    let _ = with_pico8(&ctx, move |pico8| {
                        pico8.ovalfill(
                            Vec2::new(x0.unwrap_or(0.0), y0.unwrap_or(0.0)),
                            Vec2::new(x1.unwrap_or(0.0), y1.unwrap_or(0.0)),
                            c,
                        )
                    })?;
                    Ok(())
                },
            )
            .register(
                "oval",
                |ctx: FunctionCallContext,
                 x0: Option<f32>,
                 y0: Option<f32>,
                 x1: Option<f32>,
                 y1: Option<f32>,
                 c: Option<N9Color>| {
                    let _ = with_pico8(&ctx, move |pico8| {
                        pico8.oval(
                            Vec2::new(x0.unwrap_or(0.0), y0.unwrap_or(0.0)),
                            Vec2::new(x1.unwrap_or(0.0), y1.unwrap_or(0.0)),
                            c,
                        )
                    })?;
                    Ok(())
                },
            );
    }
}
