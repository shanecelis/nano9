use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bitvec::prelude::*;

#[derive(Debug, Deref, DerefMut, Hash, PartialEq, Eq, Clone, Copy, Reflect, Default)]
pub struct FillPat {
    #[reflect(ignore)]
    pub data: BitArray<[u16; 1], Msb0>,
}

impl FillPat {
    /// Get a fill.
    pub fn get(&self, x: usize, y: usize) -> bool {
        self.data
            .get((x % 4) + (y % 4) * 4)
            .as_deref()
            .copied()
            .unwrap_or(false)
    }

    /// Set fill.
    pub fn set(&mut self, x: usize, y: usize, value: bool) {
        self.data.set((x % 4) + (y % 4) * 4, value);
    }

    /// Create an image.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn try_to_image<E>(
        &self,
        width: usize,
        height: usize,
        mut write_color: impl FnMut(bool, usize, &mut [u8]) -> Result<(), E>,
    ) -> Result<Image, E> {
        let mut pixel_bytes = vec![0x00; width * height * 4];
        let mut i = 0;
        for y in 0..height {
            for x in 0..width {
                let pat_bit = self.get(x, y);
                write_color(pat_bit, i, &mut pixel_bytes[i * 4..(i + 1) * 4])?;
                i += 1;
            }
        }
        let mut image = Image::new(
            Extent3d {
                width: width as u32,
                height: height as u32,
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
}

impl From<u16> for FillPat {
    fn from(x: u16) -> Self {
        let mut p = FillPat::default();
        p.data.copy_from_bitslice(x.view_bits());
        p
    }
}

impl From<FillPat> for u16 {
    fn from(p: FillPat) -> u16 {
        let mut x: u16 = 0;
        x.view_bits_mut().copy_from_bitslice(&p.data);
        x
    }
}
