pub use crate::Value;
pub mod interpolate;
pub mod multi_hash;
pub mod noise;
pub mod patterns;

// Patterns taken from https://github.com/tuxalin/procedural-tileable-shaders

/// Global texture size (square).
pub const TEX_SIZE: usize = 512;

use rayon::prelude::*;

pub struct TexStorage {
    pub data: Vec<Value>, // length = TEX_SIZE * TEX_SIZE
}

impl TexStorage {
    pub fn new() -> Self {
        Self {
            data: vec![Value::zero(); TEX_SIZE * TEX_SIZE],
        }
    }

    /// Nearest-neighbor sample of a normalized UV carried in a Value (Vec3<f32>), using x/y as uv.
    #[inline]
    pub fn sample(&self, uv: Value) -> Value {
        let (x, y) = sample_index(uv);
        unsafe { *self.data.get_unchecked(y * TEX_SIZE + x) }
    }

    /// Directly set a pixel by integer coordinates (no bounds check).
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, value: Value) {
        let idx = y * TEX_SIZE + x;
        self.data[idx] = value;
    }

    /// Fill the texture in parallel using a callback.
    pub fn compute_par<F>(&mut self, func: F)
    where
        F: Sync + Send + Fn(Value) -> Value,
    {
        let inv = 1.0f32 / TEX_SIZE as f32;
        self.data
            .par_chunks_mut(TEX_SIZE)
            .enumerate()
            .for_each(|(y, row)| {
                let v = y as f32 * inv;
                for x in 0..TEX_SIZE {
                    let u = x as f32 * inv;
                    // Pack UV into Value; z is available for extra params if desired
                    row[x] = func(Value::new(u, v, 0.0));
                }
            });
    }
}

#[inline]
fn rem_i32(a: i32, m: i32) -> i32 {
    let r = a % m;
    if r < 0 { r + m } else { r }
}

#[inline]
fn sample_index(uv: Value /* , tile: bool*/) -> (usize, usize) {
    let mut u = uv.x;
    let mut v = uv.y;

    u = u - u.floor();
    v = v - v.floor();

    let mut x = (u * TEX_SIZE as f32).floor() as i32;
    let mut y = (v * TEX_SIZE as f32).floor() as i32;

    x = rem_i32(x, TEX_SIZE as i32);
    y = rem_i32(y, TEX_SIZE as i32);

    (x as usize, y as usize)
}
