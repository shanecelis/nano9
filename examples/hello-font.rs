use bevy::prelude::*;
use nano9::prelude::*;

fn draw(mut pico8: Pico8) {
    pico8.cls(None).unwrap();
    let t = pico8.time();
    let size = (t / 3.0 % 10.0 + 4.0).floor();
    let font = (t % 2.0) as usize;
    pico8
        .print("hello world", None, None, Some(size), Some(font))
        .unwrap();

    pico8
        .print(
            format!("font {} size {:.1} ", &font, &size),
            Some(Vec2::new(0.0, 20.0)),
            Some(PColor::Palette(12)),
            None,
            None,
        )
        .unwrap();
}

fn main() {
    let mut app = App::new();
    app.add_systems(nano9::schedule::Draw, draw);

    app.add_plugins(Nano9Plugins::default())
        .add_systems(Startup, load_and_insert_pico8("hello-font.toml"))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .add_systems(
            Update,
            nano9::action::toggle_pause.run_if(nano9::condition::on_just_pressed(KeyCode::KeyP)),
        );
    #[cfg(feature = "")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    #[cfg(feature = "debugdump")]
    bevy_mod_debugdump::print_schedule_graph(&mut app, Update);
    app.run();
}
