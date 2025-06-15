use bevy::prelude::*;
use nano9::prelude::*;

fn update(mut pico8: Pico8, mut t: Local<usize>) {
    pico8.cls(None).unwrap();
    let n = ((pico8.time() * 4.0) % 8.0) + 8.0;
    let x = *t % 128;
    let y = *t / 128;

    pico8
        .spr(
            n as usize,
            Vec2::new(x as f32, y as f32),
            None,
            Some(BVec2::new(true, false)),
            None,
        )
        .unwrap();
    *t += 1;
}

fn main() {
    let mut app = App::new();
    app.add_systems(Update, update.run_if(in_state(RunState::Run)));

    let config = if std::env::args().next().map(|s| s == "string").unwrap_or(false) {
        println!("Loading configuration from string.");
        // OR provide configuration string.
        Config::from_str(r#"
            template = "pico8"
            [[image]]
            path = "BirdSprite.png"
            sprite_size = [16, 16]
        "#).expect("invalid config")
    } else {
        println!("Constructing configuration manually.");
        // Construct a configuration.
        let mut config = Config::pico8();
        config.sprite_sheets.push(SpriteSheet {
            path: "BirdSprite.png".into(),
            sprite_size: Some(UVec2::splat(16)),
            ..default()
        });
        config
    };
    app.add_systems(PreUpdate, run_pico8_when_loaded);
    app.add_plugins(Nano9Plugins::new(config))
        .add_systems(PreUpdate, run_pico8_when_loaded)
        .run();
}
