use super::*;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

impl super::Pico8<'_, '_> {
    // cls([n])
    pub fn cls(&mut self, color: Option<PColor>) -> Result<(), Error> {
        trace!("cls");
        let c = self.get_color(color.unwrap_or(PColor::Palette(self.defaults.clear_color)))?;
        self.state.draw_state.clear_screen();
        let image = self
            .images
            .get_mut(&self.canvas.handle)
            .ok_or(Error::NoAsset("canvas".into()))?;
        for i in 0..image.width() {
            for j in 0..image.height() {
                image.set_color_at(i, j, c)?;
            }
        }
        self.commands.send_event(ClearEvent::default());
        Ok(())
    }

    pub fn pset(&mut self, pos: UVec2, color: impl Into<N9Color>) -> Result<(), Error> {
        let c = self.get_color(color.into())?;
        let image = self
            .images
            .get_mut(&self.canvas.handle)
            .ok_or(Error::NoAsset("canvas".into()))?;
        image.set_color_at(pos.x, pos.y, c)?;
        Ok(())
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

    use bevy_mod_scripting::core::bindings::function::{
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
                |ctx: FunctionCallContext, x: u32, y: u32, color: Option<N9Color>| {
                    with_pico8(&ctx, |pico8| {
                        // We want to ignore out of bounds errors specifically but possibly not others.
                        // Ok(pico8.pset(x, y, color)?)
                        let _ = pico8.pset(UVec2::new(x, y), color.unwrap_or(N9Color::Pen));
                        Ok(())
                    })
                },
            );
    }
}
