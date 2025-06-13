use bevy::prelude::Vec2;

/// A PVec2 represents a Pico-8 vector, for which the positive y-axis points
/// downward.
///
/// When [Vec2::from] converts to a PVec2 to a Vec2, the y-element will be
/// negated if "negate-y" feature is enabled.
///
/// [PVec2::from] converts a Vec2 to a PVec2 by negating its y-element.
///
/// To construct a PVec2 `v` with its y-element already in the Pico-8
/// convention, use the constructor `PVec2(v)`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PVec2(pub Vec2);

impl From<PVec2> for Vec2 {
    fn from(a: PVec2) -> Vec2 {
        let mut v = a.0;
        #[cfg(feature = "negate-y")]
        {
            v.y = -v.y;
        }
        v
    }
}

impl From<Vec2> for PVec2 {
    fn from(mut v: Vec2) -> PVec2 {
        #[cfg(feature = "negate-y")]
        {
            v.y = -v.y;
        }
        PVec2(v)
    }
}
