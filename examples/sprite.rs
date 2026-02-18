use bevy::prelude::*;
use nano9::prelude::*;
// use nano9::{cls, camera, spr, print};
//use nano9::print;
//use nano9::spr;

fn update(mut pico8: Pico8, mut t: Local<usize>) {
    // pico8.cls(Some(PColor::Palette(2))).unwrap();
    //pico8.cls(Some(2)).unwrap();
    //pico8.cls(Some(2)).unwrap();
    cls!(pico8, PColor::Palette(2)).unwrap();
    // cls!(pico8, 2i32).unwrap();
    // cls!(pico8, 2).unwrap();
    let n = ((pico8.time() * 4.0) % 8.0) + 8.0;
    let x = *t % 128;
    let y = *t / 128;

    // pico8.camera(Some((-(x as f32), 0.0)));
    // camera!(pico8, Vec2::new(-(x as f32), 0.0));
    camera!(pico8, Vec2::new(-(x as f32), 0.0));
    // pico8
    //     .spr(
    //         n as usize,
    //         Vec2::new(0.0 * x as f32, y as f32),
    //         None,
    //         Some(BVec2::new(true, false)),
    //         None,
    //     )
    //     .unwrap();
    spr!(
        pico8,
        n as usize,
        // Vec2::new(0.0 * x as f32, y as f32),
        (0.0 * x as f32, y as f32),
        _,
        BVec2::new(true, false)
    )
    .unwrap();
    // pico8.camera(Some(Vec2::ZERO));
    camera!(pico8, Vec2::ZERO);
    // pico8.print("hello world", Some(Vec2::ZERO), None, None, None).unwrap();
    crate::print!(pico8, "hello world", Vec2::ZERO).unwrap();
    *t += 1;
}

fn main() {
    let mut app = App::new();

    let config = if std::env::args()
        .next()
        .map(|s| s == "string")
        .unwrap_or(false)
    {
        println!("Loading configuration from string.");
        // OR provide configuration string.
        Config::from_str(
            r#"
            template = "pico8"
            [[sprite_sheet]]
            path = "bird-sprite.png"
            sprite_size = [16, 16]
        "#,
        )
        .expect("invalid config")
    } else {
        println!("Constructing configuration manually.");
        // Construct a configuration.
        let mut config = Config::pico8();
        config.sprite_sheets.push(nano9::config::SpriteSheet {
            path: "bird-sprite.png".into(),
            sprite_size: Some(UVec2::splat(16)),
            ..default()
        });
        config
    };
    app.add_systems(nano9::schedule::Update, update);

    app.add_plugins(Nano9Plugins)
        .add_systems(Startup, move |mut configs: ResMut<Assets<Config>>,
                     pico8assets: ResMut<Assets<nano9::pico8::Pico8Asset>>| {
                         let config_handle = configs.add(config.clone());
                         todo!("Load the pico8 asset here");

                     })
        .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
