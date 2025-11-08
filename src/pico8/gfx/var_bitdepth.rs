use crate::pico8::*;
use bevy::{
    image::ImageSampler,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bitvec::{prelude::*, view::BitView};

/// An indexed image using `n`-bit palette with color index `T`.
#[derive(Asset, Debug, Reflect, Clone)]
pub struct Gfx<T: TypePath + Send + Sync + BitStore = u8> {
    #[reflect(ignore)]
    pub data: BitVec<T, Lsb0>,
    pub bitdepth: usize,
    pub width: usize,
    pub height: usize,
}

/// Find the number of bits required to describe the number of colors given.
#[allow(dead_code)]
fn bits_required(color_count: usize) -> Option<u32> {
    if color_count == 0 {
        return None;
    }
    Some(usize::BITS - (color_count - 1).leading_zeros())
}

impl Gfx<u8> {
    pub fn from_png(
        bytes: &[u8],
        mut palette: Option<&mut Palette>,
    ) -> Result<Self, png::DecodingError> {
        let cursor = std::io::Cursor::new(bytes);
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder.read_info()?;
        let info = reader.info();
        if let Some(palette) = &mut palette {
            if let Some(pal) = Palette::from_png_palette_info(&info) {
                palette.data = pal.data;
            }
        }
        // Find highest power of 2 in the length of colors.
        let png_bit_depth: usize = info.bit_depth as u8 as usize;
        // let required_bit_depth: usize = bits_required(todo!("color count"));
        // let dest_bit_depth = png_bit_depth.min(required_bit_depth);
        let dest_bit_depth = png_bit_depth;
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
                bitdepth: dest_bit_depth,
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

impl<T: TypePath + Send + Sync + Default + BitView<Store = T> + BitStore + Copy> Gfx<T> {
    /// Create an indexed image.
    pub fn new(bitdepth: usize, width: usize, height: usize) -> Self {
        Gfx {
            data: BitVec::<T, Lsb0>::repeat(false, width * height * bitdepth),
            bitdepth,
            width,
            height,
        }
    }

    pub fn from_vec(bitdepth: usize, width: usize, height: usize, vec: Vec<T>) -> Self {
        let gfx = Gfx {
            data: BitVec::<T, Lsb0>::from_vec(vec),
            bitdepth,
            width,
            height,
        };
        assert!(width * height * bitdepth <= gfx.data.len());
        gfx
    }

    /// Get a color index.
    pub fn get(&self, x: usize, y: usize) -> Option<T> {
        let n = self.bitdepth;
        let start = x * n + y * n * self.width;
        self.data.get(start..start + n).map(|slice| {
            let mut result = T::default();
            let bits = result.view_bits_mut::<Lsb0>();
            bits[0..n].copy_from_bitslice(slice);
            result
        })
    }

    /// Set a color index. Returns true if set.
    pub fn set(&mut self, x: usize, y: usize, color_index: T) -> bool {
        let n = self.bitdepth;
        let bits = color_index.view_bits::<Lsb0>();
        let start = x * n + y * n * self.width;
        self.data
            .get_mut(start..start + n)
            .map(|slice| {
                slice.copy_from_bitslice(&bits[0..n]);
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
        let n = self.bitdepth;
        // let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        let chunks = self.data.chunks_exact(n);
        assert!(
            chunks.len() >= self.width * self.height,
            "cannot write full {}x{} gfx to image only has {} pixels",
            self.width,
            self.height,
            chunks.len()
        );
        for (i, pixel) in chunks.enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..n].copy_from_bitslice(pixel);
            write_color(color_index, i, &mut pixel_bytes[i * 4..(i + 1) * 4]);
        }
    }

    /// Try to write pixel data.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn try_write_bytes<E>(
        &self,
        pixel_bytes: &mut [u8],
        mut write_color: impl FnMut(T, usize, &mut [u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        let n = self.bitdepth;
        // let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        let chunks = self.data.chunks_exact(n);
        assert!(
            chunks.len() >= self.width * self.height,
            "cannot write full {}x{} gfx to image only has {} pixels",
            self.width,
            self.height,
            chunks.len()
        );
        for (i, pixel) in chunks.enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..n].copy_from_bitslice(pixel);
            write_color(color_index, i, &mut pixel_bytes[i * 4..(i + 1) * 4])?;
        }
        Ok(())
    }

    /// Create an image.
    ///
    /// The `write_color` function accepts a color_index and the pixel_index and
    /// writes a Srgba set of u8 pixels.
    pub fn try_to_image<E>(
        &self,
        mut write_color: impl FnMut(T, usize, &mut [u8]) -> Result<(), E>,
    ) -> Result<Image, E> {
        let n = self.bitdepth;
        assert!(n > 0);
        let mut pixel_bytes = vec![0x00; self.width * self.height * 4];
        let mut color_index = T::default();
        for (i, pixel) in self.data.chunks_exact(n).enumerate() {
            color_index.view_bits_mut::<Lsb0>()[0..n].copy_from_bitslice(pixel);
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
    #[test]
    fn test_highest_bit_index() {
        assert_eq!(bits_required(16), Some(4));
        assert_eq!(bits_required(17), Some(5));
        assert_eq!(bits_required(4), Some(2));
        assert_eq!(bits_required(5), Some(3));
        assert_eq!(bits_required(6), Some(3));
        assert_eq!(bits_required(7), Some(3));
        assert_eq!(bits_required(8), Some(3));
        assert_eq!(bits_required(9), Some(4));
        assert_eq!(bits_required(2), Some(1));
    }
}
