use crate::pico8::Clearable;
use bevy::prelude::*;
use crate::translate::Position;
use bevy_ecs_tiled::prelude::*;
use bevy_ecs_tilemap::prelude::*;
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
    pub fn map(&self, screen_start: Vec2, _level: usize, commands: &mut Commands) -> Entity {
        // commands.insert_resource(LevelSelection::index(level));
        let clearable = Clearable::default();

        // let mut transform =
        //     get_tilemap_top_left_transform(&map_size, &grid_size, &map_type, clearable.suggest_z());
        // transform.translation += screen_start.extend(0.0);
        match self {
            Tiled::SpriteMap { handle } => {
                // TODO: Fix when TiledMapHandle is available
                commands
                    .spawn((
                        TiledMap(handle.clone()),
                        Position(screen_start),
                        TilemapAnchor::TopLeft,
                        TiledMapLayerZOffset(1.0),
                        Name::new("level"),
                        clearable,
                        InheritedVisibility::default(),
                    ))
                    .id()
            }
            Tiled::World { handle } => {
                // TODO: Fix when TiledWorldHandle is available
                commands
                    .spawn((
                        TiledWorld(handle.clone()),
                        // TiledWorldChunking::new(1000., 1000.),
                        Position(screen_start),
                        TilemapAnchor::TopLeft,
                        TiledMapLayerZOffset(1.0),
                        Name::new("level"),
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
