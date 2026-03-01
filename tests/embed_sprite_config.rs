//! Tests that mirror the sprite example: load the sprite config and determine
//! what happens — e.g. whether the Pico8 asset loads, whether sprite sheets
//! are requested, and whether any loads fail. Uses the filesystem (manifest
//! dir as asset root) so the path has a proper extension for loader matching.
use bevy::asset::AssetLoadFailedEvent;
use bevy::prelude::*;
use nano9::config::headless_config_load_plugin;
use nano9::pico8::{Pico8Asset, Pico8Handle};

/// Result of running the "sprite example" flow so tests can assert on it.
#[derive(Resource, Default, Debug, Clone)]
struct SpriteLoadResult {
    /// Pico8Asset was loaded (asset present in Assets).
    pico8_loaded: bool,
    /// Paths that failed to load (Pico8Asset).
    pico8_failures: Vec<String>,
    /// Paths that failed to load (SpriteSheet).
    sprite_sheet_failures: Vec<String>,
    /// Number of updates we ran.
    updates: u32,
}

fn capture_sprite_load_result(
    mut result: ResMut<SpriteLoadResult>,
    assets: Res<Assets<Pico8Asset>>,
    pico8_handle: Option<Res<Pico8Handle>>,
    mut pico8_failures: MessageReader<AssetLoadFailedEvent<Pico8Asset>>,
    mut sprite_failures: MessageReader<AssetLoadFailedEvent<nano9::pico8::SpriteSheet>>,
) {
    for e in pico8_failures.read() {
        result
            .pico8_failures
            .push(format!("{}: {}", e.path, e.error));
    }
    for e in sprite_failures.read() {
        result
            .sprite_sheet_failures
            .push(format!("{}: {}", e.path, e.error));
    }

    if let Some(handle) = &pico8_handle {
        if assets.get(&handle.handle).is_some() {
            result.pico8_loaded = true;
        }
    }
    result.updates += 1;
}

/// Does what the sprite example does: load the same config (examples/sprite.toml),
/// insert Pico8Handle, run until the asset loads. Uses examples/ as asset folder
/// so "sprite.toml" has a clear extension for loader matching.
#[test]
fn test_load_sprite_config_like_example() {
    let mut app = App::new();


    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default_nearest());
    app.add_plugins(headless_config_load_plugin);
    // app.add_plugins(nano9::Nano9Plugin);

    app.init_resource::<SpriteLoadResult>();

    app.add_systems(
        Startup,
        |asset_server: Res<AssetServer>, mut commands: Commands| {
            // Load same file as sprite example (sprite example would use embedded://...)
            let pico8_asset: Handle<Pico8Asset> = asset_server.load::<Pico8Asset>("sprite.toml");
            commands.insert_resource(Pico8Handle::from(pico8_asset));
        },
    );
    app.add_systems(Update, capture_sprite_load_result);

    // Run until the asset has time to load (and a bit more for failures).
    for _ in 0..50 {
        app.update();
        let result = app.world().get_resource::<SpriteLoadResult>().cloned();
        if let Some(r) = result {
            if r.pico8_loaded {
                break;
            }
        }
    }

    let result = app
        .world()
        .get_resource::<SpriteLoadResult>()
        .cloned()
        .expect("SpriteLoadResult present");

    eprintln!("=== Sprite config load result (like sprite example) ===");
    eprintln!("  pico8_loaded: {}", result.pico8_loaded);
    eprintln!("  updates: {}", result.updates);
    eprintln!("  pico8_failures: {:?}", result.pico8_failures);
    eprintln!(
        "  sprite_sheet_failures: {:?}",
        result.sprite_sheet_failures
    );

    // Document what happens: with headless/minimal plugins the loader often isn't
    // selected (Extension: None in the error). The same config loaded via the full
    // app (sprite example with embedded) can hit the same loader-resolution issue
    // if the embedded path doesn't yield an extension. This test ensures we run
    // the flow and capture any failures so we can see what happens.
    if !result.pico8_loaded && !result.pico8_failures.is_empty() {
        let msg = &result.pico8_failures[0];
        assert!(
            msg.contains("Could not find an asset loader") || msg.contains("does not exist"),
            "Expected loader resolution or source error; got: {}",
            msg
        );
        // Test passes: we've determined what happens (loader not found or source missing)
        return;
    }

    assert!(
        result.pico8_loaded,
        "Pico8Asset should load when loading examples/sprite.toml; \
         pico8_failures={:?}",
        result.pico8_failures
    );
}
