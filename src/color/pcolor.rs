use bevy::prelude::*;
use std::any::TypeId;

use crate::pico8::{Error, PalMap};
use bevy::reflect::TypeRegistry;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    GetTypeDependencies,
    bindings::{
        InteropError, IntoScript, WorldAccessGuard,
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        function::from::FromScript,
        script_value::ScriptValue,
    },
};

#[derive(Debug, Clone, Copy, Reflect)]
#[cfg_attr(feature = "scripting", derive(GetTypeDependencies))]
pub enum PColor {
    Palette(usize),
    Color(Srgba),
}

impl PColor {
    /// Map the palette
    pub fn map_pal(&self, f: impl FnOnce(usize) -> usize) -> PColor {
        match self {
            PColor::Palette(i) => PColor::Palette(f(*i)),
            x => *x,
        }
    }

    pub fn write_color(
        &self,
        palette: &[[u8; 4]],
        pal_map: &PalMap,
        pixel_bytes: &mut [u8],
    ) -> Result<(), Error> {
        match self {
            PColor::Palette(i) => pal_map.write_color(palette, *i as u8, pixel_bytes),
            PColor::Color(c) => {
                let arr = c.to_u8_array();
                pixel_bytes.copy_from_slice(&arr);
                Ok(())
            }
        }
    }
}

impl std::hash::Hash for PColor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            PColor::Palette(n) => n.hash(state),
            PColor::Color(c) => {
                for x in c.to_u8_array() {
                    x.hash(state);
                }
            }
        }
    }
}

#[cfg(feature = "scripting")]
impl TypedThrough for PColor {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<PColor as bevy::reflect::Typed>::type_info())
    }
}

#[cfg(feature = "scripting")]
impl FromScript for PColor {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => Ok(PColor::Palette(n as usize)),
            ScriptValue::Float(n) => Ok(PColor::Palette(n as usize)),
            x => Err(InteropError::value_mismatch(TypeId::of::<i64>(), x)),
        }
    }
}

#[cfg(feature = "scripting")]
impl IntoScript for PColor {
    fn into_script(self, _world: WorldAccessGuard<'_>) -> Result<ScriptValue, InteropError> {
        match self {
            PColor::Palette(n) => Ok(ScriptValue::Integer(n as i64)),
            PColor::Color(n) => {
                let a = n.to_u8_array();
                Ok(ScriptValue::List(
                    a.into_iter()
                        .map(|x| ScriptValue::Integer(x as i64))
                        .collect(),
                ))
            }
        }
    }
}

impl From<Color> for PColor {
    fn from(c: Color) -> Self {
        PColor::Color(c.into())
    }
}

impl From<Srgba> for PColor {
    fn from(c: Srgba) -> Self {
        PColor::Color(c)
    }
}

impl From<usize> for PColor {
    fn from(n: usize) -> Self {
        PColor::Palette(n)
    }
}

impl From<i32> for PColor {
    fn from(n: i32) -> Self {
        PColor::Palette(n as usize)
    }
}
