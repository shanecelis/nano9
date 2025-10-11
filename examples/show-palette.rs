/// Draw the palette for a given template.
use bevy::prelude::*;
use nano9::prelude::*;
use std::{io, process::ExitCode};

fn init(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
    let n = pico8.paln(None).unwrap();

    let UVec2 {
        x: width,
        y: height,
    } = pico8.canvas_size();
    let dw = width as f32 / n as f32;

    for i in 0..n {
        pico8
            .rectfill(
                Vec2::new(i as f32 * dw, 0.0),
                Vec2::new((i + 1) as f32 * dw, height as f32),
                Some(i.into()),
            )
            .unwrap();
    }
}

fn main() -> io::Result<ExitCode> {
    let mut app = App::new();
    app
        .add_systems(nano9::schedule::Init, init);

    let mut args = std::env::args();
    if let Some(template) = args.nth(1) {
        let mut config = Config::default();
        if let Err(e) = config.inject_template(Some(&template)) {
            eprintln!("error: {e}");
            return Ok(ExitCode::from(2));
        }
        app.add_plugins(Nano9Plugins::new(config))
            .add_systems(PreUpdate, run_pico8_when_loaded)
            .run();
        Ok(ExitCode::from(0))
    } else {
        eprintln!("usage: show-palette <pico8|gameboy>");
        eprintln!("error: no template given.");
        Ok(ExitCode::from(1))
    }
}
