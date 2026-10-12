use super::*;
use bevy::{
    platform::{collections::HashMap, hash::FixedHasher},
    prelude::{AssetEvent, MessageReader},
};
use std::hash::{BuildHasher, Hash, Hasher};

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Update, ensure_pal_map_capacity_on_pico8_asset_change);
}

/// When the Pico8Asset matching the current Pico8Handle is added or modified,
/// ensure pal_map has at least as many entries as the largest palette in the asset.
pub fn ensure_pal_map_capacity_on_pico8_asset_change(
    mut reader: MessageReader<AssetEvent<Pico8Asset>>,
    assets: Res<Assets<Pico8Asset>>,
    mut images: ResMut<Assets<Image>>,
    mut pal_map_luts: ResMut<Assets<PalMapLut>>,
    pico8_handle: Option<Res<Pico8Handle>>,
    mut state: ResMut<Pico8State>,
) {
    let Some(pico8_handle) = pico8_handle else {
        return;
    };
    let handle_id = pico8_handle.handle.id();
    for e in reader.read() {
        let id = match e {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => *id,
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
            state.display_pal_map.ensure_capacity(max_len);
            let display = state.display_pal_map.clone();
            let handle = state.pal_map_lut.clone();
            if let Some(mut lut) = pal_map_luts.get_mut(&handle) {
                lut.ensure_capacity(max_len);
                let _ = lut.reify_existing(&display, &mut images);
            }
        }
        break;
    }
}

/// Pico8State's state.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct Pico8State {
    /// Current draw [`PalMapLut`]. Mode 0 copies this handle after a sprite has used it.
    #[reflect(ignore)]
    pub(crate) pal_map_lut: Handle<PalMapLut>,
    /// Set once `pal_map_lut` is stored on a sprite. The next mode-0 edit copies the asset.
    #[reflect(ignore)]
    pub(crate) pal_map_lut_attached: bool,
    /// Mode-1 display remap. Applied when a [`PalMapLut`] image is reified.
    #[reflect(ignore)]
    pub(crate) display_pal_map: PalMap,
    /// Current palette
    pub(crate) palette: usize,
    pub(crate) sprite_sheet_index: usize,
    // TODO: Add image_index?
    pub(crate) draw_state: DrawState,
    pub(crate) gfx_material: Option<Handle<GfxMaterial>>,
    pub(crate) gfx_materials: HashMap<u64, Handle<GfxMaterial>>,
    /// Stem for the next `extcmd("screen")` (`extcmd("set_filename", name)`).
    pub(crate) screenshot_filename: Option<String>,
}

impl FromWorld for Pico8State {
    fn from_world(world: &mut World) -> Self {
        let defaults = world.resource::<pico8::Defaults>().clone();
        Pico8State::from_defaults(world, &defaults)
    }
}

impl Pico8State {
    pub(crate) fn from_defaults(world: &mut World, defaults: &pico8::Defaults) -> Self {
        let mut pal_map = PalMap::default();
        if let Some(trans) = defaults.initial_transparent_color {
            pal_map.transparency.set(trans, true);
        }
        let pal_map_lut = world
            .resource_mut::<Assets<PalMapLut>>()
            .add(PalMapLut::new(pal_map));
        Pico8State {
            palette: defaults.initial_palette,
            sprite_sheet_index: 0,
            pal_map_lut,
            pal_map_lut_attached: false,
            display_pal_map: PalMap::default(),
            draw_state: DrawState {
                pen: PColor::Palette(defaults.initial_pen_color),
                ..default()
            },
            gfx_material: None,
            gfx_materials: default(),
            screenshot_filename: None,
        }
    }

    pub(crate) fn replace_from_defaults(world: &mut World, defaults: &pico8::Defaults) {
        let state = Self::from_defaults(world, defaults);
        world.insert_resource(state);
    }

    pub fn mark_palette_dirty(&mut self) {
        self.gfx_material = None;
    }

    pub(crate) fn gfx_material(
        &mut self,
        pal_map: &PalMap,
        gfx_materials: &mut Assets<GfxMaterial>,
    ) -> Handle<GfxMaterial> {
        self.gfx_material
            .get_or_insert_with(|| {
                let hash = {
                    let mut hasher = FixedHasher.build_hasher();
                    self.palette.hash(&mut hasher);
                    pal_map.hash(&mut hasher);
                    hasher.finish()
                };
                self.gfx_materials
                    .entry(hash)
                    .or_insert_with(|| {
                        let gfx_material = GfxMaterial {
                            palette: self.palette,
                            pal_map: pal_map.clone(),
                        };
                        gfx_materials.add(gfx_material)
                    })
                    .clone()
            })
            .clone()
    }
}
