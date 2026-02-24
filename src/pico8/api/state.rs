use super::*;
use bevy::{
    platform::{collections::HashMap, hash::FixedHasher},
    prelude::{AssetEvent, MessageReader},
};
use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) fn plugin(app: &mut App) {
    app
        .add_systems(Update, ensure_pal_map_capacity_on_pico8_asset_change);
}


/// When the Pico8Asset matching the current Pico8Handle is added or modified,
/// ensure pal_map has at least as many entries as the largest palette in the asset.
pub fn ensure_pal_map_capacity_on_pico8_asset_change(
    mut reader: MessageReader<AssetEvent<Pico8Asset>>,
    assets: Res<Assets<Pico8Asset>>,
    images: Res<Assets<Image>>,
    pico8_handle: Option<Res<Pico8Handle>>,
    mut state: ResMut<Pico8State>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    let handle_id = pico8_handle.handle.id();
    for e in reader.read() {
        let id = match e {
            AssetEvent::Added { id } | AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } => *id,
            AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => continue,
        };
        if id != handle_id {
            continue;
        }
        let Some(asset) = assets.get(id) else {
            continue;
        };
        let max_len = asset
            .palettes
            .iter()
            .filter_map(|p| images.get(&p.image).map(|img| p.len_in(img)))
            .max();
        if let Some(max_len) = max_len {
            state.pal_map.ensure_capacity(max_len);
        }
        break;
    }
}

/// Pico8State's state.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    #[reflect(ignore)]
    pub(crate) pal_map: PalMap,
    /// Current palette
    pub(crate) palette: usize,
    pub(crate) sprite_sheet_index: usize,
    // TODO: Add image_index?
    pub(crate) draw_state: DrawState,
    pub(crate) gfx_material: Option<Handle<GfxMaterial>>,
    pub(crate) gfx_materials: HashMap<u64, Handle<GfxMaterial>>,
}

// XXX: Dump this after refactor.
impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let defaults = world.resource::<pico8::Defaults>();
        let mut pal_map = PalMap::default();
        if let Some(trans) = defaults.initial_transparent_color {
            pal_map.transparency.set(trans, true);
        }
        Pico8State {
            palette: defaults.initial_palette,
            sprite_sheet_index: 0,
            pal_map,
            draw_state: DrawState {
                pen: PColor::Palette(defaults.initial_pen_color),
                ..default()
            },
            gfx_material: None,
            gfx_materials: default(),
        }
    }
}

impl Pico8State {
    pub fn mark_palette_dirty(&mut self) {
        self.gfx_material = None;
    }

    pub(crate) fn gfx_material(
        &mut self,
        gfx_materials: &mut Assets<GfxMaterial>,
    ) -> Handle<GfxMaterial> {
        self.gfx_material
            .get_or_insert_with(|| {
                let hash = {
                    let mut hasher = FixedHasher.build_hasher();
                    self.palette.hash(&mut hasher);
                    self.pal_map.hash(&mut hasher);
                    hasher.finish()
                };
                // dbg!(hash, self.palette);
                self.gfx_materials
                    .entry(hash)
                    .or_insert_with(|| {
                        let gfx_material = GfxMaterial {
                            palette: self.palette,
                            pal_map: self.pal_map.clone(),
                        };
                        gfx_materials.add(gfx_material)
                    })
                    .clone()
            })
            .clone()
    }
}
