use crate::pico8::Clearable;
use crate::translate::Position;
use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
// pub mod ldtk;
// use ldtk::*;
pub(crate) mod asset;
pub(crate) mod reader;
pub mod tiled;

#[derive(Debug, Clone, Reflect)]
pub enum Tiled {
    // TODO: TiledMap is not an Asset in bevy_ecs_tiled 0.9.5
    SpriteMap { handle: Handle<TiledMapAsset> },
    World { handle: Handle<TiledWorldAsset> },
}

impl Tiled {
    pub fn map(
        &self,
        screen_start: Vec2,
        _level: usize,
        hash: u64,
        commands: &mut Commands,
    ) -> Entity {
        let mut clearable = Clearable::default().with_hash(hash);
        clearable.time_to_live = 1;
        match self {
            Tiled::SpriteMap { handle } => commands
                .spawn((
                    TiledMap(handle.clone()),
                    Position(screen_start),
                    TilemapAnchor::TopLeft,
                    TiledMapLayerZOffset(1.0),
                    Name::new("map"),
                    clearable,
                    InheritedVisibility::default(),
                ))
                .id(),
            Tiled::World { handle } => {
                // TODO: Fix when TiledWorldHandle is available
                commands
                    .spawn((
                        TiledWorld(handle.clone()),
                        // TiledWorldChunking::new(1000., 1000.),
                        Position(screen_start),
                        TilemapAnchor::TopLeft,
                        TiledMapLayerZOffset(1.0),
                        Name::new("map"),
                        clearable,
                        InheritedVisibility::default(),
                    ))
                    .id()
            }
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app//.add_plugins(LdtkPlugin)
        .register_type::<Tiled>()
        .init_asset_loader::<asset::TiledSetLoader>()
        .add_plugins(TilemapPlugin)
        .add_plugins(TiledPlugin::default())
        .add_plugins(tiled::plugin)
        // .add_plugins(ldtk::LdtkPlugin)
        // .register_ldtk_entity::<Slime>("Slime")
        // .insert_resource(LevelSelection::index(0))
        // .add_systems(PostUpdate, process_entities)
        ;
}
