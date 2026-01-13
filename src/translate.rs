use bevy::prelude::*;
use crate::pico8::{Clearable};

/// The position the Nano9 element was drawn. Note: it may be altered by
/// subsequent camera position changes.
#[derive(Default, Debug, Component, Reflect, Clone, Copy)]
#[require(Transform)]
pub struct Position(pub Vec2);

#[derive(Default, Debug, Component, Reflect, Clone, Copy)]
pub struct Rotation(pub Vec3);

impl From<Vec2> for Position {
    fn from(v: Vec2) -> Self {
        Position(v)
    }
}

pub(crate) fn plugin(app: &mut App) {
    app
        .register_type::<Position>()
        .register_type::<Rotation>()
        .add_systems(PostUpdate,
                     apply_translation.before(TransformSystems::Propagate));

}

fn apply_translation(
    mut query: Query<(&Position, &mut Transform, Option<&Clearable>)>) {
    for (position, mut transform, clearable_maybe) in &mut query {
        let mut v = position.0;
        v = pixel_snap(v);
        v.y = negate_y(v.y);
        transform.translation = v.extend(clearable_maybe.map(|c| c.suggest_z()).unwrap_or(0.0));
    }
}

/// Negates y IF the feature "negate-y" is enabled.
#[inline]
pub fn negate_y(y: f32) -> f32 {
    if cfg!(feature = "negate-y") { -y } else { y }
}

/// Snap to pixel IF the feature "pixel-snap" is enabled.
#[inline]
fn pixel_snap(v: Vec2) -> Vec2 {
    if cfg!(feature = "pixel-snap") {
        v.floor()
    } else {
        v
    }
}
