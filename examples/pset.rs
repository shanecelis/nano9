use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
    pico8.color(None).unwrap();
}

fn update(mut pico8: Pico8, mut x: Local<u32>) {
    let _ = pico8.pset(UVec2::new(*x, *x), PColor::Palette(1));
    *x += 1;
}

fn main() {
    let mut app = App::new();
    app
        .add_systems(nano9::schedule::Init, init)
        .add_systems(nano9::schedule::Update, update);

    let config = Config::pico8();
    // let config = Config::gameboy();
    app.add_plugins(Nano9Plugins::new(config))
        .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
