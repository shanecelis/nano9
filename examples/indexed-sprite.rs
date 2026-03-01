use bevy::prelude::*;
use nano9::prelude::*;
fn update(mut pico8: Pico8, mut t: Local<usize>, mut p: Local<usize>) {
    cls!(pico8).unwrap();

    // cls!(pico8, PColor::Palette(2)).unwrap();
    // cls!(pico8, 2i32).unwrap();

    // cls!(pico8, 2).unwrap();
    let n = ((pico8.time() * 4.0) % 8.0) + 8.0;
    let x = *t % 128;
    let y = *t / 128;
    let flip_horizontal = true;

    if pico8.btnp(None, None).unwrap() {
        *p += 1;
    }
    spr!(
        pico8,
        n as usize,
        (0.0 * x as f32, y as f32),
        _,
        BVec2::new(flip_horizontal, false)
    )
    .unwrap();
    pico8.palm(Some(*p % 2)).unwrap();
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
                    asset_server.load::<Pico8Asset>("indexed-sprite.toml");
                commands.insert_resource(Pico8Handle::from(pico8_asset));
            },
        )
        .add_systems(PreUpdate, run_pico8_when_loaded);
    // embedded_asset!(app, "examples", "sprite.toml");

    #[cfg(feature = "minibuffer")]
    app.add_plugins(nano9::minibuffer::quick_plugin);
    app.run();
}
