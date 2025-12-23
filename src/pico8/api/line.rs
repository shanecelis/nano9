use super::*;
use crate::translate::Position;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn line(&mut self, a: IVec2, b: IVec2, color: impl Into<N9Color>) -> Result<Entity, Error> {
        let color = self.get_color(color)?;
        let a = self.state.draw_state.apply_camera_delta_ivec2(a);
        let b = self.state.draw_state.apply_camera_delta_ivec2(b);
        let min = a.min(b);
        let delta = b - a;
        let size = UVec2::new(delta.x.unsigned_abs(), delta.y.unsigned_abs()) + UVec2::ONE;
        let mut image = Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0u8, 0u8, 0u8, 0u8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        let c = a - min;
        let d = b - min;
        for (x, y) in
            bresenham::Bresenham::new((c.x as isize, c.y as isize), (d.x as isize, d.y as isize))
        {
            image.set_color_at(x as u32, y as u32, Color::WHITE)?;
        }
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("line"),
                Sprite {
                    image: handle,
                    color,
                    anchor: Anchor::TopLeft,
                    custom_size: Some(Vec2::new(size.x as f32, size.y as f32)),
                    ..default()
                },
                Position::from(min.as_vec2()),
                clearable,
            ))
            .id();
        Ok(id)
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

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world).register(
            "line",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             c: Option<PColor>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.line(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        c,
                    )
                })?;
                Ok(())
            },
        );
    }
}
