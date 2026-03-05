use bevy::prelude::*;
use nano9::prelude::*;

fn update(mut pico8: Pico8, mut t: Local<usize>) {
    // pico8.cls(Some(PColor::Palette(2))).unwrap();
    //pico8.cls(Some(2)).unwrap();
    //pico8.cls(Some(2)).unwrap();
    cls!(pico8).unwrap();

    // cls!(pico8, PColor::Palette(2)).unwrap();
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
    let sheet = if btn!(pico8).unwrap() { 1 } else { 0 };
    spr!(
        pico8,
        (n as usize, sheet as usize),
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

    app.add_systems(nano9::schedule::Update, update);

    app.add_plugins(Nano9Plugins)
        .add_systems(
            Startup,
            move |asset_server: Res<AssetServer>, mut commands: Commands| {
                // let pico8_asset: Handle<Pico8Asset> = asset_server.load::<Pico8Asset>("embedded://sprite/sprite.toml");
                let pico8_asset: Handle<Pico8Asset> =
                    asset_server.load::<Pico8Asset>("sprite.toml");
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            },
        )
        .add_systems(PreUpdate, run_pico8_when_loaded);
    // embedded_asset!(app, "examples", "sprite.toml");

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
