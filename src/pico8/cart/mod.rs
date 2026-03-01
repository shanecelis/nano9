use crate::pico8::{audio::*, *};
use bevy::asset::{
    AssetLoader, AssetPath, LoadContext,
    io::{AssetSourceId, Reader},
};
use bevy::reflect::TypePath;
use bitvec::prelude::*;
use pico8_decompress::{decompress, extract_bits_from_png};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, path::PathBuf};

mod asset;

pub(crate) fn plugin(app: &mut App) {
    app.init_asset::<Cart>()
        // .init_asset_loader::<P8CartLoader>()
        .init_asset_loader::<PngCartLoader>()
        .add_plugins(asset::plugin);
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CartLoaderError {
    /// An [IO](std::io) Error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not convert to utf-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Could not convert to utf-8: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
    #[error("Unexpected header: {0}")]
    UnexpectedHeader(String),
    #[error("Unexpected hexadecimal: {0}")]
    UnexpectedHex(char),
    #[error("Missing: {0}")]
    Missing(String),
    #[error("Sfx error: {0}")]
    Sfx(#[from] SfxError),
    #[error("Load error: {0}")]
    LoadDirect(#[from] Box<bevy::asset::LoadDirectError>),
    #[error("Decompression error: {0}")]
    Decompression(String),
    #[error("Read bytes error: {0}")]
    ReadBytes(#[from] bevy::asset::ReadAssetBytesError),
}

#[allow(dead_code)]
#[derive(Debug, Reflect)]
pub struct MusicParts {
    begin: bool,
    end: bool,
    stop: bool,
    patterns: Vec<u8>,
}

#[derive(Asset, Debug, Reflect)]
pub struct Cart {
    pub lua: String,
    pub gfx: Option<Gfx>,
    pub map: Vec<u8>,
    pub flags: Vec<u8>,
    pub sfx: Vec<Sfx>,
    pub music: Vec<MusicParts>,
}

pub(crate) const PALETTE: [[u8; 4]; 16] = [
    [0x00, 0x00, 0x00, 0xff], //black
    [0x1d, 0x2b, 0x53, 0xff], //dark-blue
    [0x7e, 0x25, 0x53, 0xff], //dark-purple
    [0x00, 0x87, 0x51, 0xff], //dark-green
    [0xab, 0x52, 0x36, 0xff], //brown
    [0x5f, 0x57, 0x4f, 0xff], //dark-grey
    [0xc2, 0xc3, 0xc7, 0xff], //light-grey
    [0xff, 0xf1, 0xe8, 0xff], //white
    [0xff, 0x00, 0x4d, 0xff], //red
    [0xff, 0xa3, 0x00, 0xff], //orange
    [0xff, 0xec, 0x27, 0xff], //yellow
    [0x00, 0xe4, 0x36, 0xff], //green
    [0x29, 0xad, 0xff, 0xff], //blue
    [0x83, 0x76, 0x9c, 0xff], //lavender
    [0xff, 0x77, 0xa8, 0xff], //pink
    [0xff, 0xcc, 0xaa, 0xff], //light-peach
];

impl std::str::FromStr for Cart {
    type Err = CartLoaderError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Cart::from_str(s, &CartLoaderSettings::default())
    }
}

impl Cart {
    pub fn from_bytes(
        bytes: &[u8],
        settings: &CartLoaderSettings,
    ) -> Result<Cart, CartLoaderError> {
        let content = std::str::from_utf8(bytes)?;
        Cart::from_str(content, settings)
    }

    pub fn from_str(content: &str, settings: &CartLoaderSettings) -> Result<Cart, CartLoaderError> {
        const LUA: usize = 0;
        const GFX: usize = 1;
        const GFF: usize = 3;
        const MAP: usize = 4;
        const SFX: usize = 5;
        const MUSIC: usize = 6;
        let headers = ["lua", "gfx", "label", "gff", "map", "sfx", "music"];
        let mut sections = [(None, None); 7];
        let mut even_match: Option<usize> = None;
        for (index, _) in content.match_indices("__") {
            if let Some(begin) = even_match {
                let header = &content[begin + 2..index];
                if let Some(h) = headers.iter().position(|word| *word == header) {
                    sections[h].0 = Some(index + 3);
                    if let Some(last_section) = sections[0..h]
                        .iter_mut()
                        .rev()
                        .find(|(start, end)| start.is_some() && end.is_none())
                    {
                        last_section.1 = Some(begin - 1);
                    }
                } else {
                    Err(CartLoaderError::UnexpectedHeader(String::from(header)))?;
                }
                even_match = None;
            } else {
                // first or odd match of '__'
                if index == 0 || content.as_bytes()[index - 1] == b'\n' {
                    even_match = Some(index);
                }
            }
        }
        // Close the last segment.
        //
        if let Some(last_section) = sections
            .iter_mut()
            .rev()
            .find(|(start, end)| start.is_some() && end.is_none())
        {
            last_section.1 = Some(content.len());
        }
        let get_segment = |(i, j): &(Option<usize>, Option<usize>)| -> Option<&str> {
            i.zip(*j).map(|(i, j)| &content[i..j])
        };

        // lua
        let lua: String = get_segment(&sections[LUA]).unwrap_or("").into();
        let mut shared_data = vec![];
        // gfx
        let mut gfx = None;
        if let Some(content) = get_segment(&sections[GFX]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let mut rows = lines.count() + 1;
                // rows needs to be a multiple of 8.
                let partial_rows = rows % 8;
                if partial_rows != 0 {
                    rows = (rows / 8 + 1) * 8;
                }
                let mut bytes = vec![0x00; columns * rows / 2];
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len(), "line: {}", &line);

                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                if settings.shared_data == SharedData::Map {
                    // Convert the bottom portions of the sprite sheet to the map.
                    // The first two pages of sprites are rows [0,63].
                    if rows > 64 {
                        rows = 64;
                        shared_data = bytes.split_off(columns * 64 / 2);
                    } else {
                        warn!(
                            "cart settings specify shared data for map expected more than 64 rows but was {rows}."
                        );
                    }
                }
                gfx = Some(Gfx {
                    bitdepth: 4,
                    data: BitVec::<u8, Lsb0>::from_vec(bytes),
                    width: columns,
                    height: rows,
                });
            }
        }
        // gff
        let mut gff = Vec::new();
        if let Some(content) = get_segment(&sections[GFF]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let rows = lines.count() + 1;
                let mut bytes = vec![0x00; columns / 2 * rows];
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len());
                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                gff = bytes;
            }
        }
        // map
        let mut map = Vec::new();
        if let Some(content) = get_segment(&sections[MAP]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                let rows = lines.count() + 1;
                let mut bytes = vec![0x00; columns / 2 * rows];
                let mut i = 0;
                for line in content.lines() {
                    assert_eq!(columns, line.len());
                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        bytes[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                }
                map = bytes;
            }
        }
        map.extend(shared_data);
        // music
        let mut music = Vec::new();
        if let Some(content) = get_segment(&sections[MUSIC]) {
            let mut lines = content.lines();
            let columns = lines.next().map(|l| l.len());
            if let Some(columns) = columns {
                // let rows = lines.count() + 1;
                for line in content.lines() {
                    if line.is_empty() {
                        continue;
                    }
                    assert_eq!(columns, line.len());
                    let line_bytes = line.as_bytes();
                    let mut j = 0;
                    let c = line_bytes[j] as char;
                    let high: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                    let c = line_bytes[j + 1] as char;
                    let low: u8 = c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                    let lead_byte = (high << 4) | low;
                    j += 3;

                    let mut i = 0;
                    let mut patterns = [0u8; 4];
                    while j < line_bytes.len() {
                        let c = line_bytes[j] as char;
                        let high: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        let c = line_bytes[j + 1] as char;
                        let low: u8 =
                            c.to_digit(16).ok_or(CartLoaderError::UnexpectedHex(c))? as u8;
                        patterns[i] = (high << 4) | low;
                        i += 1;
                        j += 2;
                    }
                    music.push(MusicParts {
                        begin: lead_byte & 1 != 0,
                        end: lead_byte & 2 != 0,
                        stop: lead_byte & 4 != 0,
                        patterns: patterns.into_iter().filter(|p| p & 64 != 0).collect(),
                    })
                }
            }
        }

        let sfx = if let Some(content) = get_segment(&sections[SFX]) {
            let count = content.lines().count();
            let mut sfxs = Vec::with_capacity(count);
            for line in content.lines() {
                sfxs.push(Sfx::try_from(line)?);
            }
            sfxs
        } else {
            Vec::new()
        };
        Ok(Cart {
            lua,
            gfx,
            map,
            flags: gff,
            sfx,
            music,
        })
    }
}

pub(crate) fn to_nybble(a: u8) -> Option<u8> {
    let b = a as char;
    b.to_digit(16).map(|x| x as u8)
}

pub(crate) fn to_byte(a: u8, b: u8) -> Option<u8> {
    let a = to_nybble(a)?;
    let b = to_nybble(b)?;
    Some((a << 4) | b)
}

/// Pico-8 cart's have 128 8x8 sprites plus a shared region that overlaps with
/// the bottom of the map.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "cli", derive(clap::ValueEnum))]
pub enum SharedData {
    #[default]
    Sprite,
    Map,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CartLoaderSettings {
    pub shared_data: SharedData,
}

/// Convert Pico-8 dialect to Lua.
#[cfg(feature = "pico8-to-lua")]
pub(crate) async fn translate_pico8_to_lua(
    lua: &str,
    load_context: &mut LoadContext<'_>,
) -> Result<Option<String>, CartLoaderError> {
    // Patch the includes.
    let include_paths: Vec<String> = pico8_to_lua::find_includes(lua).collect();
    let has_includes = !include_paths.is_empty();
    let mut include_patch: Cow<'_, str> = Cow::Borrowed(lua);
    let mut path_contents = std::collections::HashMap::new();
    if has_includes {
        // There are included files, let's read them all then add them.
        for path in include_paths.into_iter() {
            let mut cart_path: PathBuf = load_context.path().path().to_path_buf();
            cart_path.pop();
            cart_path.push(&path);
            // dbg!(&cart_path);
            let source: AssetSourceId<'static> = load_context.path().source().clone_owned();
            let extension = cart_path.extension().and_then(|s| s.to_str()).unwrap_or("");
            match extension {
                "p8" | "png" => {
                    let include_path = AssetPath::from(cart_path).with_source(source); //.with_label("lua");
                    let pico8_asset = load_context
                        .loader()
                        .immediate()
                        // .load::<bevy_mod_scripting::prelude::ScriptAsset>(include_path)
                        .load::<Pico8Asset>(include_path)
                        .await
                        .map_err(Box::new)?;
                    let script = pico8_asset
                        .get_labeled("lua")
                        .and_then(|erased_asset| {
                            erased_asset.get::<bevy_mod_scripting::prelude::ScriptAsset>()
                        })
                        .expect("lua script");
                    path_contents
                        .insert(path, String::from_utf8(script.content.clone().into_vec())?);
                }
                "lua" => {
                    // Lua or some other code.
                    let include_path = AssetPath::from(cart_path).with_source(source);
                    let contents = load_context.read_asset_bytes(&include_path).await?;
                    path_contents.insert(path, String::from_utf8(contents)?);
                }
                ext => {
                    warn!(
                        "Extension {} not supported. Cannot include {:?}.",
                        ext, &cart_path
                    );
                    path_contents.insert(path, "error(\"Cannot include file\")".into());
                }
            }
        }

        include_patch =
            pico8_to_lua::patch_includes(lua, |path| path_contents.get(path).unwrap().into());
    }
    // Patch the code.
    let result = pico8_to_lua::patch_lua(include_patch);
    Ok(if has_includes || pico8_to_lua::was_patched(&result) {
        Some(result.to_string())
    } else {
        None
    })
}

#[derive(Default, TypePath)]
struct PngCartLoader;

pub(crate) fn log_lua_code(code: &str) {
    if let Ok(filename) = std::env::var("NANO9_LUA_CODE") {
        if let Err(e) = std::fs::write(&filename, code) {
            warn!("Unable to log lua code due to {e}");
        } else {
            info!("WROTE LUA CODE to {:?}", filename);
        }
    }
}

impl AssetLoader for PngCartLoader {
    type Asset = Cart;
    type Settings = CartLoaderSettings;
    type Error = CartLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &CartLoaderSettings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let v = extract_bits_from_png(&bytes[..])?;
        let p8scii_code: Vec<u8> = decompress(&v[0x4300..=0x7fff], None)
            .map_err(|e| CartLoaderError::Decompression(format!("{e}")))?;
        let mut code: String = p8scii::vec_to_utf8(p8scii_code);

        #[cfg(feature = "pico8-to-lua")]
        {
            let mut include_paths = vec![];
            // Patch the includes.
            let include_patch = pico8_to_lua::patch_includes(&code, |path| {
                include_paths.push(path.to_string());
                "".into()
            });
            // I believe the png format flattens the code before generated.
            assert!(include_paths.is_empty());
            // if !include_paths.is_empty() {
            // There are included files, let's read them all then add them.
            // let mut path_contents = std::collections::HashMap::new();
            // for path in include_paths.into_iter() {
            //     let contents = load_context.read_asset_bytes(&path).await?;
            //     path_contents.insert(path, String::from_utf8(contents)?);
            // }

            // include_patch = pico8_to_lua::patch_includes(&code, |path| path_contents.remove(path).unwrap());
            // }

            // Patch the code.
            let result = pico8_to_lua::patch_lua(include_patch);
            if pico8_to_lua::was_patched(&result) {
                code = result.to_string();
                log_lua_code(&code);
            }
        }
        // use std::io::Write;
        // std::fs::File::create("code.lua").expect("create code.lua").write_all(&code).expect("write code.lua");
        // dbg!(std::str::from_utf8(&code).expect("utf8"));
        let (gfx, map, flags) = match settings.shared_data {
            SharedData::Sprite => {
                let mut nybbles = vec![0; 0x2000];
                nybbles.copy_from_slice(&v[0..=0x1fff]);

                let gfx = Gfx {
                    bitdepth: 4,
                    data: BitVec::<u8, Lsb0>::from_vec(nybbles),
                    width: 128,
                    height: 128,
                };
                let mut map = vec![0; 0x1000];
                map.copy_from_slice(&v[0x2000..=0x2fff]);
                let flags = Vec::from(&v[0x3000..=0x30ff]);
                (gfx, map, flags)
            }
            SharedData::Map => {
                let mut nybbles = vec![0; 0x1000];
                nybbles.copy_from_slice(&v[0..=0x0fff]);

                let gfx = Gfx {
                    bitdepth: 4,
                    data: BitVec::<u8, Lsb0>::from_vec(nybbles),
                    width: 128,
                    height: 64,
                };
                let mut map = vec![0; 0x2000];
                map[..=0xfff].copy_from_slice(&v[0x2000..=0x2fff]);
                map[0x1000..].copy_from_slice(&v[0x1000..=0x1fff]);
                let flags = Vec::from(&v[0x3000..=0x30ff]);
                (gfx, map, flags)
            }
        };

        let sfx = v[0x3200..=0x42ff].chunks(68).map(Sfx::from_u8).collect();

        let parts = Cart {
            lua: code,
            gfx: Some(gfx),
            map,
            flags,
            sfx,
            music: vec![],
        };
        Ok(parts)
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const SAMPLE_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
 cls()
	spr(1, 0, 0)
end
__gfx__
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const BLACK_ROW_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
 cls()
	spr(1, 0, 0)
end
__gfx__
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const MAP_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
 cls()
	spr(1, 0, 0)
end
__map__
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700888888880000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const TEST_MAP_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__lua__
function _draw()
cls()
map(0, 0, 10, 10)
end
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006006600000000000880800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000006600600000000000808800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000006066000000000000888000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006660000000000000088000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__map__
0000010101000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0000010001010100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001000303030100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0101030300000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001030303000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0001010303010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
0000010101010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;
    const TEST_SFX_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__sfx__
00010000020503f050200002107000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const POOH_SFX_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__sfx__
000100001b02000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
"#;

    const GFF_CART: &str = r#"pico-8 cartridge // http://www.pico-8.com
version 41
__gfx__
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700066600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000060006000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00077000066000600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00700700006600600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
00000000000066600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
__gff__
0002020200020200000000000002020000000000000202000300000000020200000000000001010101000000000202000000000000000002000000000202020000000000000000000000000000000000000200000000000000000000000002000000000000000000020000000000000000000000000000000000000000000000
0000000000000000000200000000000002020000000000000002000000000200020200000001010303000000000000000202000000010103030000000000000000000000020200020000000000000000020200000202020200000000030303030202000002020002000002020000000002020002020200020000020200000000
"#;

    #[test]
    fn test_string_find() {
        let s = String::from("Hello World");
        assert_eq!(s.find('o'), Some(4));
        assert_eq!(s[5..].find('o'), Some(2));
    }

    #[test]
    fn test_cart_from() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(SAMPLE_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#
        );
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.width), Some(128));
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.height), Some(8));
    }

    #[test]
    fn test_cart_black_row() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(BLACK_ROW_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
 cls()
	spr(1, 0, 0)
end"#
        );
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.width), Some(128));
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.height), Some(8));
    }

    #[test]
    fn map() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(MAP_CART, &settings).unwrap();
        assert_eq!(cart.map[5], 136);
    }

    #[test]
    fn test_cart_map() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(TEST_MAP_CART, &settings).unwrap();
        assert_eq!(
            cart.lua,
            r#"function _draw()
cls()
map(0, 0, 10, 10)
end"#
        );
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.width), Some(128));
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.height), Some(8));
        assert_eq!(cart.map.len(), 128 * 7);
    }

    #[test]
    fn test_cart_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(TEST_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.width), Some(128));
        assert_eq!(cart.gfx.as_ref().map(|gfx| gfx.height), Some(8));

        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[1].volume(), 5.0 / 7.0);
        assert_eq!(notes[2].volume(), 0.0);
        assert_eq!(notes[3].volume(), 1.0);
        assert_eq!(notes[0].pitch(), 37);
        assert_eq!(notes[1].pitch(), 98);
        assert_eq!(notes[2].pitch(), 67);
        assert_eq!(notes[3].pitch(), 68);
    }

    #[test]
    fn test_pooh_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(POOH_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 2.0 / 7.0);
        assert_eq!(notes[0].pitch(), 62);
        assert_eq!(notes[0].wave(), WaveForm::Triangle);
    }

    #[test]
    fn test_sfx() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(TEST_SFX_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 1);
        let sfx = &cart.sfx[0];
        let notes = &sfx.notes;
        assert_eq!(sfx.speed, 1);
        assert_eq!(notes[0].volume(), 5.0 / 7.0);
        assert_eq!(notes[0].pitch(), 37);
        assert_eq!(notes[0].wave(), WaveForm::Triangle);
    }

    #[test]
    fn test_gff_cart() {
        let settings = CartLoaderSettings::default();
        let cart = Cart::from_str(GFF_CART, &settings).unwrap();
        assert_eq!(cart.lua, "");
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.width), None);
        // assert_eq!(cart.sprites.as_ref().map(|s| s.texture_descriptor.size.height), None);
        assert_eq!(cart.map.len(), 0);
        assert_eq!(cart.sfx.len(), 0);
        assert_eq!(cart.flags.len(), 256);
        assert_eq!(cart.flags[1], 2);
        assert_eq!(cart.flags[37], 1);
    }

    #[test]
    fn test_non_marking_space() {
        let s = "➡️ o";
        assert_eq!(s.len(), 8);
        assert_eq!(s.chars().count(), 4);
    }

    #[test]
    fn test_extension() {
        let path = PathBuf::from("dir/cart.p8.png");
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        assert_eq!(extension, "png");
    }
}
