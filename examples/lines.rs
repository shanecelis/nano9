use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) -> Result<(), BevyError> {
    cls!(pico8)?;
    Ok(())
}

fn draw_pico8(mut pico8: Pico8) -> Result<(), BevyError> {
    let x = pico8.rnd(128);
    let y = pico8.rnd(128);
    let c = pico8.rnd(16);
    crate::line!(pico8, IVec2::ZERO, IVec2::new(x, y), PColor::Palette(c))?;
    Ok(())
}

fn draw_gameboy(mut pico8: Pico8) -> Result<(), BevyError> {
    let x = pico8.rnd(160);
    let y = pico8.rnd(144);
    let c = pico8.rnd(4);
    crate::line!(pico8, IVec2::ZERO, IVec2::new(x, y), PColor::Palette(c))?;
    Ok(())
}

fn main() {
    let gameboy = std::env::args().any(|s| s == "gameboy");
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);
    let path = if gameboy {
        app.add_systems(nano9::schedule::Draw, draw_gameboy);
        nano9::config::gameboy::CONFIG
    } else {
        app.add_systems(nano9::schedule::Draw, draw_pico8);
        nano9::config::pico8::CONFIG
    };

    app.add_plugins(Nano9Plugins::default())
        .add_systems(Startup, load_and_insert_pico8(path))
        .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
