use super::*;

use bevy::platform::hash::FixedHasher;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::bindings::{
    InteropError, WorldAccessGuard, function::from::FromScript, script_value::ScriptValue,
};

use crate::{
    hash::hash_f32,
    pico8::Gfx,
    translate::{Position, Rotation},
};
use std::{
    any::TypeId,
    hash::{BuildHasher, Hash, Hasher},
};

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

#[derive(Reflect, Clone, Debug, Copy, Hash)]
pub enum Spr {
    /// Sprite at current spritesheet.
    Cur { sprite: usize },
    /// Sprite from given spritesheet.
    From { sprite: usize, sheet: usize },
}

bobtail::define! {
    // #[macro_export]
    #[doc(hidden)]
    pub __spr => fn spr(
    // pub spr => fn spr(
        &mut self,
        spr: impl Into<Spr>,
        #[tail]
        pos: Vec2,
        size: Option<Vec2>,
        flip: Option<BVec2>,
        turns: Option<f32>,
    ) -> Result<Entity, Error>;
    #[doc(hidden)]
    pub __sspr => fn sspr(
        &mut self,
        sprite_rect: Rect,
        screen_pos: Vec2,
        #[tail]
        screen_size: Option<Vec2>,
        flip: Option<BVec2>,
        sheet_index: Option<usize>,
    ) -> Result<Entity, Error>;
    #[doc(hidden)]
    pub __sget => fn sget(
        &mut self,
        pos: UVec2,
        #[tail]
        sheet_index: Option<usize>,
    ) -> Result<Option<PColor>, Error>;
    #[doc(hidden)]
    pub __sset => fn sset(
        &mut self,
        pos: UVec2,
        #[tail]
        color: Option<PColor>,
        sheet_index: Option<usize>,
    ) -> Result<(), Error>;
    #[doc(hidden)]
    pub __fget => fn fget(
        &self,
        #[tail]
        index: Option<usize>,
        flag_index: Option<u8>,
    ) -> Result<u8, Error>;
    #[doc(hidden)]
    pub __fset => fn fset(
        &mut self,
        index: usize,
        #[tail]
        flag_index: Option<u8>,
        value: u8,
    ) -> Result<(), Error>;
}

pub use __fget as fget;
pub use __fset as fset;
pub use __sget as sget;
pub use __spr as spr;
pub use __sset as sset;
pub use __sspr as sspr;

#[cfg(feature = "scripting")]
impl FromScript for Spr {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Float(f) => Ok(Spr::Cur { sprite: f as usize }),
            ScriptValue::Integer(n) => Ok(Spr::Cur { sprite: n as usize }),
            ScriptValue::List(list) => {
                assert_eq!(list.len(), 2, "Expect two elements for spr.");
                let mut iter = list.into_iter().map(|v| match v {
                    ScriptValue::Integer(n) => Ok(n as usize),
                    x => Err(InteropError::external_boxed(Box::new(
                        Error::InvalidArgument(format!("{x:?}").into()),
                    ))),
                });
                let sprite = iter.next().expect("sprite index")?;
                let sheet = iter.next().expect("sheet index")?;
                Ok(Spr::From { sprite, sheet })
            }
            x => Err(InteropError::value_mismatch(TypeId::of::<Spr>(), x)),
        }
    }
}

impl From<i64> for Spr {
    fn from(index: i64) -> Self {
        Spr::Cur {
            sprite: index as usize,
        }
    }
}

impl From<usize> for Spr {
    fn from(sprite: usize) -> Self {
        Spr::Cur { sprite }
    }
}

impl From<i32> for Spr {
    fn from(sprite: i32) -> Self {
        Spr::Cur {
            sprite: sprite as usize,
        }
    }
}

impl From<(usize, usize)> for Spr {
    fn from((sprite, sheet): (usize, usize)) -> Self {
        Spr::From { sprite, sheet }
    }
}

#[derive(Debug, Clone, Reflect)]
pub enum SprHandle {
    Gfx(Handle<Gfx>),
    Image(Handle<Image>),
}

// I don't think this block is doing anything yet.
// The define! block does this.
// #[bobtail::block]
impl super::Pico8<'_, '_> {
    pub fn sprite_sheet(&self, sheet_index: Option<usize>) -> Result<&SpriteSheet, Error> {
        let sheet_index = sheet_index.unwrap_or(0);
        let handle = self
            .pico8_asset()?
            .sprite_sheets
            .get(sheet_index)
            .ok_or_else(|| Error::NoSuch("sprite sheet handle".into()))?;
        self.sprite_sheets
            .get(handle)
            .ok_or_else(|| Error::NoSuch("sprite sheet asset".into()))
    }

    pub fn sprite_sheet_mut(
        &mut self,
        sheet_index: Option<usize>,
    ) -> Result<&mut SpriteSheet, Error> {
        let sheet_index = sheet_index.unwrap_or(0);
        let handle = self
            .pico8_asset()?
            .sprite_sheets
            .get(sheet_index)
            .ok_or_else(|| Error::NoSuch("sprite sheet handle".into()))?
            .clone();
        self.sprite_sheets
            .get_mut(&handle)
            .ok_or_else(|| Error::NoSuch("sprite sheet asset".into()))
    }

    // fn sprite_sheet(&self, sheet_index: Option<usize>) -> Result<&SpriteSheet, Error> {
    //     let index = sheet_index.unwrap_or(0);
    //     self.pico8_asset()?
    //         .sprite_sheets
    //         .get(index)
    //         .ok_or(Error::NoSuch(format!("image index {index}").into()))
    // }

    // fn sprite_sheet_mut(&mut self, sheet_index: Option<usize>) -> Result<&mut SpriteSheet, Error> {
    //     let index = sheet_index.unwrap_or(0);
    //     self.pico8_asset_mut()?
    //         .sprite_sheets
    //         .get_mut(index)
    //         .ok_or(Error::NoSuch(format!("image index {index}").into()))
    // }
    /// sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] [sheet_index])
    pub fn sspr(
        &mut self,
        sprite_rect: Rect,
        screen_pos: Vec2,
        screen_size: Option<Vec2>,
        flip: Option<BVec2>,
        sheet_index: Option<usize>,
    ) -> Result<Entity, Error> {
        // let sheet_index = sheet_index.unwrap_or(0);

        let hash = {
            let mut hasher = FixedHasher.build_hasher();
            "sspr".hash(&mut hasher);
            sprite_rect.as_irect().hash(&mut hasher);
            // Need to hash the palette choice and
            self.state.palette.hash(&mut hasher);
            self.state.pal_map.hash(&mut hasher);
            screen_size.inspect(|s| s.as_uvec2().hash(&mut hasher));
            flip.inspect(|f| f.hash(&mut hasher));
            sheet_index.hash(&mut hasher);
            hasher.finish()
        };
        let flip = flip.unwrap_or_default();
        self.state.draw_state.mark_drawn();
        // See if there's already an entity available.
        if let Some(id) = self.resurrect(hash, screen_pos) {
            return Ok(id);
        }
        let sheet = self.sprite_sheet(sheet_index)?;
        let mut gfx_handle = None;
        let sprite = Sprite {
            image: match &sheet.handle {
                SprHandle::Image(handle) => handle.clone(),
                SprHandle::Gfx(handle) => {
                    gfx_handle = Some(handle.clone());
                    Handle::default()
                }
            },
            rect: Some(sprite_rect),
            custom_size: screen_size,
            flip_x: flip.x,
            flip_y: flip.y,
            ..default()
        };
        let clearable = Clearable::new(self.defaults.time_to_live).with_hash(hash);
        let material = self.gfx_material();
        let mut ecommands = self.commands.spawn((
            Name::new("sspr"),
            sprite,
            Anchor::TOP_LEFT,
            Position::from(screen_pos),
            clearable,
        ));
        if let Some(gfx_handle) = gfx_handle {
            ecommands.insert(GfxSprite {
                image: gfx_handle,
                material,
            });
        }
        Ok(ecommands.id())
    }

    // pub(crate) fn pico8_asset(&self) -> Result<&Pico8Asset, Error> {
    //     self.pico8_assets
    //         .get(&self.pico8_handle.handle)
    //         .ok_or(Error::NoSuch("Pico8Asset".into()))
    // }

    // /// TODO: Seriously reconsider this. Causes a log of asset modified events.
    // pub(crate) fn pico8_asset_mut(&mut self) -> Result<&mut Pico8Asset, Error> {
    //     self.pico8_assets
    //         .get_mut(&self.pico8_handle.handle)
    //         .ok_or(Error::NoSuch("Pico8Asset".into()))
    // }

    // #[bob]
    /// spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y])
    pub fn spr(
        &mut self,
        spr: impl Into<Spr>,
        // #[tail]
        pos: Vec2,
        size: Option<Vec2>,
        flip: Option<BVec2>,
        turns: Option<f32>,
    ) -> Result<Entity, Error> {
        // let mut pos = pixel_snap(self.state.draw_state.apply_camera_delta(pos));
        // pos.y = negate_y(pos.y);
        let spr = spr.into();
        let index;
        let hash = {
            let mut hasher = FixedHasher.build_hasher();
            "spr".hash(&mut hasher);
            let sheet = match spr {
                Spr::Cur { sprite } => {
                    index = sprite;
                    self.state.sprite_sheet_index
                }
                Spr::From { sheet, sprite } => {
                    index = sprite;
                    sheet
                }
            };
            sheet.hash(&mut hasher);
            // Need to hash the palette choice and
            self.state.palette.hash(&mut hasher);
            self.state.pal_map.hash(&mut hasher);
            size.inspect(|s| s.as_uvec2().hash(&mut hasher));
            turns.inspect(|t| hash_f32(*t, 2, &mut hasher));
            hasher.finish()
        };
        self.state.draw_state.mark_drawn();
        let flip = flip.unwrap_or_default();
        // See if there's already an entity available.
        if let Some(id) = self.resurrect(hash, pos) {
            // TODO: Set the sprite number and flip before handing off. It may have changed.
            self.commands.queue(move |world: &mut World| {
                if let Some(mut sprite) = world.get_mut::<Sprite>(id) {
                    if let Some(atlas) = &mut sprite.texture_atlas {
                        atlas.index = index;
                    }
                    sprite.flip_x = flip.x;
                    sprite.flip_y = flip.y;
                }
            });
            return Ok(id);
        }

        let (sprites, index): (&SpriteSheet, usize) = match spr {
            Spr::Cur { sprite } => (self.sprite_sheet(None)?, sprite),
            Spr::From { sheet, sprite } => (self.sprite_sheet(Some(sheet))?, sprite),
        };
        let atlas = TextureAtlas {
            layout: sprites.layout.clone(),
            index,
        };
        let rect = size.map(|v| Rect {
            min: Vec2::ZERO,
            max: sprites.sprite_size.as_vec2() * v,
        });
        let _pixel_size = sprites.sprite_size.as_vec2() * size.unwrap_or(Vec2::ONE) / 2.0;

        let mut gfx_handle = None;
        let image = match sprites.handle.clone() {
            SprHandle::Image(handle) => handle,
            SprHandle::Gfx(handle) => {
                gfx_handle = Some(handle);
                Handle::default()
                // self.palettes.get_or_create(
                //     self.state.palette,
                //     &self.state.pal_map,
                //     None,
                //     &handle,
                //     &self.gfxs,
                //     &mut self.images,
                // )?
            }
        };
        let sprite = {
            Sprite {
                image,
                texture_atlas: Some(atlas),
                rect,
                flip_x: flip.x,
                flip_y: flip.y,
                ..default()
            }
        };
        let clearable = Clearable::new(self.defaults.time_to_live).with_hash(hash);
        let material = self.gfx_material();
        let position = Position::from(pos);
        let mut ecommands = self.commands.spawn((
            Name::new("spr"),
            sprite,
            Anchor::TOP_LEFT,
            position,
            clearable,
        ));

        // let mut transform = Transform::default();
        // let mut translation = Vec2::new(pos.x, pos.y);
        if let Some(turns) = turns {
            ecommands.insert(Rotation(Vec3::new(0.0, 0.0, turns)));
            // translation.x += pixel_size.x;
            // translation.y += negate_y(pixel_size.y);
            // sprite.anchor = Anchor::Center;
            // transform.rotation = Quat::from_rotation_z(turns * 2.0 * PI);
        }
        if let Some(handle) = gfx_handle {
            ecommands.insert(GfxSprite {
                image: handle,
                material,
            });
        }
        Ok(ecommands.id())
    }

    pub fn sset(
        &mut self,
        pos: UVec2,
        color: Option<PColor>,
        sheet_index: Option<usize>,
    ) -> Result<(), Error> {
        let sheet = self.sprite_sheet(sheet_index)?;
        match sheet.handle.clone() {
            SprHandle::Gfx(handle) => {
                let color = match self.get_pcolor(color) {
                    PColor::Palette(n) => Ok(n as u8),
                    PColor::Color(_) => Err(Error::InvalidArgument(
                        "Cannot write pen `Color` to Gfx asset".into(),
                    )),
                }?;
                let gfx = self
                    .gfxs
                    .get_mut(&handle)
                    .ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.set(pos.x as usize, pos.y as usize, color);
            }
            SprHandle::Image(handle) => {
                let c = self.get_color(color)?;
                let image = self
                    .images
                    .get_mut(&handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                image.set_color_at(pos.x, pos.y, c)?;
            }
        }
        Ok(())
    }

    pub fn sget(
        &mut self,
        pos: UVec2,
        sheet_index: Option<usize>,
    ) -> Result<Option<PColor>, Error> {
        let sheet = self.sprite_sheet(sheet_index)?;
        Ok(match &sheet.handle {
            SprHandle::Gfx(handle) => {
                let gfx = self.gfxs.get(handle).ok_or(Error::NoSuch("Gfx".into()))?;
                gfx.get(pos.x as usize, pos.y as usize)
                    .map(|i| PColor::Palette(i as usize))
            }
            SprHandle::Image(handle) => {
                let image = self
                    .images
                    .get(handle)
                    .ok_or(Error::NoAsset("canvas".into()))?;
                Some(PColor::Color(image.get_color_at(pos.x, pos.y)?.into()))
            }
        })
    }

    pub fn fget(&self, index: Option<usize>, flag_index: Option<u8>) -> Result<u8, Error> {
        if index.is_none() {
            return Ok(0);
        }
        let index = index.unwrap();
        let flags = &self.sprite_sheet(None)?.flags;
        if let Some(v) = flags.get(index) {
            match flag_index {
                Some(flag_index) => {
                    if v & (1 << flag_index) != 0 {
                        Ok(1)
                    } else {
                        Ok(0)
                    }
                }
                None => Ok(*v),
            }
        } else {
            if flags.is_empty() {
                warn_once!("No flags present.");
            } else {
                warn!(
                    "Requested flag at {index}. There are only {} flags.",
                    flags.len()
                );
            }
            Ok(0)
        }
    }

    pub fn fset(&mut self, index: usize, flag_index: Option<u8>, value: u8) -> Result<(), Error> {
        let flags = &mut self.sprite_sheet_mut(None)?.flags;
        match flag_index {
            Some(flag_index) => {
                if value != 0 {
                    // Set the bit.
                    flags[index] |= 1 << flag_index;
                } else {
                    // Unset the bit.
                    flags[index] &= !(1 << flag_index);
                }
            }
            None => {
                flags[index] = value;
            }
        };
        Ok(())
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::{DropPolicy, N9Entity, pico8::lua::with_pico8};

    use bevy_mod_scripting::bindings::{
        ReflectReference,
        function::{
            from::FromScript,
            into_ref::IntoScriptRef,
            namespace::{GlobalNamespace, NamespaceBuilder},
            script_function::FunctionCallContext,
        },
        script_value::ScriptValue,
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            // spr(n, [x,] [y,] [w,] [h,] [flip_x,] [flip_y,] [turns])
            .register(
                "spr",
                |ctx: FunctionCallContext,
                 n: ScriptValue,
                 x: Option<f32>,
                 y: Option<f32>,
                 w: Option<f32>,
                 h: Option<f32>,
                 flip_x: Option<bool>,
                 flip_y: Option<bool>,
                 turns: Option<f32>| {
                    let pos = Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0));
                    let flip = (flip_x.is_some() || flip_y.is_some())
                        .then(|| BVec2::new(flip_x.unwrap_or(false), flip_y.unwrap_or(false)));
                    let size = w
                        .or(h)
                        .is_some()
                        .then(|| Vec2::new(w.unwrap_or(1.0), h.unwrap_or(1.0)));

                    let n = Spr::from_script(n, ctx.world()?)?;
                    let id = with_pico8(&ctx, move |pico8| pico8.spr(n, pos, size, flip, turns))?;

                    let entity = N9Entity {
                        entity: id,
                        drop: DropPolicy::Nothing,
                    };
                    let world = ctx.world()?;
                    let reference = {
                        let allocator = world.allocator();
                        let mut allocator = allocator.write();
                        ReflectReference::new_allocated(entity, &mut allocator)
                    };
                    ReflectReference::into_script_ref(reference, world)
                    // Ok(())
                },
            )
            // sspr( sx, sy, sw, sh, dx, dy, [dw,] [dh,] [flip_x,] [flip_y,] [sheet_index] )
            .register(
                "sspr",
                |ctx: FunctionCallContext,
                 sx: f32,
                 sy: f32,
                 sw: f32,
                 sh: f32,
                 dx: f32,
                 dy: f32,
                 dw: Option<f32>,
                 dh: Option<f32>,
                 flip_x: Option<bool>,
                 flip_y: Option<bool>,
                 sheet_index: Option<usize>| {
                    let sprite_rect = Rect::new(sx, sy, sx + sw, sy + sh);
                    let pos = Vec2::new(dx, dy);
                    let size = dw
                        .or(dh)
                        .is_some()
                        .then(|| Vec2::new(dw.unwrap_or(sw), dh.unwrap_or(sh)));
                    let flip = (flip_x.is_some() || flip_y.is_some())
                        .then(|| BVec2::new(flip_x.unwrap_or(false), flip_y.unwrap_or(false)));
                    // We get back an entity. Not doing anything with it here yet.
                    let _id = with_pico8(&ctx, move |pico8| {
                        pico8.sspr(sprite_rect, pos, size, flip, sheet_index)
                    })?;
                    Ok(())
                },
            )
            .register(
                "sset",
                |ctx: FunctionCallContext,
                 x: u32,
                 y: u32,
                 color: Option<PColor>,
                 sprite_index: Option<usize>| {
                    with_pico8(&ctx, move |pico8| {
                        pico8.sset(UVec2::new(x, y), color, sprite_index)
                    })
                },
            )
            .register(
                "sget",
                |ctx: FunctionCallContext, x: u32, y: u32, sprite_index: Option<usize>| {
                    with_pico8(&ctx, move |pico8| {
                        pico8.sget(UVec2::new(x, y), sprite_index)
                    })
                },
            )
            .register(
                "fget",
                |ctx: FunctionCallContext, n: Option<usize>, f: Option<u8>| {
                    with_pico8(&ctx, move |pico8| {
                        let v = pico8.fget(n, f)?;
                        Ok(if f.is_some() {
                            ScriptValue::Bool(v == 1)
                        } else {
                            ScriptValue::Integer(v as i64)
                        })
                    })
                },
            )
            .register(
                "fset",
                |ctx: FunctionCallContext, n: usize, f_or_v: u8, v: Option<u8>| {
                    let (f, v) = v.map(|v| (Some(f_or_v), v)).unwrap_or((None, f_or_v));
                    with_pico8(&ctx, move |pico8| pico8.fset(n, f, v))
                },
            );
    }
}
