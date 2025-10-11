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
    pub(crate) fn palette(&self, index: Option<usize>) -> Result<&Palette, PalError> {
        self.palettes.get_pal(index.unwrap_or(self.state.palette))
    }

    pub(crate) fn get_color(&self, c: impl Into<N9Color>) -> Result<Color, PalError> {
        let pcolor = c.into().into_pcolor(&self.state.draw_state.pen);
        self.palettes.get_color(
            pcolor.map_pal(|i| self.state.pal_map.map_or_mod(i)),
            self.state.palette,
        )
    }

    pub fn color(&mut self, color: Option<PColor>) -> Result<PColor, Error> {
        let last_color = self.state.draw_state.pen;
        if let Some(color) = color {
            if let PColor::Palette(n) = color {
                // Check that it's within the palette.
                if n >= self.palette(None)?.data.len() {
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

    /// Return the number of colors in the current palette.
    pub fn paln(&self, palette_index: Option<usize>) -> Result<usize, PalError> {
        self.palettes
            .get_pal(palette_index.unwrap_or(self.state.palette))
            .map(|pal| pal.data.len())
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
                        if old.is_some() && new.is_none() && mode.is_none() {
                            // Set the palette.
                            pico8.state.palette = old.unwrap();
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
            .register(
                "color",
                |ctx: FunctionCallContext, color: Option<PColor>| {
                    with_pico8(&ctx, move |pico8| pico8.color(color))
                },
            );
    }
}
