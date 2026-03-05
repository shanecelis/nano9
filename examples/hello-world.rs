use bevy::prelude::*;
use nano9::prelude::*;

fn init(mut pico8: Pico8) {
    let _ = crate::print!(pico8, "hello world");
}

fn main() {
    let mut app = App::new();
    app.add_systems(nano9::schedule::Init, init);
    app.add_plugins(Nano9Plugins::default())
        .add_systems(Startup, load_and_insert_pico8(nano9::config::pico8::CONFIG))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .run();
}
