use super::PColor;
use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use std::any::TypeId;

#[cfg(feature = "scripting")]
use bevy_mod_scripting::{
    GetTypeDependencies,
    bindings::{
        InteropError, WorldAccessGuard,
        docgen::typed_through::{ThroughTypeInfo, TypedThrough},
        function::from::FromScript,
        script_value::ScriptValue,
    },
};

#[derive(Debug, Clone, Copy, Reflect, Hash, Default)]
#[cfg_attr(feature = "scripting", derive(GetTypeDependencies))]
pub enum N9Color {
    #[default]
    Pen,
    PColor(PColor),
}

impl N9Color {
    pub fn into_pcolor(&self, pen_color: &PColor) -> PColor {
        match self {
            N9Color::Pen => *pen_color,
            N9Color::PColor(p) => *p,
        }
    }
}

impl From<PColor> for N9Color {
    fn from(c: PColor) -> Self {
        N9Color::PColor(c)
    }
}

impl From<usize> for N9Color {
    fn from(n: usize) -> Self {
        N9Color::PColor(n.into())
    }
}

#[cfg(feature = "scripting")]
impl TypedThrough for N9Color {
    fn through_type_info() -> ThroughTypeInfo {
        ThroughTypeInfo::TypeInfo(<N9Color as bevy::reflect::Typed>::type_info())
    }
}

#[cfg(feature = "scripting")]
impl FromScript for N9Color {
    type This<'w> = Self;
    fn from_script(
        value: ScriptValue,
        _world: WorldAccessGuard<'_>,
    ) -> Result<Self::This<'_>, InteropError> {
        match value {
            ScriptValue::Integer(n) => Ok(N9Color::PColor((n as usize).into())),
            ScriptValue::Float(f) => Ok(N9Color::PColor((f as usize).into())),
            ScriptValue::Unit => Ok(N9Color::Pen),
            _ => Err(InteropError::type_mismatch(TypeId::of::<i64>(), None)),
        }
    }
}

impl From<Option<PColor>> for N9Color {
    fn from(c: Option<PColor>) -> Self {
        match c {
            Some(c) => N9Color::PColor(c),
            None => N9Color::Pen,
        }
    }
}

// impl From<Option<N9Color>> for N9Color {
//     fn from(c: Option<N9Color>) -> Self {
//         match c {
//             Some(c) => c,
//             None => N9Color::Pen,
//         }
//     }
// }

impl From<Option<usize>> for N9Color {
    fn from(c: Option<usize>) -> Self {
        match c {
            Some(index) => N9Color::PColor(index.into()),
            None => N9Color::Pen,
        }
    }
}

impl From<Color> for N9Color {
    fn from(c: Color) -> Self {
        N9Color::PColor(c.into())
    }
}
