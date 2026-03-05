/// Draw the palette for a given template.
use bevy::prelude::*;
use nano9::prelude::*;
use std::{io, process::ExitCode};

fn init(mut pico8: Pico8) -> Result<(), BevyError> {
    cls!(pico8)?;
    let n = paln!(pico8, 0)?;

    let UVec2 {
        x: width,
        y: height,
    } = pico8.canvas_size();
    let dw = width as f32 / n as f32;

    for i in 0..n {
        rectfill!(
            pico8,
            Vec2::new(i as f32 * dw, 0.0),
            Vec2::new((i + 1) as f32 * dw, height as f32),
            Some(i.into())
        )?;
    }
    Ok(())
}

fn main() -> io::Result<ExitCode> {
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);

    let mut args = std::env::args();
    let path = if let Some(template) = args.nth(1) {
        match template.as_str() {
            "gameboy" => nano9::config::gameboy::CONFIG,
            _ => nano9::config::pico8::CONFIG,
        }
    } else {
        eprintln!("usage: show-palette <pico8|gameboy>");
        eprintln!("error: no template given.");
        return Ok(ExitCode::from(1));
    };

    app.add_plugins(Nano9Plugins::default())
        .add_systems(Startup, load_and_insert_pico8(path))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .run();
    Ok(ExitCode::from(0))
}
