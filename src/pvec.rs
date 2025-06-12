use bevy::prelude::Vec2;

/// A PVec2 represents a Pico-8 vector, for which the positive y-axis points
/// downward.
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
