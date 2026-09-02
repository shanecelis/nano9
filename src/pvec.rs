use bevy::prelude::Vec2;

/// A PVec2 represents a Pico-8 vector, for which the positive y-axis points
/// downward.
///
/// [Vec2::from] / [PVec2::from] negate y to convert between Pico-8 and Bevy
/// conventions. Construct with `PVec2(v)` when `v` is already Pico-8-oriented.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PVec2(pub Vec2);

impl From<PVec2> for Vec2 {
    fn from(a: PVec2) -> Vec2 {
        let mut v = a.0;
        v.y = -v.y;
        v
    }
}

impl From<Vec2> for PVec2 {
    fn from(mut v: Vec2) -> PVec2 {
        v.y = -v.y;
        PVec2(v)
    }
}
