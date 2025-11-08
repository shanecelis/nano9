use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) {
    pico8.cls(Some(PColor::Palette(2))).unwrap();
    pico8.color(None).unwrap();
    pico8
        .line(IVec2::ZERO, IVec2::new(127, 127), Some(PColor::Palette(1)))
        .unwrap();
}

fn main() {
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);

    let config = Config::pico8();
    // let config = Config::gameboy();
    app.add_plugins(Nano9Plugins::new(config))
        .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
