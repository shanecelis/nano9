use super::*;
use crate::pico8::pal_map_lut::ImageWrite;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

bobtail::define! {
    #[doc(hidden)]
    pub __color => fn color(&mut self, #[tail] col: Option<PColor>) -> Result<PColor, Error>;
    #[doc(hidden)]
    pub __pal => fn pal_map(
        &mut self,
        #[tail] original_to_new: Option<(usize, usize)>,
        mode: Option<PalModify>,
    ) -> ();
    #[doc(hidden)]
    pub __palt => fn palt(
        &mut self,
        #[tail] c: Option<usize>,
        t: Option<bool>,
    ) -> ();
    #[doc(hidden)]
    pub __palm => fn palm(&mut self, #[tail] palette_index: Option<usize>) -> Result<usize, PalError>;
    #[doc(hidden)]
    pub __paln => fn paln(&self, #[tail] palette_index: Option<usize>) -> Result<usize, PalError>;
}
pub use __color as color;
pub use __pal as pal;
pub use __palm as palm;
pub use __paln as paln;
pub use __palt as palt;

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
        let mapped = pcolor.map_pal(|i| self.draw_pal_map().map_or_mod(i));
        self.palettes()?
            .get_color(mapped, self.state.palette, &self.images)
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
        match (original_to_new, mode) {
            (_, Some(PalModify::Secondary)) => {
                panic!("palette mode 2 is not supported");
            }
            (None, None) => {
                self.mutate_draw_map(PalMap::reset);
                self.state.display_pal_map.reset();
                self.reify_display_luts();
            }
            (None, Some(PalModify::Following)) => {
                self.mutate_draw_map(PalMap::reset);
            }
            (None, Some(PalModify::Present)) => {
                self.state.display_pal_map.reset();
                self.reify_display_luts();
            }
            (Some((old, new)), None | Some(PalModify::Following)) => {
                self.mutate_draw_map(|pal_map| pal_map.remap(old, new));
            }
            (Some((old, new)), Some(PalModify::Present)) => {
                self.state.display_pal_map.remap(old, new);
                self.reify_display_luts();
            }
        }
    }

    /// Current draw [`PalMapLut`]. Marks it shared so the next mode-0 edit copies the asset.
    pub(crate) fn attach_indexed_lut(&mut self) -> Handle<PalMapLut> {
        let handle = self.state.pal_map_lut.clone();
        if self.state.pal_map_lut_attached
            && self
                .pal_map_luts
                .get(&handle)
                .and_then(PalMapLut::prepared_image)
                .is_some()
        {
            return handle;
        }
        let display = self.state.display_pal_map.clone();
        let write = self
            .pal_map_luts
            .get_mut(&handle)
            .expect("current PalMapLut")
            .ensure_image(&display, &mut self.images);
        self.emit_image_write(&write);
        self.state.pal_map_lut_attached = true;
        handle
    }

    /// Point a pooled sprite at the current [`PalMapLut`] so it cannot keep a stale asset.
    pub(crate) fn rebind_indexed_lut(&mut self, id: Entity) {
        let lut = self.attach_indexed_lut();
        self.commands.queue(move |world: &mut World| {
            if world.get::<FixedPaletteLut>(id).is_some() {
                return;
            }
            if let Some(mut sprite) = world.get_mut::<GfxSprite>(id) {
                sprite.pal_map_lut = lut;
            }
        });
    }

    fn mutate_draw_map(&mut self, edit: impl FnOnce(&mut PalMap)) {
        self.state.mark_palette_dirty();
        if self.state.pal_map_lut_attached {
            let mut pal_map = self.draw_pal_map().clone();
            edit(&mut pal_map);
            self.state.pal_map_lut = self.pal_map_luts.add(PalMapLut::new(pal_map));
            self.state.pal_map_lut_attached = false;
            return;
        }
        let handle = self.state.pal_map_lut.clone();
        let display = self.state.display_pal_map.clone();
        let write = {
            let mut lut = self
                .pal_map_luts
                .get_mut(&handle)
                .expect("current PalMapLut");
            edit(lut.pal_map_mut());
            lut.image.clone().map(|image| (lut.pal_map.clone(), image))
        };
        if let Some((pal_map, image)) = write
            && crate::pico8::pal_map_lut::write_composed_lut(
                &mut self.images,
                &image,
                &pal_map,
                &display,
            )
        {
            if let Some(lut) = self.pal_map_luts.get_mut_untracked(&handle) {
                lut.mark_clean();
            }
            self.emit_modified_images(&[image.id()]);
        }
    }

    fn reify_display_luts(&mut self) {
        let display = self.state.display_pal_map.clone();
        let ids: Vec<_> = self
            .pal_map_luts
            .iter()
            .filter_map(|(id, lut)| (!lut.frozen && lut.image.is_some()).then_some(id))
            .collect();
        let mut modified = Vec::new();
        for id in ids {
            if let Some(lut) = self.pal_map_luts.get_mut_untracked(id)
                && let Some(image_id) = lut.reify_existing(&display, &mut self.images)
            {
                modified.push(image_id);
            }
        }
        self.emit_modified_images(&modified);
    }

    fn emit_image_write(&mut self, write: &ImageWrite) {
        if write.added {
            self.image_events.write(AssetEvent::Added {
                id: write.handle.id(),
            });
        } else if write.modified {
            self.emit_modified_images(&[write.handle.id()]);
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
            let transparent = transparent.unwrap_or(false);
            self.mutate_draw_map(|pal_map| pal_map.transparency.set(color_index, transparent));
        } else {
            self.mutate_draw_map(PalMap::reset_transparency);
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
                        match (old, new, mode) {
                            (None, None, None) => pico8.pal_map(None, None),
                            (Some(0), None, None) => {
                                pico8.pal_map(None, Some(PalModify::Following))
                            }
                            (Some(1), None, None) => pico8.pal_map(None, Some(PalModify::Present)),
                            (Some(p), None, None) => {
                                panic!("No such palette modify mode {p}")
                            }
                            _ => pico8.pal_map(
                                old.zip(new),
                                mode.map(|i| match i {
                                    0 => PalModify::Following,
                                    1 => PalModify::Present,
                                    2 => PalModify::Secondary,
                                    x => panic!("No such palette modify mode {x}"),
                                }),
                            ),
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
