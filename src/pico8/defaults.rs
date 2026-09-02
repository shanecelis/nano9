use crate::config;
use bevy::prelude::*;

#[derive(Debug, Clone, Resource)]
pub struct Defaults {
    pub initial_pen_color: usize,
    pub font_size: f32,
    pub clear_color: usize,
    pub initial_transparent_color: Option<usize>,
    pub time_to_live: u8,
    pub initial_palette: usize,
    pub canvas_bit_depth: u8,
    pub negate_y: bool,
    pub pixel_snap: bool,
    pub mute: bool,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            initial_pen_color: 1,
            font_size: 5.0,
            clear_color: 0,
            initial_transparent_color: Some(0),
            time_to_live: 1,
            initial_palette: 0,
            canvas_bit_depth: 4,
            negate_y: true,
            pixel_snap: true,
            mute: cfg!(target_arch = "wasm32"),
        }
    }
}

impl Defaults {
    pub fn from_config(config: &config::Config) -> Self {
        let d = config.defaults.as_ref();
        let screen = config.screen.as_ref();
        Self {
            initial_pen_color: d.and_then(|d| d.initial_pen_color).unwrap_or(1),
            initial_transparent_color: match d {
                Some(d) => d.initial_transparent_color,
                None => Some(0),
            },
            font_size: d.and_then(|d| d.font_size).unwrap_or(5.0),
            clear_color: d.and_then(|d| d.clear_color).unwrap_or(0),
            time_to_live: d.and_then(|d| d.time_to_live).unwrap_or(1),
            initial_palette: d.and_then(|d| d.initial_palette).unwrap_or(0),
            canvas_bit_depth: d.and_then(|d| d.canvas_bit_depth).unwrap_or(4),
            negate_y: screen.and_then(|s| s.negate_y).unwrap_or(true),
            pixel_snap: screen.and_then(|s| s.pixel_snap).unwrap_or(true),
            mute: config.mute.unwrap_or(cfg!(target_arch = "wasm32")),
        }
    }

    #[inline]
    pub fn apply_negate_y(&self, y: f32) -> f32 {
        if self.negate_y { -y } else { y }
    }

    #[inline]
    pub fn apply_pixel_snap(&self, v: Vec2) -> Vec2 {
        if self.pixel_snap { v.floor() } else { v }
    }
}
