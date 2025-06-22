use crate::{
    pico8::*,
    one_or_map::OneOrMap,
};
use bevy::{
    image::ImageSampler,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    utils::{HashMap, hashbrown::hash_map::DefaultHashBuilder},
    // platform::hash::DefaultHasher,
    prelude::*,
};
use std::{
    collections::VecDeque,
    hash::{Hasher, DefaultHasher, Hash},
};
use bitvec::{prelude::*, view::BitView};

pub(crate) fn plugin(app: &mut App) {
    app
        .register_type::<Gfx>()
        .register_asset_reflect::<Gfx>()
        .register_type::<GfxSprite>()
        .init_resource::<GfxImageMap>()
        .init_asset::<Gfx>()
        .init_asset::<GfxMaterial>()
        .add_systems(PostUpdate, (compute_image_on_asset_event,
                                  compute_image_on_gfx_sprite_change.after(compute_image_on_asset_event),
                                  check_dirty));
        // .add_systems(PreUpdate, (create_sprite, update_sprite));

}

type GfxImage = OneOrMap<u64, Handle<Image>>;

#[derive(Component, Default, Reflect)]
pub struct GfxSprite {
    pub image: Handle<Gfx>,
    pub material: Handle<GfxMaterial>,
}

#[derive(Asset, Debug, Reflect, Clone, Hash, PartialEq, Eq)]
pub struct GfxMaterial {
    pub palette: usize,
    pub pal_map: PalMap,
}

// #[derive(Asset, Debug, Reflect, Clone, Hash, Eq)]
// pub struct GfxMat<'a> {
//     pub palette: usize,
//     pub pal_map: &'a PalMap,
// }

#[derive(Resource, Default, Reflect, Deref, DerefMut)]
pub struct GfxImageMap(HashMap<AssetId<Gfx>, GfxImage>);

#[derive(Component, Debug, Default)]
pub struct GfxDirty(pub bool);

fn check_dirty(
    mut events: EventReader<AssetEvent<Gfx>>,
    mut query: Query<(&mut GfxDirty, &GfxSprite)>) {

    let mut modified_handles: Option<bevy::utils::HashSet<_>> = None;
    for (mut gfx_dirty, gfx_sprite) in &mut query {
        if gfx_dirty.0 {
            continue;
        }
        if modified_handles.is_none() {
            modified_handles = Some(events
                                    .read()
                                    .filter_map(|e| match e {
                                        AssetEvent::Modified { id } => Some(*id),
                                        _ => None,
                                    })
                                    .collect());
        }

        if modified_handles.as_ref().map(|set| set.contains(&gfx_sprite.image.id())).unwrap_or(false) {
            gfx_dirty.0 = true;
        }
    }
}

pub(crate) fn compute_image_sys(In(gfx_sprite): In<GfxSprite>,
                                state: Res<Pico8State>,
                                gfxs: Res<Assets<Gfx>>,
                                gfx_materials: Res<Assets<GfxMaterial>>,
                                mut images: ResMut<Assets<Image>>,
                                palettes: Res<Palettes>,
                                mut pairs: ResMut<GfxImageMap>) -> Result<Handle<Image>, Error> {
    let my_span = info_span!("gfx::compute_image", name = "system").entered();
    compute_image(&gfx_sprite.image,
                  gfx_materials.get(&gfx_sprite.material).ok_or_else(|| Error::NoSuch("gfx material".into()))?,
                  &gfxs,
                  &mut images,
                  &palettes,
                  &mut pairs)
}


fn compute_image(gfx_handle: &Handle<Gfx>,
                 gfx_material: &GfxMaterial,
                 gfxs: &Assets<Gfx>,
                 mut images: &mut Assets<Image>,
                 palettes: &Palettes,
                 mut pairs: &mut GfxImageMap,
) -> Result<Handle<Image>, Error> {

    let my_span = info_span!("gfx::compute_image", name = "function").entered();

    if gfx_material.palette >= palettes.len() {
        return Err(Error::NoSuch("palette".into()));
    }
    let mut hasher = DefaultHasher::new();
    gfx_material.pal_map.hash(&mut hasher);
    gfx_material.palette.hash(&mut hasher);
    // let drawing = &state.draw_state;
    // drawing.fill_pat.inspect(|fill_pat| {
    //     fill_pat.hash(&mut hasher);
    // });
    let hash = hasher.finish();
    let gfx_id = gfx_handle.id();
    let palette = palettes.get_pal(gfx_material.palette)?;
    let image_handle: Option<Handle<Image>> = pairs.get(&gfx_id).and_then(|gfx_image| {
        gfx_image.get(&hash).inspect(|handle| {
            let my_span = info_span!("gfx::compute_image", name = "update image").entered();
            let gfx = gfxs.get(gfx_id);
            // Update existing image.
            if let Some((gfx, mut image)) = gfx.zip(images.get_mut(*handle)) {
                trace!("updating image for gfx {}", gfx_id);
                gfx.write_bytes(
                    &mut image.data,
                    |i, _, bytes| {
                    gfx_material.pal_map.write_color(&palette.data, i, bytes);
                });
            }
        }).cloned()
    });
    let image_handle: Result<Handle<Image>, Error> = image_handle.map(Ok).unwrap_or_else(|| {
        let my_span = info_span!("gfx::compute_image", name = "create image").entered();
        let gfx = gfxs.get(gfx_handle)
            .ok_or(Error::NoSuch("gfx image".into()))?;
        trace!("creating image for gfx {}", gfx_id);
        let image = images.add(gfx.try_to_image(|i, n, bytes| {
            // trace!("pixel {} writing color {}", n, i);
            gfx_material.pal_map.write_color(&palette.data, i, bytes)
        })?);
        // Update or add image to the map.
        pairs.entry(gfx_id)
                .and_modify(|gfx_image| { gfx_image.insert(hash, image.clone()); } )
            .or_insert_with(|| GfxImage::new(hash, image.clone()));
        Ok(image)
    });
    image_handle
}

// Informed from Bevy's Sprite::compute_slices_on_asset_event.
fn compute_image_on_asset_event(
    mut commands: Commands,
    mut events: EventReader<AssetEvent<Gfx>>,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    state: Res<Pico8State>,
    palettes: Res<Palettes>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>)>,
    mut pairs: ResMut<GfxImageMap>,
    mut update_ids: Local<Vec<Entity>>,
    mut update_images: Local<VecDeque<Handle<Image>>>,
    // mut update_images: Local<Vec<(Entity, Handle<Image>)>>,
) {
    // We store the asset ids of added/modified image assets.
    let added_handles: bevy::utils::HashSet<_> = events
        .read()
        .filter_map(|e| match e {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => Some(*id),
            AssetEvent::Removed { id } => {
                pairs.remove(id);
                None
            }
            _ => None,
        })
        .collect();
    if added_handles.is_empty() {
        return;
    }
    for (id, gfx_sprite, sprite) in &sprites {
        if !added_handles.contains(&gfx_sprite.image.id()) {
            continue;
        }

        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            continue;
        };
        let image_handle = compute_image(&gfx_sprite.image,
                                         &gfx_material,
                                         &gfxs,
                                         &mut images,
                                         &palettes,
                                         &mut pairs);
        match image_handle {
            Ok(image) => {
                match sprite {
                    Some(sprite) => {
                        if sprite.image != image {
                            // trace!("updating existant sprite on {}", id);
                            // sprite.image = image;
                            update_ids.push(id);
                            update_images.push_back(image);
                        }
                    }
                    None => {
                        // trace!("inserting new sprite into {}", id);
                        commands.entity(id)
                            .insert(Sprite::from_image(image));
                    }
                }
            }
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
    // Try not to trigger a sprite change if we don't have to.
    let mut iter = sprites.iter_many_mut(update_ids.iter());
    while let Some((_, _, mut sprite)) = iter.fetch_next() {
        match sprite {
            Some(mut sprite) => {
                sprite.image = update_images.pop_front().unwrap();
            }
            _ => unreachable!()
        }

    }
}

fn compute_image_on_gfx_sprite_change(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    gfxs: Res<Assets<Gfx>>,
    gfx_materials: Res<Assets<GfxMaterial>>,
    state: Res<Pico8State>,
    palettes: Res<Palettes>,
    mut sprites: Query<(Entity, &GfxSprite, Option<&mut Sprite>), Changed<GfxSprite>>,
    mut pairs: ResMut<GfxImageMap>,
) {
    for (id, gfx_sprite, mut sprite) in &mut sprites {
        let Some(gfx_material) = gfx_materials.get(&gfx_sprite.material) else {
            continue;
        };
        let image_handle = compute_image(&gfx_sprite.image,
                                         &gfx_material,
                                         &gfxs,
                                         &mut images,
                                         &palettes,
                                         &mut pairs);
        match image_handle {
            Ok(image) => {
                match sprite {
                    Some(mut sprite) => {
                        trace!("updating existant sprite on {}", id);
                        sprite.image = image;
                    }
                    None => {
                        trace!("inserting new sprite into {}", id);
                        commands.entity(id)
                            .insert(Sprite::from_image(image));
                    }
                }
            }
            Err(e) => {
                warn!("Unable to update gfx {}: {e}", gfx_sprite.image.id());
            }
        }
    }
}


fn create_sprite(mut query: Query<(Entity, &GfxSprite), Without<Sprite>>,
                 gfxs: Res<Assets<Gfx>>,
                 state: Res<Pico8State>,
                 mut images: ResMut<Assets<Image>>,
                 palettes: Res<Palettes>,  // for palettes, ugh. TODO: Move palettes elsewhere.
                 mut commands: Commands,
) {
    let palette = state.palette;
    let Ok(palette) = palettes.get_pal(state.palette) else {
        return;
    };
    for (id, gfx_sprite) in &mut query {
        info!("create_sprite {id}");

        let Some(gfx) = gfxs.get(&gfx_sprite.image) else { continue; };
        // TODO: Update existing image if it's the right size instead of recreating it.
        match gfx.try_to_image(|i, _, bytes| {
            state.pal_map.write_color(&palette.data, i, bytes)
        }) {
            Ok(image) => {
                commands.entity(id)
                    .insert(Sprite {
                        image: images.add(image),
                        ..default()
                    });
            }
            Err(e) => {
                error!("Could not write gfx to image: {e}");
            }
        }
    }
}

/// An indexed image using `N`-bit palette with color index `T`.
#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx<const N: usize = 4, T: TypePath + Send + Sync + BitStore = u8> {
    #[reflect(ignore)]
    pub data: BitVec<T, Lsb0>,
    pub width: usize,
    pub height: usize,
}

impl<T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy> Gfx<1, T> {
    pub fn mirror_horizontal(mut self) -> Self {
        for elem in self.data.chunks_mut(self.width) {
            elem.reverse();
        }
        self
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PngError {
    #[error("Not an indexed png")]
    NotIndexed,
    #[error("Unexpected bit-depth of {expected} but was {actual}")]
    BitDepth { expected: u8, actual: u8 },
    #[error("Cannot convert bit-depth for pixel {pixel_index} with value {pixel_value}")]
    BitDepthConversion { pixel_index: usize, pixel_value: u8 },
}

impl<const N: usize> Gfx<N, u8> {

    pub fn from_png(bytes: &[u8], mut palette: Option<&mut Palette>) -> Result<Self, png::DecodingError> {
        let cursor = std::io::Cursor::new(bytes);
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        if let Some(ref mut palette) = &mut palette {
            info.palette.as_ref().inspect(|png_palette| {
                let mut colors = png_palette.chunks(3);
                let mut data = vec![[0x00, 0x00, 0x00, 0xff]; colors.len()];
                for (i, rgb) in colors.enumerate() {
                    data[i][0..3].copy_from_slice(rgb);
                }
                palette.data = data;
            });
        }
        let dest_bit_depth = N;
        if info.color_type == png::ColorType::Indexed {
            let mut buf = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut buf).unwrap();
            let width = info.width as usize;
            let height = info.height as usize;
            let src_bit_depth = info.bit_depth as usize;
            // eprintln!("buf {:?}", &buf);
            let mut data = BitVec::from_vec(buf);
            if src_bit_depth > dest_bit_depth {
                // Let's try and truncate the index to fit what we're using.

                // i is the pixel index.
                for i in 0..width * height {
                    let a = i * src_bit_depth;
                    let b = a + dest_bit_depth;
                    let c = i * dest_bit_depth;
                    // Check the high bits of the pixel before overwriting them.
                    if data[a + dest_bit_depth..(a + src_bit_depth - dest_bit_depth)].any() {
                        let mut pixel_value: u8 = 0;
                        pixel_value
                            .view_bits_mut::<Lsb0>()
                            .copy_from_bitslice(&data[a..a + src_bit_depth]);
                        return Err(png::DecodingError::IoError(std::io::Error::other(
                            PngError::BitDepthConversion {
                                pixel_index: i,
                                pixel_value,
                            },
                        )));
                    }
                    data.copy_within(a..b, c);
                }
                data.truncate(width * height * dest_bit_depth);
            } else if src_bit_depth < dest_bit_depth {
                // This works similarly to the first, but it will start with the
                // last pixel to avoid overwriting the source information.
                todo!("Convert to a bigger bit depth");
            }
            Ok(Gfx {
                data,
                width,
                height,
            })
        } else {
            Err(png::DecodingError::IoError(std::io::Error::other(
                PngError::NotIndexed,
            )))
        }
    }
}

impl<
        const N: usize,
        T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy,
    > Gfx<N, T>
{
    /// Create an indexed image.
    pub fn new(width: usize, height: usize) -> Self {
        Gfx {
            data: BitVec::<T, Lsb0>::repeat(false, width * height * N),
            width,
            height,
        }
    }

    pub fn from_vec(width: usize, height: usize, vec: Vec<T>) -> Self {
        let gfx = Gfx {
            data: BitVec::<T, Lsb0>::from_vec(vec),
            width,
            height,
        };
        assert!(width * height * N <= gfx.data.len());
        gfx
    }

    /// Get a color index.
    pub fn get(&self, x: usize, y: usize) -> Option<T> {
        let start = x * N + y * N * self.width;
        self.data.get(start..start + N).map(|slice| {
            let mut result = T::default();
            let bits = result.view_bits_mut::<Lsb0>();
            bits[0..N].copy_from_bitslice(slice);
            result
        })
    }

    /// Set a color index. Returns true if set.
    pub fn set(&mut self, x: usize, y: usize, color_index: T) -> bool {
        let bits = color_index.view_bits::<Lsb0>();
        let start = x * N + y * N * self.width;
        self.data
            .get_mut(start..start + N)
            .map(|slice| {
                slice.copy_from_bitslice(&bits[0..N]);
                true
            })
            .unwrap_or(false)
    }

    /// Write pixel data.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn write_bytes(
        &self,
        pixel_bytes: &mut [u8],
        mut write_color: impl FnMut(T, usize, &mut [u8]),
    ) {
        // let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        let mut chunks = self.data.chunks_exact(N);
        assert!(chunks.len() >= self.width * self.height,
                "cannot write full {}x{} gfx to image only has {} pixels", self.width, self.height, chunks.len());
        for (i, pixel) in chunks.enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..N].copy_from_bitslice(pixel);
            write_color(color_index, i, &mut pixel_bytes[i * 4..(i + 1) * 4]);
        }
    }

    /// Create an image.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn try_to_image<E>(
        &self,
        mut write_color: impl FnMut(T, usize, &mut [u8]) -> Result<(), E>,
    ) -> Result<Image, E> {
        let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        for (i, pixel) in self.data.chunks_exact(N).enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..N].copy_from_bitslice(pixel);
            write_color(color_index, i, &mut pixel_bytes[i * 4..(i + 1) * 4])?;
        }
        let mut image = Image::new(
            Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                ..default()
            },
            TextureDimension::D2,
            pixel_bytes,
            TextureFormat::Rgba8UnormSrgb,
            // Must have main world, not sure why.
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        Ok(image)
    }

    pub fn to_image(&self, mut write_color: impl FnMut(T, usize, &mut [u8])) -> Image {
        self.try_to_image::<Error>(move |color_index, pixel_index, pixel_bytes| {
            write_color(color_index, pixel_index, pixel_bytes);
            Ok(())
        })
        .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const BIT1_PALETTE: [[u8; 4]; 2] = [[0x00, 0x00, 0x00, 0xff], [0xff, 0xff, 0xff, 0xff]];

    #[test]
    fn ex0() {
        let mut a = Gfx::<4>::new(8, 8);
        assert_eq!(0, a.get(0, 0).unwrap());
        a.set(0, 0, 15);
        assert_eq!(15, a.get(0, 0).unwrap());
    }

    #[test]
    fn create_image() {
        let mut a = Gfx::<4>::new(8, 8);
        assert_eq!(0, a.get(0, 0).unwrap());
        a.set(0, 0, 15);
        let _ = a.to_image(|_, _, _| {});
    }

    #[rustfmt::skip]
    #[test]
    fn create_1bit_image() {
        let a = Gfx::<1>::from_vec(
            8,
            8,
            vec![
                0b00000001,
                0b00000010,
                0b00000100,
                0b00001000,
                0b00010000,
                0b00100000,
                0b01000000,
                0b10000000,
            ],
        );
        let image = a.to_image(|i, _, pixel_bytes| {
            pixel_bytes.copy_from_slice(&BIT1_PALETTE[i as usize]);
        });
        let color: Srgba = image.get_color_at(0, 0).unwrap().into();
        assert_eq!(color, Srgba::WHITE);
        let color: Srgba = image.get_color_at(0, 7).unwrap().into();
        assert_eq!(color, Srgba::BLACK);
    }
}
