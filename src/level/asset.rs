#[cfg(feature = "user_properties")]
use std::ops::Deref;

#[cfg(feature = "user_properties")]
use bevy::reflect::TypeRegistryArc;

// #[cfg(feature = "user_properties")]
// use bevy_ecs_tiled::properties::load::DeserializedMapProperties;

use crate::level::reader::BytesResourceReader;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};

/// [TiledMap] loading error.
#[derive(Debug, thiserror::Error)]
pub enum TiledSetLoaderError {
    /// An [IO](std::io) Error
    #[error("Could not load Tiled file: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(TypePath, Asset, Debug)]
pub struct TiledSet(pub tiled::Tileset);
#[derive(Default, Debug, Clone, Copy)]
pub struct TiledSetLoader;

impl AssetLoader for TiledSetLoader {
    type Asset = TiledSet;
    type Settings = ();
    type Error = TiledSetLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let tileset_path = load_context.path().to_path_buf();
        let tileset = {
            // Allow the loader to also load tileset images.
            let mut loader = tiled::Loader::with_cache_and_reader(
                tiled::DefaultResourceCache::new(),
                BytesResourceReader::new(&bytes, load_context),
            );
            // Load the tile set.
            loader
                .load_tsx_tileset(&tileset_path)
                .map_err(|e| std::io::Error::other(format!("Could not load TSX tile set: {e}")))?
        };
        Ok(TiledSet(tileset))
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["tsx"];
        EXTENSIONS
    }
}
