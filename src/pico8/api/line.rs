use super::*;
use crate::translate::Position;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    pub fn line(&mut self, a: IVec2, b: IVec2, color: Option<PColor>) -> Result<Entity, Error> {
        let color = self.get_color(color)?;
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
                    custom_size: Some(size.as_vec2()),
                    ..default()
                },
                Anchor::TOP_LEFT,
                Position::from(min.as_vec2()),
                clearable,
            ))
            .id();
        Ok(id)
    }

    pub fn tline(
        &mut self,
        a: IVec2,
        b: IVec2,
        m_start: IVec2,
        m_delta: Option<IVec2>,
        _layers: Option<u8>,
    ) -> Result<Entity, Error> {
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
        let mut m = m_start;
        let dm = m_delta.unwrap_or(IVec2::X);

        for (_i, (_x, _y)) in
            bresenham::Bresenham::new((c.x as isize, c.y as isize), (d.x as isize, d.y as isize))
                .enumerate()
        {
            // TODO: Make this do the real thing.
            // let map_color =
            // image.set_color_at(x as u32, y as u32, if i % 4 >= 2 { Color::WHITE } else { Color::BLACK })?;
            m += dm;
            todo!();
        }
        let handle = self.images.add(image);
        let clearable = Clearable::default();
        let id = self
            .commands
            .spawn((
                Name::new("tline"),
                Sprite {
                    image: handle,
                    custom_size: Some(size.as_vec2()),
                    ..default()
                },
                Anchor::TOP_LEFT,
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

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world).register(
            "tline",
            |ctx: FunctionCallContext,
             x0: Option<i32>,
             y0: Option<i32>,
             x1: Option<i32>,
             y1: Option<i32>,
             mx: Option<i32>,
             my: Option<i32>,
             mdx: Option<i32>,
             mdy: Option<i32>,
             layers: Option<u8>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.tline(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        IVec2::new(mx.unwrap_or(0), my.unwrap_or(0)),
                        if mdx.is_some() || mdy.is_some() {
                            Some(IVec2::new(mx.unwrap_or(0), my.unwrap_or(0)))
                        } else {
                            None
                        },
                        layers,
                    )
                })?;
                Ok(())
            },
        );
    }
}
