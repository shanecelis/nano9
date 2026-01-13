use super::*;
use crate::hash::hash_f32;
use crate::translate::Position;
use bevy::platform::hash::FixedHasher;
use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) fn plugin(app: &mut App) {
    #[cfg(feature = "scripting")]
    lua::plugin(app);
}

bobtail::define! {
    #[macro_export]
    fn print(
        &mut self,
        text: impl Into<String>,
        #[tail]
        pos: Option<Vec2>,
        color: Option<PColor>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<Entity, Error>;
}

impl super::Pico8<'_, '_> {
    pub fn cursor(&mut self, pos: Option<Vec2>, color: Option<PColor>) -> (Vec2, PColor) {
        let last_pos = self.state.draw_state.print_cursor;
        let last_color = self.state.draw_state.pen;
        if let Some(pos) = pos {
            self.state.draw_state.print_cursor = pos;
        }
        if let Some(color) = color {
            self.state.draw_state.pen = color;
        }
        (last_pos, last_color)
    }

    /// print(text, [x,] [y,] [color,] [font_size])
    ///
    /// Print the given text. The Lua `print()` function will return the new x
    /// value. This function only returns the entity. To recover the new x
    /// value, one can call the `cursor().x` function.
    pub fn print(
        &mut self,
        text: impl Into<String>,
        pos: Option<Vec2>,
        color: Option<PColor>,
        font_size: Option<f32>,
        font_index: Option<usize>,
    ) -> Result<Entity, Error> {
        let text = text.into();

        let hash = {
            let mut hasher = FixedHasher.build_hasher();
            "print".hash(&mut hasher);
            text.hash(&mut hasher);
            // TODO: Color could be amended.
            // Need to hash the palette choice and
            self.state.palette.hash(&mut hasher);
            self.state.pal_map.hash(&mut hasher);
            color.inspect(|c| c.hash(&mut hasher));
            font_size.inspect(|s| hash_f32(*s, 2, &mut hasher));
            font_index.inspect(|f| f.hash(&mut hasher));
            hasher.finish()
        };
        self.state.draw_state.mark_drawn();
        // See if there's already an entity available.
        if let Some(id) = self.resurrect(hash, pos.unwrap_or(Vec2::ZERO)) {
            return Ok(id);
        }
        let clearable = Clearable::new(self.defaults.time_to_live).with_hash(hash);
        let id = self.commands.spawn_empty().id();
        self.commands.queue(move |world: &mut World| {
            if let Err(e) = Self::print_world(
                world,
                Some(id),
                text,
                pos,
                color,
                font_size,
                font_index,
                Some(clearable),
            ) {
                warn!("print error {e}");
            }
        });
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn print_world(
        world: &mut World,
        dest: Option<Entity>,
        text: String,
        pos: Option<Vec2>,
        color: Option<PColor>,
        font_size: Option<f32>,
        font_index: Option<usize>,
        clearable: Option<Clearable>,
    ) -> Result<f32, Error> {
        let (id, add_newline) = Self::pre_print_world(
            world, dest, text, pos, color, font_size, font_index, clearable,
        )?;
        // TODO: Fix for Bevy 0.17 - update_text2d_layout might have been moved/renamed
        // world
        //     .run_system_cached(bevy::text::update_text2d_layout)
        //     .expect("update_text2d_layout");
        world
            .run_system_cached_with(Self::post_print_world, (id, add_newline))
            .expect("post_print_world")
    }

    fn post_print_world(
        In((id, add_newline)): In<(Entity, bool)>,
        query: Query<(&Position, &TextLayoutInfo)>,
        mut state: ResMut<Pico8State>,
    ) -> Result<f32, Error> {
        let (position, text_layout) = query
            .get(id)
            .map_err(|_| Error::NoSuch("text layout".into()))?;
        let pos = &position.0;
        if add_newline {
            state.draw_state.print_cursor.x = pos.x;
            state.draw_state.print_cursor.y = pos.y + text_layout.size.y;
        } else {
            assert_ne!(text_layout.size.x, 0.0);
            state.draw_state.print_cursor.x = pos.x + text_layout.size.x;
        }
        Ok(pos.x + text_layout.size.x)
    }

    #[allow(clippy::too_many_arguments)]
    fn pre_print_world(
        world: &mut World,
        entity: Option<Entity>,
        mut text: String,
        pos: Option<Vec2>,
        color: Option<PColor>,
        font_size: Option<f32>,
        font_index: Option<usize>,
        clearable: Option<Clearable>,
    ) -> Result<(Entity, bool), Error> {
        let assets = world
            .get_resource::<Assets<Pico8Asset>>()
            .expect("Pico8Assets");
        let state = world.get_resource::<Pico8State>().expect("Pico8State");
        let pico8_handle = world.get_resource::<Pico8Handle>().expect("Pico8Handle");
        let pico8_asset = assets
            .get(&pico8_handle.handle)
            .ok_or(Error::NoSuch("Pico8Asset".into()))?;
        let font = pico8_asset
            .font
            .get(font_index.unwrap_or(0))
            .ok_or(Error::NoSuch("font".into()))?
            .handle
            .clone();
        let pcolor = color.unwrap_or(state.draw_state.pen);
        let c: Color = {
            let pico8_handle = world.resource::<Pico8Handle>();
            let assets = world.resource::<Assets<Pico8Asset>>();
            let pico8_asset = assets
                .get(&pico8_handle.handle)
                .ok_or_else(|| Error::NoAsset("pico8".into()))?;
            pico8_asset.palettes.get_color(pcolor, state.palette)
        }?;

        // XXX: Should the camera delta apply to the print cursor position?
        let pos = pos.unwrap_or_else(|| state.draw_state.print_cursor);
        let clearable = clearable.unwrap_or_default();
        let add_newline = if text.ends_with('\0') {
            text.pop();
            false
        } else {
            true
        };
        let font_size = font_size.unwrap_or(5.0);
        let id = entity.unwrap_or_else(|| world.spawn_empty().id());
        world.entity_mut(id).insert((
            Name::new("print"),
            Position::from(pos),
            Text2d::new(text),
            Visibility::default(),
            TextColor(c),
            TextFont {
                font,
                font_smoothing: bevy::text::FontSmoothing::None,
                font_size,
                ..default()
            },
            Anchor::TOP_LEFT,
            clearable,
        ));
        Ok((id, add_newline))
    }

    pub fn sub(string: &str, start: isize, end: Option<isize>) -> String {
        let count = string.chars().count() as isize;
        let start = if start < 0 {
            (count - start - 1) as usize
        } else {
            (start - 1) as usize
        };
        match end {
            Some(end) => {
                let end = if end < 0 {
                    (count - end) as usize
                } else {
                    end as usize
                };
                if start <= end {
                    string.chars().skip(start).take(end - start).collect()
                    // BUG: This cuts unicode boundaries.
                    // Ok(string[start..end].to_string())
                } else {
                    String::new()
                }
            }
            None => string.chars().skip(start).collect(),
        }
    }
}

#[cfg(feature = "scripting")]
mod lua {
    use super::*;
    use crate::pico8::lua::with_pico8;

    use bevy_mod_scripting::{
        bindings::InteropError,
        bindings::{
            IntoScript,
            access_map::ReflectAccessId,
            function::{
                namespace::{GlobalNamespace, NamespaceBuilder},
                script_function::FunctionCallContext,
            },
            script_value::ScriptValue,
        },
    };
    pub(crate) fn plugin(app: &mut App) {
        let world = app.world_mut();

        NamespaceBuilder::<GlobalNamespace>::new_unregistered(world)
            .register(
                "print",
                |ctx: FunctionCallContext,
                 text: Option<ScriptValue>,
                 x: Option<f32>,
                 y: Option<f32>,
                 c: Option<PColor>,
                 font_size: Option<f32>,
                 font_index: Option<usize>| {
                    let text: Cow<'_, str> = match text.unwrap_or(ScriptValue::Unit) {
                        ScriptValue::String(s) => s,
                        ScriptValue::Float(f) => format!("{f:.4}").into(),
                        ScriptValue::Integer(x) => format!("{x}").into(),
                        // If we print a zero-length string, nothing is printed.
                        // This ensures there will be a newline.
                        _ => " ".into(),
                    };
                    let (pos, hash, cached_id, ttl) = with_pico8(&ctx, |pico8| {
                        let pos_p8 = Vec2::new(
                            x.unwrap_or(pico8.state.draw_state.print_cursor.x),
                            y.unwrap_or(pico8.state.draw_state.print_cursor.y),
                        );

                        let hash = {
                            let mut hasher = FixedHasher.build_hasher();
                            "print".hash(&mut hasher);
                            text.hash(&mut hasher);
                            // TODO: Color could be amended.
                            // Need to hash the palette choice and
                            pico8.state.palette.hash(&mut hasher);
                            pico8.state.pal_map.hash(&mut hasher);
                            c.inspect(|c| c.hash(&mut hasher));
                            font_size.inspect(|s| hash_f32(*s, 2, &mut hasher));
                            font_index.inspect(|f| f.hash(&mut hasher));
                            hasher.finish()
                        };
                        // See if there's already an entity available.
                        let cached_id = pico8.resurrect(hash, pos_p8);
                        if let Some(id) = cached_id {
                            if let Ok(color) = pico8.get_color(c) {
                                pico8.commands.queue(move |world: &mut World| {
                                    if let Some(mut text_color) = world.get_mut::<TextColor>(id) {
                                        text_color.0 = color;
                                    }
                                });
                            } else {
                                warn!("Could not get textcolor");
                            }
                        }
                        pico8.state.draw_state.mark_drawn();
                        Ok((pos_p8, hash, cached_id, pico8.defaults.time_to_live))
                    })?;
                    if let Some(_id) = cached_id {
                        // TODO: It expects the width to be returned.
                        return Ok(0.0);
                    }
                    let clearable = Clearable::new(ttl).with_hash(hash);

                    let world_guard = ctx.world()?;
                    let raid = ReflectAccessId::for_global();
                    if world_guard.claim_global_access() {
                        let world = world_guard.as_unsafe_world_cell()?;
                        let world = unsafe { world.world_mut() };
                        let r = Pico8::print_world(
                            world,
                            None,
                            text.to_string(),
                            Some(pos),
                            c,
                            font_size,
                            font_index,
                            Some(clearable),
                        );
                        unsafe { world_guard.release_global_access() };
                        r.map_err(|e| InteropError::external(Box::new(e)))
                    } else {
                        Err(InteropError::cannot_claim_access(
                            raid,
                            world_guard.get_access_location(raid),
                            "print",
                        ))
                    }
                },
            )
            .register(
                "_cursor",
                |ctx: FunctionCallContext,
                 x: Option<f32>,
                 y: Option<f32>,
                 color: Option<PColor>| {
                    let (last_pos, last_color) = with_pico8(&ctx, move |pico8| {
                        let pos = if x.is_some() || y.is_some() {
                            Some(Vec2::new(x.unwrap_or(0.0), y.unwrap_or(0.0)))
                        } else {
                            None
                        };

                        Ok(pico8.cursor(pos, color))
                    })?;
                    Ok(ScriptValue::List(vec![
                        ScriptValue::Float(last_pos.x as f64),
                        ScriptValue::Float(last_pos.y as f64),
                        last_color.into_script(ctx.world()?)?,
                    ]))
                },
            )
            .register("sub", |s: String, start: isize, end: Option<isize>| {
                Pico8::sub(&s, start, end)
            });
    }
}
