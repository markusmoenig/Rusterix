// src/noise/hash.rs
use vek::{Vec2, Vec3, Vec4};

/* =========================
Helpers
========================= */

#[inline(always)]
fn fract(x: f32) -> f32 {
    x - x.floor()
}

#[inline(always)]
fn _fract2(v: Vec2<f32>) -> Vec2<f32> {
    Vec2::new(fract(v.x), fract(v.y))
}

#[inline(always)]
fn _fract3(v: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(fract(v.x), fract(v.y), fract(v.z))
}

#[inline(always)]
fn fract4(v: Vec4<f32>) -> Vec4<f32> {
    Vec4::new(fract(v.x), fract(v.y), fract(v.z), fract(v.w))
}

#[inline(always)]
fn floor4(v: Vec4<f32>) -> Vec4<f32> {
    Vec4::new(v.x.floor(), v.y.floor(), v.z.floor(), v.w.floor())
}

const INV_U32_MAX: f32 = 1.0 / 4_294_967_295.0;

/* =========================
“permute” family (Brian Sharpe)
========================= */

#[inline(always)]
pub fn permute_prepare_mod289_3(x: Vec3<f32>) -> Vec3<f32> {
    // x - floor(x * (1/289)) * 289
    let t = (x * (1.0 / 289.0)).floor() * 289.0;
    x - t
}

#[inline(always)]
pub fn permute_prepare_mod289_4(x: Vec4<f32>) -> Vec4<f32> {
    let t = floor4(x * (1.0 / 289.0)) * 289.0;
    x - t
}

#[inline(always)]
pub fn permute_resolve(x: Vec4<f32>) -> Vec4<f32> {
    // fract(x * (7/288))
    fract4(x * (7.0 / 288.0))
}

#[inline(always)]
pub fn permute_hash_internal(x: Vec4<f32>) -> Vec4<f32> {
    // fract(x * ((34/289)*x + (1/289))) * 289
    let t = x * ((34.0 / 289.0) * x + (1.0 / 289.0));
    fract4(t) * 289.0
}

/* ---- 2D: permute hashes ---- */

/// Generates a random number for each of the 4 cell corners (2D).
#[inline]
pub fn permute_hash2d(cell: Vec4<f32>) -> Vec4<f32> {
    // cell = permutePrepareMod289(cell * 32.0);
    let cell = permute_prepare_mod289_4(cell * 32.0);
    // return permuteResolve(permuteHashInternal(permuteHashInternal(cell.xzxz) + cell.yyww));
    let xzxz = Vec4::new(cell.x, cell.z, cell.x, cell.z);
    let yyww = Vec4::new(cell.y, cell.y, cell.w, cell.w);
    let h = permute_hash_internal(xzxz);
    permute_resolve(permute_hash_internal(h + yyww))
}

/// Generates 2 random numbers for each of the 4 cell corners (2D).
#[inline]
pub fn permute_hash2d_xy(cell: Vec4<f32>) -> (Vec4<f32>, Vec4<f32>) {
    // cell = permutePrepareMod289(cell);
    let cell = permute_prepare_mod289_4(cell);
    // hashX = permuteHashInternal(permuteHashInternal(cell.xzxz) + cell.yyww);
    // hashY = permuteResolve(permuteHashInternal(hashX));
    // hashX = permuteResolve(hashX);
    let xzxz = Vec4::new(cell.x, cell.z, cell.x, cell.z);
    let yyww = Vec4::new(cell.y, cell.y, cell.w, cell.w);
    let mut hash_x = permute_hash_internal(permute_hash_internal(xzxz) + yyww);
    let hash_y = permute_resolve(permute_hash_internal(hash_x));
    hash_x = permute_resolve(hash_x);
    (hash_x, hash_y)
}

/* =========================
Integer hash (u32 -> u32) for “betterHash” family
========================= */

/// A fast 32-bit integer hash (Wang/Murmur-inspired).
#[inline(always)]
pub fn ihash1d(mut x: u32) -> u32 {
    x = x.wrapping_add(0x9E3779B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    x
}

/* =========================
“betterHash” 2D family
========================= */

/// Generates 2 random numbers for a single 2D coordinate.
#[inline]
pub fn better_hash2d_vec2(x: Vec2<f32>) -> Vec2<f32> {
    let qx = x.x as u32;
    let qy = x.y as u32;
    let h0 = ihash1d(ihash1d(qx).wrapping_add(qy));
    let h1 = h0.wrapping_mul(1_933_247).wrapping_add(!h0) ^ 230_123u32;
    Vec2::new(h0 as f32 * INV_U32_MAX, h1 as f32 * INV_U32_MAX)
}

/// Generates a random number for each of the 4 cell corners.
#[inline]
pub fn better_hash2d_cell(cell: Vec4<f32>) -> Vec4<f32> {
    let i = Vec4::new(cell.x as u32, cell.y as u32, cell.z as u32, cell.w as u32);
    // ihash1D(ihash1D(i.xzxz) + i.yyww)
    let xzxz = Vec4::new(i.x, i.z, i.x, i.z);
    let yyww = Vec4::new(i.y, i.y, i.w, i.w);
    let h = map_u32_vec4(xzxz, ihash1d) + yyww;
    let h = map_u32_vec4(h, ihash1d);
    Vec4::new(
        h.x as f32 * INV_U32_MAX,
        h.y as f32 * INV_U32_MAX,
        h.z as f32 * INV_U32_MAX,
        h.w as f32 * INV_U32_MAX,
    )
}

/// Generates 2 random numbers for each of the 4 cell corners.
#[inline]
pub fn better_hash2d_xy(cell: Vec4<f32>) -> (Vec4<f32>, Vec4<f32>) {
    let i = Vec4::new(cell.x as u32, cell.y as u32, cell.z as u32, cell.w as u32);
    let xzxz = Vec4::new(i.x, i.z, i.x, i.z);
    let yyww = Vec4::new(i.y, i.y, i.w, i.w);
    let h0 = map_u32_vec4(map_u32_vec4(xzxz, ihash1d) + yyww, ihash1d);
    let h1 = map_u32_vec4(h0 ^ splat_u32(1_933_247), ihash1d); // slight tweak: original did ihash1D(hash0 ^ 1933247u)
    (u32_to_unit_vec4(h0), u32_to_unit_vec4(h1))
}

/// Generates 2 random numbers for each of two 2D coordinates.
#[inline]
pub fn better_hash2d_2coords(coords0: Vec2<f32>, coords1: Vec2<f32>) -> Vec4<f32> {
    let i = Vec4::new(
        coords0.x as u32,
        coords0.y as u32,
        coords1.x as u32,
        coords1.y as u32,
    );
    // ihash1D(ihash1D(i.xz) + i.yw).xxyy; then mutate .yw
    let ixz = Vec2::new(i.x, i.z);
    let iyw = Vec2::new(i.y, i.w);
    let h = ihash1d_vec2(map_u32_vec2(ixz, ihash1d) + iyw);
    let mut out = Vec4::new(h.x, h.x, h.y, h.y);
    // out.yw = out.yw * 1933247u + ~out.yw ^ 230123u
    out.y = out.y.wrapping_mul(1_933_247).wrapping_add(!out.y) ^ 230_123u32;
    out.w = out.w.wrapping_mul(1_933_247).wrapping_add(!out.w) ^ 230_123u32;
    u32_to_unit_vec4(out)
}

/// Generates 2 random numbers for each of the four 2D coordinates.
#[inline]
pub fn better_hash2d_4coords(coords0: Vec4<f32>, coords1: Vec4<f32>) -> (Vec4<f32>, Vec4<f32>) {
    // hash0 = ihash1D( ihash1D(uvec4(coords0.xz, coords1.xz)) + uvec4(coords0.yw, coords1.yw) );
    let a = Vec4::new(
        coords0.x as u32,
        coords0.z as u32,
        coords1.x as u32,
        coords1.z as u32,
    );
    let b = Vec4::new(
        coords0.y as u32,
        coords0.w as u32,
        coords1.y as u32,
        coords1.w as u32,
    );
    let h0 = map_u32_vec4(map_u32_vec4(a, ihash1d) + b, ihash1d);
    // hash1 = h0 * 1933247u + ~h0 ^ 230123u
    let h1 = mul_add_xor_vec4(h0, 1_933_247, 230_123);
    (u32_to_unit_vec4(h0), u32_to_unit_vec4(h1))
}

/* =========================
3D families
========================= */

/// permuteHash3D: one value for each of the 8 cell corners.
#[inline]
pub fn permute_hash3d(cell: Vec3<f32>, cell_plus_one: Vec3<f32>) -> (Vec4<f32>, Vec4<f32>) {
    let cell = permute_prepare_mod289_3(cell);
    // cellPlusOne = step(cell, vec3(287.5)) * cellPlusOne;
    let gate = Vec3::new(
        if 287.5 >= cell.x { 1.0 } else { 0.0 },
        if 287.5 >= cell.y { 1.0 } else { 0.0 },
        if 287.5 >= cell.z { 1.0 } else { 0.0 },
    );
    let cell_plus_one = Vec3::new(
        cell_plus_one.x * gate.x,
        cell_plus_one.y * gate.y,
        cell_plus_one.z * gate.z,
    );

    // highHash = permuteHashInternal( permuteHashInternal( vec2(cell.x, cellPlusOne.x).xyxy ) + vec2(cell.y, cellPlusOne.y).xxyy );
    let vxy = Vec2::new(cell.x, cell_plus_one.x);
    let uvy = Vec2::new(cell.y, cell_plus_one.y);
    let a = Vec4::new(vxy.x, vxy.y, vxy.x, vxy.y);
    let b = Vec4::new(uvy.x, uvy.x, uvy.y, uvy.y);
    let mut high = permute_hash_internal(permute_hash_internal(a) + b);

    // lowHash  = permuteResolve( permuteHashInternal( highHash + cell.zzzz ));
    // highHash = permuteResolve( permuteHashInternal( highHash + cellPlusOne.zzzz ));
    let czzzz = Vec4::new(cell.z, cell.z, cell.z, cell.z);
    let pzzzz = Vec4::new(
        cell_plus_one.z,
        cell_plus_one.z,
        cell_plus_one.z,
        cell_plus_one.z,
    );
    let low = permute_resolve(permute_hash_internal(high + czzzz));
    high = permute_resolve(permute_hash_internal(high + pzzzz));
    (low, high)
}

/*
/// fastHash3D: one value for each of the 8 corners.
#[inline]
pub fn fast_hash3d(mut cell: Vec3<f32>, cell_plus_one_in: Vec3<f32>) -> (Vec4<f32>, Vec4<f32>) {
    const K_OFFSET: Vec2<f32> = Vec2::new(50.0, 161.0);
    const K_DOMAIN: f32 = 289.0;
    const K_LARGE: f32 = 635.298_681;
    const KK: f32 = 48.500_388;

    // truncate domain ~ mod(cell, 289)
    cell = cell - (cell * (1.0 / K_DOMAIN)).floor() * K_DOMAIN;

    // gate plus-one like GLSL: step(cell, kDomain-1.5) * cellPlusOne
    let gate = Vec3::new(
        if (K_DOMAIN - 1.5) >= cell.x { 1.0 } else { 0.0 },
        if (K_DOMAIN - 1.5) >= cell.y { 1.0 } else { 0.0 },
        if (K_DOMAIN - 1.5) >= cell.z { 1.0 } else { 0.0 },
    );
    let cell_plus_one = Vec3::new(
        cell_plus_one_in.x * gate.x,
        cell_plus_one_in.y * gate.y,
        cell_plus_one_in.z * gate.z,
    );

    // r = vec4(cell.xy, cellPlusOne.xy) + kOffset.xyxy;
    let mut r = Vec4::new(cell.x, cell.y, cell_plus_one.x, cell_plus_one.y)
        + Vec4::new(K_OFFSET.x, K_OFFSET.y, K_OFFSET.x, K_OFFSET.y);
    r *= r;
    r = Vec4::new(r.x, r.z, r.x, r.z) * Vec4::new(r.y, r.y, r.w, r.w);

    let inv_x = 1.0 / (K_LARGE + Vec2::new(cell.z, cell_plus_one.z) * KK);
    // low = fract(r * inv_x.xxxx); high = fract(r * inv_x.yyyy);
    let low = fract4(r * Vec4::new(inv_x.x, inv_x.x, inv_x.x, inv_x.x));
    let high = fract4(r * Vec4::new(inv_x.y, inv_x.y, inv_x.y, inv_x.y));
    (low, high)
}*/

/// betterHash3D: one value for each of the 8 corners.
#[inline]
pub fn better_hash3d(cell: Vec3<f32>, cell_plus_one: Vec3<f32>) -> (Vec4<f32>, Vec4<f32>) {
    // cells = uvec4(cell.xy, cellPlusOne.xy)
    let cells = Vec4::new(
        cell.x as u32,
        cell.y as u32,
        cell_plus_one.x as u32,
        cell_plus_one.y as u32,
    );
    // hash = ihash1D( ihash1D(cells.xzxz) + cells.yyww )
    let xzxz = Vec4::new(cells.x, cells.z, cells.x, cells.z);
    let yyww = Vec4::new(cells.y, cells.y, cells.w, cells.w);
    let base = map_u32_vec4(map_u32_vec4(xzxz, ihash1d) + yyww, ihash1d);
    // low  = ihash1D(hash + uint(cell.z))
    // high = ihash1D(hash + uint(cellPlusOne.z))
    let add_low = splat_u32(cell.z as u32);
    let add_high = splat_u32(cell_plus_one.z as u32);
    let low_u = map_u32_vec4(base + add_low, ihash1d);
    let high_u = map_u32_vec4(base + add_high, ihash1d);
    (u32_to_unit_vec4(low_u), u32_to_unit_vec4(high_u))
}

/* =========================
“multi” selection + signed variant
========================= */

// Choose which family you want; here we default to “better”.
#[inline]
pub fn multi_hash2d(cell: Vec4<f32>) -> (Vec4<f32>, Vec4<f32>) {
    better_hash2d_xy(cell)
}

// Signed variant: maps to [-1, 1]
#[inline]
pub fn smulti_hash2d(cell: Vec4<f32>) -> (Vec4<f32>, Vec4<f32>) {
    let (hx, hy) = multi_hash2d(cell);
    (hx * 2.0 - 1.0, hy * 2.0 - 1.0)
}

/* =========================
Small u32 vector helpers
========================= */

#[inline(always)]
fn splat_u32(x: u32) -> Vec4<u32> {
    Vec4::new(x, x, x, x)
}

#[inline(always)]
fn map_u32_vec4(mut v: Vec4<u32>, f: fn(u32) -> u32) -> Vec4<u32> {
    v.x = f(v.x);
    v.y = f(v.y);
    v.z = f(v.z);
    v.w = f(v.w);
    v
}

#[inline(always)]
fn map_u32_vec2(mut v: Vec2<u32>, f: fn(u32) -> u32) -> Vec2<u32> {
    v.x = f(v.x);
    v.y = f(v.y);
    v
}

#[inline(always)]
fn ihash1d_vec2(v: Vec2<u32>) -> Vec2<u32> {
    Vec2::new(ihash1d(v.x), ihash1d(v.y))
}

#[inline(always)]
fn mul_add_xor_vec4(h: Vec4<u32>, mul: u32, xor_k: u32) -> Vec4<u32> {
    Vec4::new(
        h.x.wrapping_mul(mul).wrapping_add(!h.x) ^ xor_k,
        h.y.wrapping_mul(mul).wrapping_add(!h.y) ^ xor_k,
        h.z.wrapping_mul(mul).wrapping_add(!h.z) ^ xor_k,
        h.w.wrapping_mul(mul).wrapping_add(!h.w) ^ xor_k,
    )
}

#[inline(always)]
fn u32_to_unit_vec4(h: Vec4<u32>) -> Vec4<f32> {
    Vec4::new(
        h.x as f32 * INV_U32_MAX,
        h.y as f32 * INV_U32_MAX,
        h.z as f32 * INV_U32_MAX,
        h.w as f32 * INV_U32_MAX,
    )
}
