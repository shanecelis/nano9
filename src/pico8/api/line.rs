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

    pub fn sheet_size(&self, sheet_handle: &SprHandle) -> Result<UVec2, Error> {
        Ok(match &sheet_handle {
            SprHandle::Gfx(handle) => {
                let gfx = self.gfxs.get(handle).ok_or(Error::NoSuch("Gfx".into()))?;
                UVec2::new(gfx.width as u32, gfx.height as u32)
            }
            SprHandle::Image(handle) => {
                let image = self
                    .images
                    .get(handle)
                    .ok_or(Error::NoAsset("image".into()))?;
                let sz = image.size();
                UVec2::new(sz.x, sz.y)
            }
        })
    }

    pub fn tline(
        &mut self,
        a: IVec2,
        b: IVec2,
        m_start: IVec2,
        m_delta: Option<IVec2>,
        _layers: Option<u8>,
        sheet_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let min = a.min(b);
        let delta = b - a;
        let size = UVec2::new(delta.x.unsigned_abs(), delta.y.unsigned_abs()) + UVec2::ONE;
        let c = a - min;
        let d = b - min;
        let mut m: IVec2 = m_start;
        let dm = m_delta.unwrap_or(IVec2::X);

        // NOTE: PICO-8 `tline` samples the map. We currently sample the current sprite sheet as a
        // first step.
        //
        // Output strategy:
        // - If the source is indexed (`SprHandle::Gfx`) we output a tiny `Gfx` and render it through
        //   the existing `GfxSprite` pipeline so `pal()` / `palt()` apply consistently.
        // - If the source is RGBA (`SprHandle::Image`) we output an `Image` directly.
        let sheet_handle = self.sprite_sheet(sheet_index)?.handle.clone();
        let UVec2 { x: tex_w, y: tex_h } = self.sheet_size(&sheet_handle)?;

        if tex_w == 0 || tex_h == 0 {
            return Err(Error::Message(
                "sprite sheet has zero width or height".into(),
            ));
        }

        match sheet_handle {
            SprHandle::Gfx(_handle) => {
                // Indexed output: write palette indices into a tiny `Gfx`.
                let palette = self.palette(None)?;
                let palette_bits: usize = if palette.len() == 0 {
                    4
                } else {
                    // Minimum bits to encode palette indices: ceil(log2(len))
                    (usize::BITS - (palette.len().saturating_sub(1)).leading_zeros()) as usize
                };
                let output_bitdepth: usize = palette_bits.max(1);
                let mut line_gfx =
                    pico8::Gfx::new(output_bitdepth, size.x as usize, size.y as usize);

                for (_i, (x, y)) in bresenham::Bresenham::new(
                    (c.x as isize, c.y as isize),
                    (d.x as isize, d.y as isize),
                )
                .enumerate()
                {
                    let tx = m.x.rem_euclid(tex_w as i32) as u32;
                    let ty = m.y.rem_euclid(tex_h as i32) as u32;
                    if let Some(pcolor) = self.sget(UVec2::new(tx, ty), None)? {
                        if let PColor::Palette(i) = pcolor {
                            let _ = line_gfx.set(x as usize, y as usize, i as u8);
                        }
                    }
                    m += dm;
                }

                let gfx_handle = self.gfxs.add(line_gfx);
                let clearable = Clearable::default();
                let material = self.gfx_material();
                let id = self
                    .commands
                    .spawn((
                        Name::new("tline"),
                        pico8::GfxSprite {
                            image: gfx_handle,
                            material,
                        },
                        Anchor::TOP_LEFT,
                        Position::from(min.as_vec2()),
                        clearable,
                    ))
                    .id();
                Ok(id)
            }
            SprHandle::Image(_handle) => {
                // RGBA output: sample and write colors directly.
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

                for (_i, (x, y)) in bresenham::Bresenham::new(
                    (c.x as isize, c.y as isize),
                    (d.x as isize, d.y as isize),
                )
                .enumerate()
                {
                    let tx = m.x.rem_euclid(tex_w as i32) as u32;
                    let ty = m.y.rem_euclid(tex_h as i32) as u32;
                    if let Some(pcolor) = self.sget(UVec2::new(tx, ty), None)? {
                        let color = self.get_color(Some(pcolor)).map_err(Error::from)?;
                        image.set_color_at(x as u32, y as u32, color)?;
                    }
                    m += dm;
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
             layers: Option<u8>,
             sheet: Option<usize>| {
                let _ = with_pico8(&ctx, move |pico8| {
                    pico8.tline(
                        IVec2::new(x0.unwrap_or(0), y0.unwrap_or(0)),
                        IVec2::new(x1.unwrap_or(0), y1.unwrap_or(0)),
                        IVec2::new(mx.unwrap_or(0), my.unwrap_or(0)),
                        if mdx.is_some() || mdy.is_some() {
                            Some(IVec2::new(mdx.unwrap_or(0), mdy.unwrap_or(0)))
                        } else {
                            None
                        },
                        layers,
                        sheet,
                    )
                })?;
                Ok(())
            },
        );
    }
}
