//! Pixel plots for Pico-8 shape primitives. Colors are baked as sRGB bytes
//! (sprite tint round-trips through linear and lands 1/255 dark).
//!
//! Drawing walks the CPU buffer directly. Outlines use inherent `for_each`.
//! Fills walk `Fill::fill` spans into `hline`.

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use nano9_raster::{Circle, Ellipse, Fill, Inclusive, Line};

const BPP: usize = 4;

pub struct Raster {
    pub image: Image,
    pub pen: [u8; 4],
}

/// Inclusive line. `Line` is half-open; `.inclusive()` yields both endpoints.
pub(crate) fn bresenham_inclusive(
    start: (isize, isize),
    end: (isize, isize),
) -> impl Iterator<Item = (isize, isize)> {
    Line::new(start, end).inclusive()
}

impl Raster {
    pub fn new(size: UVec2, pen: [u8; 4]) -> Self {
        let mut image = Image::new_fill(
            Extent3d {
                width: size.x.max(1),
                height: size.y.max(1),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0u8, 0u8, 0u8, 0u8],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        Self { image, pen }
    }

    fn buf(&mut self) -> (&mut [u8], UVec2) {
        let size = self.image.size();
        let data = self
            .image
            .data
            .as_mut()
            .expect("raster image has CPU data")
            .as_mut_slice();
        (data, size)
    }

    #[allow(dead_code)]
    pub fn plot(&mut self, x: i32, y: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        put(data, size, x, y, pen);
    }

    pub fn hline(&mut self, x0: i32, x1: i32, y: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        fill_hline(data, size, x0, x1, y, pen);
    }

    pub fn vline(&mut self, y0: i32, y1: i32, x: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        fill_vline(data, size, x, y0, y1, pen);
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        if y0 == y1 {
            self.hline(x0, x1, y0);
            return;
        }
        if x0 == x1 {
            self.vline(y0, y1, x0);
            return;
        }
        let pen = self.pen;
        let (data, size) = self.buf();
        for (x, y) in bresenham_inclusive((x0 as isize, y0 as isize), (x1 as isize, y1 as isize)) {
            put(data, size, x as i32, y as i32, pen);
        }
    }

    pub fn circ(&mut self, ox: i32, oy: i32, r: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        Circle::new((ox as isize, oy as isize), r as isize).for_each(|(x, y)| {
            put(data, size, x as i32, y as i32, pen);
        });
    }

    pub fn circfill(&mut self, ox: i32, oy: i32, r: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        Circle::new((ox as isize, oy as isize), r as isize)
            .fill()
            .for_each(|span| {
                fill_hline(
                    data,
                    size,
                    span.x0 as i32,
                    span.x1 as i32,
                    span.y as i32,
                    pen,
                );
            });
    }

    pub fn oval(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        Ellipse::from_rect((x0 as isize, y0 as isize), (x1 as isize, y1 as isize)).for_each(
            |(x, y)| {
                put_clip(data, size, x as i32, y as i32, pen);
            },
        );
    }

    pub fn ovalfill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let pen = self.pen;
        let (data, size) = self.buf();
        Ellipse::from_rect((x0 as isize, y0 as isize), (x1 as isize, y1 as isize))
            .fill()
            .for_each(|span| {
                fill_hline_clip(
                    data,
                    size,
                    span.x0 as i32,
                    span.x1 as i32,
                    span.y as i32,
                    pen,
                );
            });
    }
}

impl From<Raster> for Image {
    fn from(raster: Raster) -> Self {
        raster.image
    }
}

#[inline(always)]
fn offset(size: UVec2, x: i32, y: i32) -> usize {
    ((y as u32 * size.x + x as u32) as usize) * BPP
}

#[inline(always)]
fn put(data: &mut [u8], size: UVec2, x: i32, y: i32, pen: [u8; 4]) {
    assert!(
        x >= 0 && y >= 0 && (x as u32) < size.x && (y as u32) < size.y,
        "plot ({x}, {y}) out of raster {size}"
    );
    let i = offset(size, x, y);
    data[i..i + BPP].copy_from_slice(&pen);
}

#[inline(always)]
fn put_clip(data: &mut [u8], size: UVec2, x: i32, y: i32, pen: [u8; 4]) {
    if x < 0 || y < 0 || (x as u32) >= size.x || (y as u32) >= size.y {
        return;
    }
    let i = offset(size, x, y);
    data[i..i + BPP].copy_from_slice(&pen);
}

#[inline(always)]
fn fill_hline_clip(data: &mut [u8], size: UVec2, x0: i32, x1: i32, y: i32, pen: [u8; 4]) {
    if y < 0 || (y as u32) >= size.y {
        return;
    }
    let (mut lo, mut hi) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    lo = lo.max(0);
    hi = hi.min(size.x as i32 - 1);
    if lo <= hi {
        fill_hline(data, size, lo, hi, y, pen);
    }
}

#[inline(always)]
fn fill_hline(data: &mut [u8], size: UVec2, x0: i32, x1: i32, y: i32, pen: [u8; 4]) {
    let (lo, hi) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    assert!(
        y >= 0 && (y as u32) < size.y && lo >= 0 && (hi as u32) < size.x,
        "hline ({lo}..={hi}, {y}) out of raster {size}"
    );
    let start = offset(size, lo, y);
    let row = &mut data[start..start + ((hi - lo + 1) as usize) * BPP];
    for px in row.chunks_exact_mut(BPP) {
        px.copy_from_slice(&pen);
    }
}

#[inline(always)]
fn fill_vline(data: &mut [u8], size: UVec2, x: i32, y0: i32, y1: i32, pen: [u8; 4]) {
    let (lo, hi) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
    assert!(
        x >= 0 && (x as u32) < size.x && lo >= 0 && (hi as u32) < size.y,
        "vline ({x}, {lo}..={hi}) out of raster {size}"
    );
    let stride = size.x as usize * BPP;
    let mut i = offset(size, x, lo);
    for _ in lo..=hi {
        data[i..i + BPP].copy_from_slice(&pen);
        i += stride;
    }
}
