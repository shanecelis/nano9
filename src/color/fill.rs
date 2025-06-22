use bevy::prelude::*;
use std::any::TypeId;

use super::PColor;
#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    core::docgen::typed_through::{ThroughTypeInfo, TypedThrough},
    core::{
        bindings::{function::from::FromScript, script_value::ScriptValue, WorldAccessGuard},
        error::InteropError,
    },
    GetTypeDependencies,
};

/// This is a fill color that specifies what color to use for the "off" bit (default) and "on" bit.
#[derive(Debug, Clone, Copy, Reflect)]
#[cfg_attr(feature = "scripting", derive(GetTypeDependencies))]
pub enum FillColor {
    One { off: PColor },
    Two { off: PColor, on: PColor },
}

impl FillColor {
    pub fn is_one_color(&self) -> bool {
        matches!(self, FillColor::One { .. })
    }

    pub fn on(&self) -> Option<PColor> {
        match self {
            FillColor::One { off: _ } => None,
            FillColor::Two { off: _, on } => Some(*on),
        }
    }

    pub fn off(&self) -> PColor {
        match self {
            FillColor::One { off } => *off,
            FillColor::Two { off, on: _ } => *off,
        }
    }
}

#[cfg(feature = "scripting")]
impl TypedThrough for FillColor {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<FillColor as bevy::reflect::Typed>::type_info())
    }
}

impl From<PColor> for FillColor {
    fn from(c: PColor) -> Self {
        FillColor::One { off: c }
    }
}

impl From<usize> for FillColor {
    fn from(c: usize) -> Self {
        FillColor::One {
            off: PColor::from(c),
        }
    }
}

impl From<Color> for FillColor {
    fn from(c: Color) -> Self {
        FillColor::One {
            off: PColor::Color(c.into()),
        }
    }
}

impl From<(Color, Color)> for FillColor {
    fn from((a, b): (Color, Color)) -> Self {
        FillColor::Two {
            off: b.into(),
            on: a.into(),
        }
    }
}

#[cfg(feature = "scripting")]
impl FromScript for FillColor {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => {
                if n <= 0xf {
                    Ok(FillColor::One {
                        off: PColor::Palette(n as usize),
                    })
                } else {
                    Ok(FillColor::Two {
                        off: PColor::Palette((n & 0xf) as usize),
                        on: PColor::Palette((n >> 4) as usize),
                    })
                }
            }
            // ScriptValue::Unit => Ok(N9Color::Pen),
            _ => Err(InteropError::impossible_conversion(
                TypeId::of::<FillColor>(),
            )),
        }
    }
}
