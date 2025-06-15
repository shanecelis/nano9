use super::*;

#[derive(Clone, Asset, Debug, Reflect)]
pub struct Pico8Asset {
    #[cfg(feature = "scripting")]
    pub scripts: Vec<Handle<bevy_mod_scripting::core::asset::ScriptAsset>>,
    pub(crate) palettes: Vec<Palette>,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Vec<SpriteSheet>,
    pub(crate) maps: Vec<SpriteMap>,
    pub(crate) font: Vec<N9Font>,
    pub(crate) audio_banks: Vec<AudioBank>,
}

#[derive(Clone, Debug, Reflect)]
pub struct N9Font {
    pub handle: Handle<Font>,
}

#[derive(Debug, Clone, Reflect)]
pub struct SpriteSheet {
    pub handle: SprHandle,
    pub layout: Handle<TextureAtlasLayout>,
    pub sprite_size: UVec2,
    pub flags: Vec<u8>,
}

impl FromWorld for Pico8Asset {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Pico8Asset {
            #[cfg(feature = "scripting")]
            scripts: vec![],
            palettes: vec![Palette::from_slice(&crate::pico8::PALETTE)],
            border: asset_server.load_with_settings(PICO8_BORDER, pixel_art_settings),
            font: vec![N9Font {
                handle: asset_server.load(PICO8_FONT),
            }],
            audio_banks: Vec::new(),
            sprite_sheets: Vec::new(),
            maps: Vec::new(),
        }
    }
}

impl Pico8Asset {
    pub(crate) fn get_color(&self, c: PColor, palette_index: usize) -> Result<Color, Error> {
        match c {
            PColor::Palette(n) => self.palettes[palette_index].get_color(n).map(|c| c.into()),
            PColor::Color(c) => Ok(c.into()),
        }
    }
}
