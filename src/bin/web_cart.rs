//! Browser cart: `_init` / `_update` / `_draw` via luars on wasm32-unknown-unknown.
//!
//! Build and serve (from the repo root):
//! ```sh
//! trunk serve --config web/Trunk.toml
//! ```
//!
//! Then pick a cart with `?cart=` (path under the served `assets/` tree). No
//! wasm rebuild is needed when you only change the URL or files in `assets/`:
//!
//! ```text
//! http://localhost:8080/
//! http://localhost:8080/?cart=pset.lua
//! http://localhost:8080/?cart=golden/spr.p8
//! http://localhost:8080/?cart=my-game.p8&param=headless
//! http://localhost:8080/?cart=celeste.p8.png&shared-data=map
//! ```
//!
//! Drop or symlink a cart into `assets/` (or `tests/golden/`, which Trunk copies
//! to `assets/golden/`). Wasm cannot read the rest of the filesystem.
//!
//! Requires a 16MB wasm stack (`-C link-arg=-zstack-size=16777216` in
//! `.cargo/config.toml`) and `AssetMetaCheck::Never` (set for wasm in
//! `Nano9Plugins`).

use bevy::prelude::*;
use nano9::pico8::{CartArgs, CartLoaderSettings, SharedData};
use nano9::prelude::*;

const DEFAULT_CART: &str = "web-cart.lua";

fn main() {
    let cart = cart_from_query().unwrap_or_else(|| DEFAULT_CART.to_string());
    let param = query_param("param");
    let shared_data = shared_data_from_query();
    let cart_path = bevy::asset::AssetPath::from(std::path::PathBuf::from(&cart));

    let mut app = App::new();
    app.add_plugins(Nano9Plugins::default());
    if let Some(param) = param {
        app.insert_resource(CartArgs { param, ..default() });
    }
    app.add_systems(Startup, {
        let cart = cart.clone();
        move || info!("web cart: {cart}")
    })
    .add_systems(
        Startup,
        load_and_insert_pico8_with_settings(cart_path, move |settings: &mut CartLoaderSettings| {
            settings.shared_data = shared_data;
        }),
    )
    .add_systems(PreUpdate, run_pico8_when_loaded)
    .run();
}

fn shared_data_from_query() -> SharedData {
    match query_param("shared-data").as_deref() {
        Some("map") => SharedData::Map,
        _ => SharedData::Sprite,
    }
}

fn cart_from_query() -> Option<String> {
    sanitize_cart_path(&query_param("cart")?)
}

fn sanitize_cart_path(raw: &str) -> Option<String> {
    let path = raw.trim().trim_start_matches('/');
    let path = path.strip_prefix("assets/").unwrap_or(path);
    if path.is_empty() || path.contains("..") || path.contains('\\') || path.contains(':') {
        return None;
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())?;
    matches!(ext, "lua" | "p8" | "p8lua" | "toml" | "png").then(|| path.to_string())
}

fn query_param(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let href = web_sys::window()?.location().href().ok()?;
        let url = web_sys::Url::new(&href).ok()?;
        url.search_params().get(name).filter(|s| !s.is_empty())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        None
    }
}
