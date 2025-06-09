use bevy::prelude::*;
use std::hash::{Hash, Hasher};

pub fn hash_f32<H: Hasher>(a: f32, decimal_places: u8, hasher: &mut H) {
    let scale = 10f32.powi(decimal_places as i32);
    let b = (a * scale) as i32;
    b.hash(hasher);
}

#[allow(dead_code)]
pub fn hash_vec2<H: Hasher>(v: Vec2, decimal_places: u8, hasher: &mut H) {
    let scale = 10f32.powi(decimal_places as i32);
    let b = (v.x * scale) as i32;
    let c = (v.y * scale) as i32;
    b.hash(hasher);
    c.hash(hasher);
}
