use super::*;
use crate::config::Config;

pub(crate) fn plugin(_app: &mut App) {}

#[derive(Clone, Asset, Debug, Reflect)]
pub struct Pico8Asset {
    #[cfg(feature = "scripting")]
    pub scripts: Vec<Handle<bevy_mod_scripting::asset::ScriptAsset>>,
    // this palette is given away and not actually used here.
    pub(crate) palettes: Palettes,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Vec<Handle<SpriteSheet>>,
    pub(crate) maps: Vec<SpriteMap>,
    pub(crate) font: Vec<N9Font>,
    pub(crate) audio_banks: Vec<Handle<AudioBank>>,
    pub(crate) meshes: Vec<MeshHandle>,
    pub(crate) config: Handle<Config>,
}

#[derive(Clone, Debug, Reflect)]
pub struct N9Font {
    pub source: FontSource,
}

impl Pico8Asset {
    pub fn sprite_map(&self, map_index: Option<usize>) -> Result<&SpriteMap, Error> {
        let index = map_index.unwrap_or(0);
        self.maps
            .get(index)
            .ok_or(Error::NoSuch(format!("map index {index}").into()))
    }

    // pub fn get_pal(&self, palette_index: usize) -> Result<&Palette, PalError> {
    //     self.palettes.get(palette_index).ok_or(PalError::NoSuchPalette {
    //         index: palette_index,
    //         count: self.palettes.len(),
    //     })
    // }

    // // Resolve a PColor into a Color.
    // pub fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, PalError> {
    //     match c {
    //         PColor::Palette(n) => self
    //             .get_pal(palette_index)?
    //             .get_color(n)
    //             .map(|c| c.into())
    //             .map_err(|e| match e {
    //                 PalError::NoSuchColor(c) => PalError::NoSuchPaletteColor {
    //                     color: c,
    //                     palette: palette_index,
    //                 },
    //                 x => x,
    //             }),
    //         PColor::Color(c) => Ok(c.into()),
    //     }
    // }

    // pub fn sprite_map_mut(&mut self, map_index: Option<usize>) -> Result<&mut SpriteMap, Error> {
    //     let index = map_index.unwrap_or(0);
    //     self.maps
    //         .get_mut(index)
    //         .ok_or(Error::NoSuch(format!("map index {index}").into()))
    // }
}

impl FromWorld for Pico8Asset {
    fn from_world(world: &mut World) -> Self {
        let config_handle = {
            let mut configs = world.resource_mut::<Assets<Config>>();
            configs.add(Config::default())
        };

        let palettes = {
            let mut images = world.resource_mut::<Assets<Image>>();
            let strip = pico8::pal::strip_image_from_data(&pico8::PALETTE);
            let palette_handle = images.add(strip);
            vec![Palette {
                image: palette_handle,
                access: pico8::pal::PaletteAccess::default(),
            }]
            .into()
        };

        let asset_server = world.resource::<AssetServer>();

        Pico8Asset {
            #[cfg(feature = "scripting")]
            scripts: vec![],
            palettes,
            border: asset_server
                .load_with_settings(crate::config::pico8::BORDER, pixel_art_settings),
            font: vec![N9Font {
                source: asset_server.load(crate::config::pico8::FONT).into(),
            }],
            audio_banks: Vec::new(),
            sprite_sheets: Vec::new(),
            maps: Vec::new(),
            meshes: Vec::new(),
            config: config_handle,
        }
    }
}
