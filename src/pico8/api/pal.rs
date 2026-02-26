use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[derive(Default, Debug, Clone)]
pub enum PalModify {
    #[default]
    Following,
    Present,
    Secondary,
}

impl super::Pico8<'_, '_> {
    pub fn palettes(&self) -> Result<&Palettes, PalError> {
        Ok(&self
            .pico8_asset()
            .map_err(|_| PalError::NoPico8Asset)?
            .palettes)
    }

    pub(crate) fn palette(&self, index: Option<usize>) -> Result<&Palette, PalError> {
        let palettes = self.palettes()?;
        let index = index.unwrap_or(self.state.palette);
        palettes.get(index).ok_or_else(|| PalError::NoSuchPalette {
            index,
            count: palettes.len(),
        })
    }

    pub(crate) fn get_pcolor(&self, c: Option<PColor>) -> PColor {
        c.unwrap_or(self.state.draw_state.pen)
    }

    pub(crate) fn get_color(&self, c: Option<PColor>) -> Result<Color, PalError> {
        let pcolor = self.get_pcolor(c);
        self.palettes()?.get_color(
            pcolor.map_pal(|i| self.state.pal_map.map_or_mod(i)),
            self.state.palette,
            &self.images,
        )
    }

    pub fn color(&mut self, color: Option<PColor>) -> Result<PColor, Error> {
        let last_color = self.state.draw_state.pen;
        if let Some(color) = color {
            if let PColor::Palette(n) = color {
                let len = self
                    .palettes()?
                    .len_in(self.state.palette, &self.images)
                    .ok()
                    .flatten()
                    .unwrap_or(0);
                if n >= len {
                    return Err(Error::NoSuch("palette color index".into()));
                }
            }
            self.state.draw_state.pen = color;
        }
        Ok(last_color)
    }

    pub fn pal_map(&mut self, original_to_new: Option<(usize, usize)>, mode: Option<PalModify>) {
        let mode = mode.unwrap_or_default();
        assert!(matches!(mode, PalModify::Following));
        if let Some((old, new)) = original_to_new {
            self.state.pal_map.remap(old, new);
            self.state.mark_palette_dirty();
        } else {
            // Reset the pal_map.
            self.state.pal_map.reset();
            self.state.mark_palette_dirty();
        }
    }

    /// Set the palette if index is given and return last palette.
    pub fn palm(&mut self, palette_index: Option<usize>) -> Result<usize, PalError> {
        let last = self.state.palette;
        Ok(match palette_index {
            Some(index) => {
                let count = self.palettes()?.len();
                if index < count {
                    self.state.palette = index;
                    self.state.mark_palette_dirty();
                } else {
                    return Err(PalError::NoSuchPalette { index, count });
                }
                last
            }
            None => last,
        })
    }

    /// Return the number of palettes if no argument is given.
    /// Return the number of colors for the given palette (from its image when loaded).
    pub fn paln(&self, palette_index: Option<usize>) -> Result<usize, PalError> {
        let palettes = self.palettes()?;
        match palette_index {
            Some(index) => {
                palettes
                    .len_in(index, &self.images)?
                    .ok_or(PalError::NoSuchPaletteColor {
                        color: 0,
                        palette: index,
                    })
            }
            None => Ok(palettes.len()),
        }
    }

    pub fn palt(&mut self, color_index: Option<usize>, transparent: Option<bool>) {
        if let Some(color_index) = color_index {
            self.state
                .pal_map
                .transparency
                .set(color_index, transparent.unwrap_or(false));
            self.state.mark_palette_dirty();
        } else {
            // Reset the pal_map's transparency.
            self.state.pal_map.reset_transparency();
            self.state.mark_palette_dirty();
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

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "pal",
                |ctx: FunctionCallContext,
                 old: Option<usize>,
                 new: Option<usize>,
                 mode: Option<u8>| {
                    with_pico8(&ctx, move |pico8| {
                        if let Some(old) = old
                            && new.is_none()
                            && mode.is_none()
                        {
                            // Set the palette.
                            pico8.state.palette = old;
                        } else {
                            pico8.pal_map(
                                old.zip(new),
                                mode.map(|i| match i {
                                    0 => PalModify::Following,
                                    1 => PalModify::Present,
                                    2 => PalModify::Secondary,
                                    x => panic!("No such palette modify mode {x}"),
                                }),
                            );
                        }
                        Ok(())
                    })
                },
            )
            .register(
                "palt",
                |ctx: FunctionCallContext, color: Option<usize>, transparency: Option<bool>| {
                    with_pico8(&ctx, move |pico8| {
                        pico8.palt(color, transparency);
                        Ok(())
                    })
                },
            )
            .register("paln", |ctx: FunctionCallContext, index: Option<usize>| {
                with_pico8(&ctx, move |pico8| pico8.paln(index).map_err(Error::from))
            })
            .register("palm", |ctx: FunctionCallContext, index: Option<usize>| {
                with_pico8(&ctx, move |pico8| {
                    match pico8.palm(index) {
                        Err(e) => match e {
                            PalError::NoSuchPalette { index, count } => {
                                pico8.palm(Some(index % count))
                            }
                            x => Err(x),
                        },
                        x => x,
                    }
                    .map_err(Error::from)
                })
            })
            .register(
                "color",
                |ctx: FunctionCallContext, color: Option<PColor>| {
                    with_pico8(&ctx, move |pico8| pico8.color(color))
                },
            );
    }
}
