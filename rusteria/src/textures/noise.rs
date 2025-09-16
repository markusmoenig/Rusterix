use vek::{Vec2, Vec4};

use crate::textures::interpolate::noise_interpolate2;
use crate::textures::multi_hash::better_hash2d_cell;

#[inline(always)]
fn mixf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 2D Value noise (tileable if `scale` components are integers >= 2).
/// - `pos`: UV in any range (we wrap by `scale` internally per the GLSL)
/// - `scale`: number of tiles along (x,y); must be integers for perfect tiling
/// - `seed`: scalar added into the hash domain
/// Returns noise in [-1, 1].
#[inline]
pub fn value_noise_2d(pos: Vec2<f32>, scale: Vec2<f32>, seed: f32) -> f32 {
    // pos *= scale
    let p = pos * scale;

    // i = floor(pos).xyxy + vec2(0,1).xxyy  => [ix, iy, ix+1, iy+1]
    let pf = p.floor();
    let mut i = Vec4::new(pf.x, pf.y, pf.x + 1.0, pf.y + 1.0);

    // f = pos - i.xy   => fractional part of pos within the cell
    let f = Vec2::new(p.x - i.x, p.y - i.y);

    // i = mod(i, scale.xyxy) + seed
    // Note: i.x,i.z wrap by scale.x; i.y,i.w wrap by scale.y
    i.x = i.x.rem_euclid(scale.x);
    i.y = i.y.rem_euclid(scale.y);
    i.z = i.z.rem_euclid(scale.x);
    i.w = i.w.rem_euclid(scale.y);
    i = i + Vec4::broadcast(seed);

    // Four corner hashes (a,b,c,d) in [0,1]
    // (You can swap to permute_hash2d(i) if you prefer that family.)
    let hash = better_hash2d_cell(i);
    let a = hash.x;
    let b = hash.y;
    let c = hash.z;
    let d = hash.w;

    // Hermite interpolation on the fractional coords
    let u = noise_interpolate2(f);

    // Bilinear blend using the Hermite-smoothed weights
    let value = mixf(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;

    // Map [0,1] -> [-1,1]
    value //* 2.0 - 1.0
}
