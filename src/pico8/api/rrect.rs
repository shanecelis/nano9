use super::raster::Raster;
use super::*;
use crate::translate::Position;

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

bobtail::define! {
    #[doc(hidden)]
    pub __rrect => fn rrect(
        &mut self,
        pos: Vec2,
        size: Vec2,
        r: f32,
        #[tail]
        color: Option<PColor>,
    ) -> Result<Entity, Error>;
    #[doc(hidden)]
    pub __rrectfill => fn rrectfill(
        &mut self,
        pos: Vec2,
        size: Vec2,
        r: f32,
        #[tail]
        color: Option<PColor>,
    ) -> Result<Entity, Error>;
}
pub use __rrect as rrect;
pub use __rrectfill as rrectfill;

/// Pico-8 `rrect`/`rrectfill` take origin + size (`x, y, w, h`), not two corners.
/// Inclusive last pixel is `origin + size - sign(size)`.
fn size_to_corners(pos: Vec2, size: Vec2) -> (i32, i32, i32, i32) {
    let x = pos.x.floor() as i32;
    let y = pos.y.floor() as i32;
    let w = size.x.floor() as i32;
    let h = size.y.floor() as i32;
    let x1 = if w == 0 { x } else { x + w - w.signum() };
    let y1 = if h == 0 { y } else { y + h - h.signum() };
    let (x0, x1) = if x <= x1 { (x, x1) } else { (x1, x) };
    let (y0, y1) = if y <= y1 { (y, y1) } else { (y1, y) };
    (x0, y0, x1, y1)
}

fn spawn_rrect(
    pico8: &mut super::Pico8<'_, '_>,
    name: &'static str,
    pos: Vec2,
    size: Vec2,
    r: f32,
    color: Color,
    fill: bool,
) -> Result<Entity, Error> {
    let pen = Srgba::from(color).to_u8_array();
    let (x0, y0, x1, y1) = size_to_corners(pos, size);
    let radius = r.floor() as i32;
    let dim = UVec2::new((x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32);
    let mut raster = Raster::new(dim, pen);
    if fill {
        raster.rrectfill(0, 0, x1 - x0, y1 - y0, radius);
    } else {
        raster.rrect(0, 0, x1 - x0, y1 - y0, radius);
    }
    let handle = pico8.images.add(raster.image);
    let clearable = Clearable::default();
    let id = pico8
        .commands
        .spawn((
            Name::new(name),
            Sprite {
                image: handle,
                custom_size: Some(dim.as_vec2()),
                ..default()
            },
            Anchor::TOP_LEFT,
            Position::from(Vec2::new(x0 as f32, y0 as f32)),
            clearable,
        ))
        .id();
    pico8.state.draw_state.mark_drawn();
    Ok(id)
}

impl super::Pico8<'_, '_> {
    pub fn rrectfill(
        &mut self,
        pos: Vec2,
        size: Vec2,
        r: f32,
        color: Option<PColor>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color)?;
        spawn_rrect(self, "rrectfill", pos, size, r, color, true)
    }

    pub fn rrect(
        &mut self,
        pos: Vec2,
        size: Vec2,
        r: f32,
        color: Option<PColor>,
    ) -> Result<Entity, Error> {
        let color = self.get_color(color)?;
        spawn_rrect(self, "rrect", pos, size, r, color, false)
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
                "rrectfill",
                |ctx: FunctionCallContext,
                 x: Option<f32>,
                 y: Option<f32>,
                 w: Option<f32>,
                 h: Option<f32>,
                 r: Option<f32>,
                 c: Option<PColor>| {
                    let _ = with_pico8(&ctx, move |pico8| {
                        pico8.rrectfill(
                            Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0)),
                            Vec2::new(w.unwrap_or(0.0), h.unwrap_or(0.0)),
                            r.unwrap_or(4.0),
                            c,
                        )
                    })?;
                    Ok(())
                },
            )
            .register(
                "rrect",
                |ctx: FunctionCallContext,
                 x: Option<f32>,
                 y: Option<f32>,
                 w: Option<f32>,
                 h: Option<f32>,
                 r: Option<f32>,
                 c: Option<PColor>| {
                    let _ = with_pico8(&ctx, move |pico8| {
                        pico8.rrect(
                            Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0)),
                            Vec2::new(w.unwrap_or(0.0), h.unwrap_or(0.0)),
                            r.unwrap_or(4.0),
                            c,
                        )
                    })?;
                    Ok(())
                },
            );
    }
}
