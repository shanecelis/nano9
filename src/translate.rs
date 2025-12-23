use bevy::prelude::*;
use crate::pico8::{negate_y, Nano9Camera, Clearable, pixel_snap};

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
                     apply_translation.before(TransformSystem::TransformPropagate));

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
