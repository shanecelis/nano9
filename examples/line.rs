use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
}

fn draw_pico8(mut pico8: Pico8) {
    let x = pico8.rnd(128);
    let y = pico8.rnd(128);
    let c = pico8.rnd(16);
    pico8
        .line(IVec2::ZERO, IVec2::new(x, y), Some(PColor::Palette(c)))
        .unwrap();
}

fn draw_gameboy(mut pico8: Pico8) {
    let x = pico8.rnd(160);
    let y = pico8.rnd(144);
    let c = pico8.rnd(4);
    pico8
        .line(IVec2::ZERO, IVec2::new(x, y), Some(PColor::Palette(c)))
        .unwrap();
}

fn main() {
    let gameboy = std::env::args().any(|s| s == "gameboy");
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);
    let mut config = if gameboy {
        app.add_systems(nano9::schedule::Draw, draw_gameboy);
        Config::gameboy()
    } else {
        app.add_systems(nano9::schedule::Draw, draw_pico8);
        Config::pico8()
    };

    let config = Config::pico8();
    // let config = Config::gameboy();
    app.add_plugins(Nano9Plugins::new(config))
        .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
