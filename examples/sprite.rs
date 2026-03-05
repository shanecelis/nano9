//! This example is the canonical draw a sprite example. It uses bobtail macros.
use bevy::prelude::*;
use nano9::prelude::*;

fn update(mut pico8: Pico8, mut t: Local<usize>) -> Result<(), BevyError> {
    cls!(pico8)?;

    let n = ((pico8.time() * 4.0) % 8.0) + 8.0;
    let x = *t % 128;
    let y = *t / 128;

    spr!(
        pico8,
        n as usize,
        (x as f32, y as f32),
        _,
        BVec2::new(true, false)
    )?;
    // It's necessary to use `nano9::print!` since `print!` is in std.
    crate::print!(pico8, "hello world")?;
    *t += 1;
    Ok(())
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
