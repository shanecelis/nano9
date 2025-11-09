use crate::pico8::Clearable;
use bevy::prelude::*;
use bevy_ecs_tiled::{TiledMapPluginConfig, prelude::*};
use bevy_ecs_tilemap::prelude::*;
// pub mod ldtk;
// use ldtk::*;
pub(crate) mod asset;
pub(crate) mod reader;
pub mod tiled;

#[derive(Debug, Clone, Reflect)]
pub enum Tiled {
    SpriteMap { handle: Handle<TiledMap> },
    World { handle: Handle<TiledWorld> },
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
                commands
                    .spawn((
                        TiledMapHandle(handle.clone()),
                        // ldtk_map: self.handle.clone(),
                        Transform::from_xyz(screen_start.x, screen_start.y, clearable.suggest_z()),
                        TiledMapAnchor::TopLeft,
                        TiledMapLayerZOffset(1.0),
                        Name::new("level"),
                        clearable,
                        InheritedVisibility::default(),
                    ))
                    .id()
            }
            Tiled::World { handle } => {
                commands
                    .spawn((
                        TiledWorldHandle(handle.clone()),
                        TiledWorldChunking::new(1000., 1000.),
                        // TiledWorldChunking(None),
                        // ldtk_map: self.handle.clone(),
                        Transform::from_xyz(screen_start.x, screen_start.y, clearable.suggest_z()),
                        TiledMapAnchor::TopLeft,
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
        .add_plugins(TiledMapPlugin(TiledMapPluginConfig { tiled_types_export_file: None }))
        .add_plugins(tiled::plugin)
        // .add_plugins(ldtk::LdtkPlugin)
        // .register_ldtk_entity::<Slime>("Slime")
        // .insert_resource(LevelSelection::index(0))
        // .add_systems(PostUpdate, process_entities)
        ;
}
