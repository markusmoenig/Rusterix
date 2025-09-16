// rusteria/src/textures/patterns.rs
//! Global pattern bank: compute-once tileable textures stored as `TexStorage`.
//! Call `ensure_patterns_initialized()` once at startup, or any accessor will lazily init.

use super::TexStorage;
use crate::Value;
use once_cell::sync::OnceCell;
use vek::{Vec2, Vec3};

/// Global storage of precomputed patterns.
static PATTERNS: OnceCell<Vec<TexStorage>> = OnceCell::new();

/// Returns true if patterns have already been computed and stored.
#[inline]
pub fn patterns_computed() -> bool {
    PATTERNS.get().is_some()
}

/// Ensure the global patterns vector is initialized. Safe to call multiple times.
pub fn ensure_patterns_initialized() {
    let _ = PATTERNS.get_or_init(|| build_patterns());
}

/// Get an immutable slice of all precomputed patterns. Lazily initializes on first call.
#[inline]
pub fn patterns() -> &'static [TexStorage] {
    PATTERNS.get_or_init(|| build_patterns()).as_slice()
}

/// Get a specific pattern by id. Panics if out of range. Lazily initializes.
#[inline]
pub fn pattern(id: usize) -> &'static TexStorage {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    &vec[id]
}

/// Internal: build the full set of patterns. Add more here as needed.
fn build_patterns() -> Vec<TexStorage> {
    let mut v = Vec::new();

    v.push(make_hash());
    v.push(make_noise());

    v
}

fn make_hash() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| hash3d(uv));
    tex
}

fn make_noise() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| {
        Vec3::broadcast(crate::textures::noise::value_noise_2d(
            uv.xy(),
            Vec2::new(20.0, 20.0),
            1.0,
        ))
    });
    tex
}

// Helpers

// #[inline(always)]
// fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
//     let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
//     t * t * (3.0 - 2.0 * t)
// }

// #[inline(always)]
// fn step(edge: f32, x: f32) -> f32 {
//     if x >= edge { 1.0 } else { 0.0 }
// }

/// PCG-style 3D hash from a 2D input
/// Returns a Value (x,y,z) each in [0, 1].
#[inline]
pub fn hash3d(uv: Value) -> Value {
    let sx = (uv.x * 8192.0) as u32;
    let sy = (uv.y * 8192.0) as u32;

    let mut vx = sx;
    let mut vy = sy;
    let mut vz = sx;

    const A: u32 = 1_664_525;
    const C: u32 = 1_013_904_223;
    vx = vx.wrapping_mul(A).wrapping_add(C);
    vy = vy.wrapping_mul(A).wrapping_add(C);
    vz = vz.wrapping_mul(A).wrapping_add(C);

    let (ox, oy, oz) = (vx, vy, vz);
    vx = vx.wrapping_add(oy.wrapping_mul(oz));
    vy = vy.wrapping_add(oz.wrapping_mul(ox));
    vz = vz.wrapping_add(ox.wrapping_mul(oy));

    vx ^= vx >> 16;
    vy ^= vy >> 16;
    vz ^= vz >> 16;

    vx = vx.wrapping_add(vy.wrapping_mul(vz));
    vy = vy.wrapping_add(vz.wrapping_mul(vx));
    vz = vz.wrapping_add(vx.wrapping_mul(vy));

    const INV_U32_MAX: f32 = 1.0 / 4_294_967_295.0;
    Value::new(
        vx as f32 * INV_U32_MAX,
        vy as f32 * INV_U32_MAX,
        vz as f32 * INV_U32_MAX,
    )
}
