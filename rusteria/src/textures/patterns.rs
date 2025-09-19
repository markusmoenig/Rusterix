// rusteria/src/textures/patterns.rs
//! Global pattern bank: compute-once tileable textures stored as `TexStorage`.
//! Call `ensure_patterns_initialized()` once at startup, or any accessor will lazily init.

use super::TexStorage;
use crate::Value;
use once_cell::sync::OnceCell;
use vek::{Mat2, Vec2, Vec3};

/// Global storage of precomputed patterns.
static PATTERNS: OnceCell<Vec<TexStorage>> = OnceCell::new();

static TILING: f32 = 10.0;

/// Enum of all available patterns, matches the build order in `build_patterns()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Hash,
    Value,
    FbmValue,
    FbmValueTurbulence,
    FbmValueRidge,
    Gradient,
    FbmGradient,
    FbmGradientTurbulence,
    FbmGradientRidge,
    Perlin,
    FbmPerlin,
    FbmPerlinTurbulence,
    FbmPerlinRidge,
    Voronoi,
    Cellular,
    Bricks,
}

impl PatternKind {
    pub fn to_index(self) -> usize {
        match self {
            PatternKind::Hash => 0,
            PatternKind::Value => 1,
            PatternKind::FbmValue => 2,
            PatternKind::FbmValueTurbulence => 3,
            PatternKind::FbmValueRidge => 4,
            PatternKind::Gradient => 5,
            PatternKind::FbmGradient => 6,
            PatternKind::FbmGradientTurbulence => 7,
            PatternKind::FbmGradientRidge => 8,
            PatternKind::Perlin => 9,
            PatternKind::FbmPerlin => 10,
            PatternKind::FbmPerlinTurbulence => 11,
            PatternKind::FbmPerlinRidge => 12,
            PatternKind::Voronoi => 13,
            PatternKind::Cellular => 14,
            PatternKind::Bricks => 15,
        }
    }

    pub fn from_index(i: usize) -> Option<Self> {
        Some(match i {
            0 => PatternKind::Hash,
            1 => PatternKind::Value,
            2 => PatternKind::FbmValue,
            3 => PatternKind::FbmValueTurbulence,
            4 => PatternKind::FbmValueRidge,
            5 => PatternKind::Gradient,
            6 => PatternKind::FbmGradient,
            7 => PatternKind::FbmGradientTurbulence,
            8 => PatternKind::FbmGradientRidge,
            9 => PatternKind::Perlin,
            10 => PatternKind::FbmPerlin,
            11 => PatternKind::FbmPerlinTurbulence,
            12 => PatternKind::FbmPerlinRidge,
            13 => PatternKind::Voronoi,
            14 => PatternKind::Cellular,
            15 => PatternKind::Bricks,
            _ => return None,
        })
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "hash" => Some(PatternKind::Hash),
            "value" => Some(PatternKind::Value),
            "fbm_value" => Some(PatternKind::FbmValue),
            "fbm_value_turbulence" => Some(PatternKind::FbmValueTurbulence),
            "fbm_value_ridge" => Some(PatternKind::FbmValueRidge),
            "gradient" => Some(PatternKind::Gradient),
            "fbm_gradient" => Some(PatternKind::FbmGradient),
            "fbm_gradient_turbulence" => Some(PatternKind::FbmGradientTurbulence),
            "fbm_gradient_ridge" => Some(PatternKind::FbmGradientRidge),
            "perlin" => Some(PatternKind::Perlin),
            "fbm_perlin" => Some(PatternKind::FbmPerlin),
            "fbm_perlin_turbulence" => Some(PatternKind::FbmPerlinTurbulence),
            "fbm_perlin_ridge" => Some(PatternKind::FbmPerlinRidge),
            "voronoi" => Some(PatternKind::Voronoi),
            "cellular" => Some(PatternKind::Cellular),
            "bricks" => Some(PatternKind::Bricks),
            _ => None,
        }
    }
}

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

/// Get a specific pattern by id. Panics if out of range.
#[inline]
pub fn pattern(id: usize) -> &'static TexStorage {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    &vec[id]
}

/// Get a specific pattern by id.
pub fn pattern_safe(id: usize) -> Option<&'static TexStorage> {
    let vec = PATTERNS.get_or_init(|| build_patterns());
    vec.get(id)
}

/// Internal: build the full set of patterns. Add more here as needed.
fn build_patterns() -> Vec<TexStorage> {
    let mut v = Vec::new();

    v.push(make_hash());
    v.push(make_noise());
    v.push(make_fbm_value());
    v.push(make_fbm_value_turbulence());
    v.push(make_fbm_value_ridge());
    v.push(make_gradient_noise());
    v.push(make_fbm_gradient());
    v.push(make_fbm_gradient_turbulence());
    v.push(make_fbm_gradient_ridge());
    v.push(make_perlin_noise());
    v.push(make_fbm_perlin());
    v.push(make_fbm_perlin_turbulence());
    v.push(make_fbm_perlin_ridge());
    v.push(make_voronoi_noise());
    v.push(make_cellular());
    v.push(make_bricks());

    v
}

fn make_hash() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| hash3d(uv));
    tex
}

fn make_noise() -> TexStorage {
    make_basic_noise(crate::textures::noise::value_noise_2d as NoiseFn)
}

fn make_gradient_noise() -> TexStorage {
    make_basic_noise(crate::textures::noise::gradient_noise_2d as NoiseFn)
}

fn make_perlin_noise() -> TexStorage {
    make_basic_noise(crate::textures::noise::perlin_noise_2d as NoiseFn)
}

fn make_fbm_gradient() -> TexStorage {
    make_fbm_noise(crate::textures::noise::gradient_noise_2d as NoiseFn)
}

fn make_fbm_gradient_turbulence() -> TexStorage {
    make_fbm_turbulence_noise(crate::textures::noise::gradient_noise_2d as NoiseFn)
}

fn make_fbm_gradient_ridge() -> TexStorage {
    make_fbm_ridge_noise(crate::textures::noise::gradient_noise_2d as NoiseFn)
}

fn make_fbm_value() -> TexStorage {
    make_fbm_noise(crate::textures::noise::value_noise_2d as NoiseFn)
}

fn make_fbm_value_turbulence() -> TexStorage {
    make_fbm_turbulence_noise(crate::textures::noise::value_noise_2d as NoiseFn)
}

fn make_fbm_value_ridge() -> TexStorage {
    make_fbm_ridge_noise(crate::textures::noise::value_noise_2d as NoiseFn)
}

fn make_fbm_perlin() -> TexStorage {
    make_fbm_noise(crate::textures::noise::perlin_noise_2d as NoiseFn)
}

fn make_fbm_perlin_turbulence() -> TexStorage {
    make_fbm_turbulence_noise(crate::textures::noise::perlin_noise_2d as NoiseFn)
}

fn make_fbm_perlin_ridge() -> TexStorage {
    make_fbm_ridge_noise(crate::textures::noise::perlin_noise_2d as NoiseFn)
}

fn make_voronoi_noise() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| {
        crate::textures::noise::voronoi_combined_2d(uv.xy(), Vec2::broadcast(TILING), 1.0, 0.0, 0.0)
    });
    tex
}

fn make_cellular() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| {
        let rc = crate::textures::noise::cellular_noise(uv.xy(), Vec2::broadcast(TILING), 1.0, 0.0);
        Vec3::new(rc.x, rc.y, 0.0)
    });
    tex
}

fn _make_cellular() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| {
        let rc = crate::textures::noise::metaballs(uv.xy(), Vec2::broadcast(TILING), 1.0, 0.0);
        Vec3::broadcast(rc)
    });
    tex
}

fn make_bricks() -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(|uv| bricks(uv));
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

/// Shared signature for scalar 2D noises
type NoiseFn = fn(Vec2<f32>, Vec2<f32>, f32) -> f32;

#[inline]
fn make_basic_noise(noise: NoiseFn) -> TexStorage {
    let mut tex = TexStorage::new();
    tex.compute_par(move |uv| Vec3::broadcast(noise(uv.xy(), Vec2::broadcast(TILING), 10.0)));
    tex
}

#[inline]
fn make_fbm_noise(noise: NoiseFn) -> TexStorage {
    let mut tex = TexStorage::new();
    const OCTAVES: u32 = 5;
    const LACUNARITY: f32 = 2.0;
    const GAIN: f32 = 0.5;
    tex.compute_par(move |uv| {
        let mut amp = 1.0f32;
        let mut freq = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for o in 0..OCTAVES {
            let n = noise(uv.xy() * freq, Vec2::broadcast(TILING), 10.0 + o as f32);
            sum += n * amp;
            norm += amp;
            freq *= LACUNARITY;
            amp *= GAIN;
        }
        Vec3::broadcast((sum / norm).clamp(0.0, 1.0))
    });
    tex
}

#[inline]
fn make_fbm_turbulence_noise(noise: NoiseFn) -> TexStorage {
    let mut tex = TexStorage::new();
    const OCTAVES: u32 = 5;
    const LACUNARITY: f32 = 2.0;
    const GAIN: f32 = 0.5;
    tex.compute_par(move |uv| {
        let mut amp = 1.0f32;
        let mut freq = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for o in 0..OCTAVES {
            let n = noise(uv.xy() * freq, Vec2::broadcast(TILING), 10.0 + o as f32);
            let t = (n - 0.5).abs() * 2.0; // turbulence
            sum += t * amp;
            norm += amp;
            freq *= LACUNARITY;
            amp *= GAIN;
        }
        Vec3::broadcast((sum / norm).clamp(0.0, 1.0))
    });
    tex
}

#[inline]
fn make_fbm_ridge_noise(noise: NoiseFn) -> TexStorage {
    let mut tex = TexStorage::new();
    const OCTAVES: u32 = 5;
    const LACUNARITY: f32 = 2.0;
    const GAIN: f32 = 0.5;
    tex.compute_par(move |uv| {
        let mut amp = 1.0f32;
        let mut freq = 1.0f32;
        let mut sum = 0.0f32;
        let mut norm = 0.0f32;
        for o in 0..OCTAVES {
            let n = noise(uv.xy() * freq, Vec2::broadcast(TILING), 10.0 + o as f32);
            let r = 1.0 - (2.0 * n - 1.0).abs();
            sum += (r * r) * amp; // ridge
            norm += amp;
            freq *= LACUNARITY;
            amp *= GAIN;
        }
        Vec3::broadcast((sum / norm).clamp(0.0, 1.0))
    });
    tex
}

// --- Bricks

fn _rot(a: f32) -> Mat2<f32> {
    Mat2::new(a.cos(), -a.sin(), a.sin(), a.cos())
}

#[inline(always)]
pub fn hash21(p: Vec2<f32>) -> f32 {
    let mut p3 = Vec3::new(
        (p.x * 0.1031).fract(),
        (p.y * 0.1031).fract(),
        (p.x * 0.1031).fract(),
    );
    let dot = p3.dot(Vec3::new(p3.y + 33.333, p3.z + 33.333, p3.x + 33.333));

    p3.x += dot;
    p3.y += dot;
    p3.z += dot;

    ((p3.x + p3.y) * p3.z).fract()
}

pub fn bricks(uv3: Vec3<f32>) -> Vec3<f32> {
    fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
        let d = p.map(|v| v.abs()) - b + Vec2::new(r, r);
        d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
    }

    let ratio = 3.0;
    let round = 0.0;
    // let rotation = 0.0;
    let gap = 0.1;
    let cell = 16.0;
    let mode = 0;

    let mut u = uv3.xy();

    let w = Vec2::new(ratio, 1.0);
    u *= Vec2::new(cell, cell) / w;

    if mode == 0 {
        u.x += 0.5 * (u.y.floor() % 2.0);
    }

    let id = hash21(u.map(|v| v.floor()));

    let p = u.map(|v| v.fract());
    // p = rot((id - 0.5) * rotation) * (p - 0.5);

    // hit.hash = id;
    // hit.uv = p;

    let d = s_box(p, Vec2::new(0.5, 0.5) - gap, round);

    Vec3::new(if d <= 0.0 { 1.0 } else { 0.0 }, id, d.abs())
}
