use crate::pico8::Defaults;
use crate::pico8::Clearable;
use bevy::prelude::*;

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
        //.register_type::<Position>()
        //.register_type::<Rotation>()
        .add_systems(
            PostUpdate,
            apply_translation.before(TransformSystems::Propagate),
        );
}

fn apply_translation(
    mut query: Query<(&Position, &mut Transform, Option<&Clearable>)>,
    defaults: Option<Res<Defaults>>,
) {
    let negate_y = defaults.as_ref().is_none_or(|d| d.negate_y);
    let pixel_snap = defaults.as_ref().is_none_or(|d| d.pixel_snap);
    for (position, mut transform, clearable_maybe) in &mut query {
        let mut v = position.0;
        if pixel_snap {
            v = v.floor();
        }
        if negate_y {
            v.y = -v.y;
        }
        transform.translation = v.extend(clearable_maybe.map(|c| c.suggest_z()).unwrap_or(0.0));
    }
}
