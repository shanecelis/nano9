use crate::config;
use bevy::prelude::*;

#[derive(Debug, Resource)]
pub struct Defaults {
    pub initial_pen_color: usize,
    pub font_size: f32,
    pub clear_color: usize,
    pub initial_transparent_color: Option<usize>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            initial_pen_color: 1,
            font_size: 5.0,
            clear_color: 0,
            initial_transparent_color: Some(0),
        }
    }
}

impl Defaults {
    pub fn from_config(config_defaults: &config::Defaults) -> Self {
        Self {
            initial_pen_color: config_defaults.initial_pen_color.unwrap_or(1),
            initial_transparent_color: config_defaults.initial_transparent_color.clone(),
            font_size: config_defaults.font_size.unwrap_or(5.0),
            clear_color: config_defaults.clear_color.unwrap_or(0),
        }
    }
}
