use super::*;

pub(crate) fn plugin(app: &mut App) {
}

#[derive(Clone, Asset, Debug, Reflect)]
pub struct Pico8Asset {
    #[cfg(feature = "scripting")]
    pub scripts: Vec<Handle<bevy_mod_scripting::core::asset::ScriptAsset>>,
    // this palette is given away and not actually used here.
    pub(crate) palettes: Vec<Palette>,
    pub(crate) border: Handle<Image>,
    pub(crate) sprite_sheets: Vec<Handle<SpriteSheet>>,
    pub(crate) maps: Vec<SpriteMap>,
    pub(crate) font: Vec<N9Font>,
    pub(crate) audio_banks: Vec<AudioBank>,
}

#[derive(Clone, Debug, Reflect)]
pub struct N9Font {
    pub handle: Handle<Font>,
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

