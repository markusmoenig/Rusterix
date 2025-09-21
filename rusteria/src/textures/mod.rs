pub use crate::Value;
use image::{ImageBuffer, Rgb, RgbImage};
use std::path::PathBuf;
pub mod patterns;

// Patterns taken from https://github.com/tuxalin/procedural-tileable-shaders

use rayon::prelude::*;

#[derive(Clone)]
pub struct TexStorage {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Value>, // length = width * height
}

impl TexStorage {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![Value::zero(); width * height],
        }
    }

    /// Nearest-neighbor sample of a normalized UV carried in a Value (Vec3<f32>), using x/y as uv.
    #[inline]
    pub fn sample(&self, uv: Value) -> Value {
        let (x, y) = self.sample_index(uv);
        unsafe { *self.data.get_unchecked(y * self.width + x) }
    }

    /// Directly set a pixel by integer coordinates (no bounds check).
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, value: Value) {
        let idx = y * self.width + x;
        self.data[idx] = value;
    }

    /// Parallel iterate over all pixels with a per-row state initializer.
    /// `init` creates state once per processed row (Rayon work item).
    /// `f` shades one pixel using that reusable state and returns the pixel Value.
    pub fn par_iterate_with<Init, F, S>(&mut self, init: Init, f: F)
    where
        Init: Fn() -> S + Sync,
        F: Fn(&mut S, usize, usize, Value) -> Value + Sync,
        S: Send,
    {
        let inv_w = 1.0f32 / self.width as f32;
        let inv_h = 1.0f32 / self.height as f32;
        self.data
            .par_chunks_mut(self.width)
            .enumerate()
            .for_each(|(y, row)| {
                let mut state = init(); // once per row
                let v = y as f32 * inv_h;
                for x in 0..self.width {
                    let u = x as f32 * inv_w;
                    let uv = Value::new(u, v, 0.0);
                    row[x] = f(&mut state, x, y, uv);
                }
            });
    }

    /// Construct a TexStorage by decoding a PNG from bytes.
    pub fn from_png_bytes(bytes: &[u8]) -> image::ImageResult<Self> {
        let img = image::load_from_memory(bytes)?.to_rgb8();
        let width = img.width() as usize;
        let height = img.height() as usize;
        let mut tex = TexStorage::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let p = img.get_pixel(x as u32, y as u32);
                tex.data[y * width + x] = Value::new(
                    p[0] as f32 / 255.0,
                    p[1] as f32 / 255.0,
                    p[2] as f32 / 255.0,
                );
            }
        }
        Ok(tex)
    }

    /// Save the texture to a PNG file using only 3 channels (RGB).
    pub fn save_png(&self, path: &PathBuf) -> image::ImageResult<()> {
        let mut img: RgbImage = ImageBuffer::new(self.width as u32, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let v = self.data[idx];
                // Clamp and convert Value (Vec3<f32>) to RGB u8
                let r = (v.x.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (v.y.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (v.z.clamp(0.0, 1.0) * 255.0) as u8;
                img.put_pixel(x as u32, y as u32, Rgb([r, g, b]));
            }
        }
        img.save(path)
    }

    /// Load the texture from a 3-channel (RGB) PNG file.
    pub fn load_png(&mut self, path: &PathBuf) -> image::ImageResult<()> {
        let img = image::open(path)?.to_rgb8();
        assert_eq!(img.width() as usize, self.width);
        assert_eq!(img.height() as usize, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let px = img.get_pixel(x as u32, y as u32);
                self.data[idx] = Value::new(
                    px[0] as f32 / 255.0,
                    px[1] as f32 / 255.0,
                    px[2] as f32 / 255.0,
                );
            }
        }
        Ok(())
    }

    #[inline]
    fn rem_i32(&self, a: i32, m: i32) -> i32 {
        let r = a % m;
        if r < 0 { r + m } else { r }
    }

    #[inline]
    fn sample_index(&self, uv: Value) -> (usize, usize) {
        let mut u = uv.x;
        let mut v = uv.y;

        u = u - u.floor();
        v = v - v.floor();

        let mut x = (u * self.width as f32).floor() as i32;
        let mut y = (v * self.height as f32).floor() as i32;

        x = self.rem_i32(x, self.width as i32);
        y = self.rem_i32(y, self.height as i32);

        (x as usize, y as usize)
    }
}
