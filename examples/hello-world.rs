use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) {
    pico8.print("hello world", None, None, None, None).unwrap();
}

fn main() {
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);

    let config = Config::pico8();
    // let config = Config::gameboy();
    app.add_plugins(Nano9Plugins::new(config))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .run();
}
