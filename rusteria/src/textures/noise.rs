use vek::{Vec2, Vec3, Vec4};

use crate::textures::interpolate::noise_interpolate2;
use crate::textures::multi_hash::better_hash2d_cell_i;
use crate::textures::multi_hash::multi_hash2d;

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
    let p = pos * scale;

    // integer base cell and fractional coords inside the cell
    let pf = p.floor();
    let mut fx = p.x - pf.x;
    let mut fy = p.y - pf.y;

    // guard against tiny FP drift outside [0,1]
    fx = fx.clamp(0.0, 1.0);
    fy = fy.clamp(0.0, 1.0);

    // wrap to integer domain and add integer seed
    let sx = scale.x as i32;
    let sy = scale.y as i32;
    let seed_i = seed as i32;

    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);

    // 4 corner hashes in [0,1] using integer-wrapped cell ids
    let h = better_hash2d_cell_i(ix, iy, sx, sy, seed_i);
    let a = h.x; // (ix,   iy)
    let b = h.y; // (ix+1, iy)
    let c = h.z; // (ix,   iy+1)
    let d = h.w; // (ix+1, iy+1)

    // Quintic fade per axis
    let u = noise_interpolate2(Vec2::new(fx, fy));

    // Canonical bilinear blend
    let ab = a + (b - a) * u.x; // row y=0
    let cd = c + (d - c) * u.x; // row y=1
    let v = ab + (cd - ab) * u.y;
    v
}

use crate::textures::multi_hash::smulti_hash2d;

/// 2D Gradient noise (tileable if `scale` is integral).
/// - `pos`: input UV
/// - `scale`: number of tiles (x,y), must be integers for perfect tiling
/// - `seed`: scalar seed
/// Returns noise in **[0,1]**.
#[inline]
pub fn gradient_noise_2d(pos: Vec2<f32>, scale: Vec2<f32>, seed: f32) -> f32 {
    // Scale position
    let p = pos * scale;

    // i = floor(pos).xyxy + vec2(0,1).xxyy
    let pf = p.floor();

    // integer corner ids (wrapped) + integer seed -> then pack to Vec4
    let sx = scale.x as i32;
    let sy = scale.y as i32;
    let seed_i = seed as i32;
    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);
    let ix1 = (ix + 1).rem_euclid(sx);
    let iy1 = (iy + 1).rem_euclid(sy);

    let i =
        Vec4::new(ix as f32, iy as f32, ix1 as f32, iy1 as f32) + Vec4::broadcast(seed_i as f32);

    let (mut hx, mut hy) = smulti_hash2d(i);

    // Fractional offsets within the base cell
    let fx = p.x - pf.x;
    let fy = p.y - pf.y;

    // Normalize gradient vectors per corner to unit length to improve dynamic range
    let eps = 1e-8;
    let len2 = hx * hx + hy * hy;
    let inv_len = (len2 + Vec4::broadcast(eps)).map(|v| v.sqrt());
    hx = hx / inv_len;
    hy = hy / inv_len;

    // Per-corner displacement vectors matching the hash corner order
    // (fx,fy), (fx-1,fy), (fx,fy-1), (fx-1,fy-1)
    let dx = Vec4::new(fx, fx - 1.0, fx, fx - 1.0);
    let dy = Vec4::new(fy, fy, fy - 1.0, fy - 1.0);

    // Dot products
    let g00_g10_g01_g11 = hx * dx + hy * dy;

    // Hermite smoothing weights from fractional coords
    let u = noise_interpolate2(Vec2::new(fx, fy));

    // Interpolate along x for y=0 and y=1 rows
    let g0 = mixf(g00_g10_g01_g11.x, g00_g10_g01_g11.y, u.x); // between (ix,iy) and (ix+1,iy)
    let g1 = mixf(g00_g10_g01_g11.z, g00_g10_g01_g11.w, u.x); // between (ix,iy+1) and (ix+1,iy+1)

    // Then along y
    let g = mixf(g0, g1, u.y);

    // √2 scaling matches GLSL; then remap [-1,1] -> [0,1]
    let val = 1.414_213_56 * g;
    0.5 * (val + 1.0)
}

#[inline(always)]
fn dot4(a: Vec4<f32>, b: Vec4<f32>) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w
}

/// 2D Perlin noise (tileable if `scale` is integer).
/// - `pos`: input UV
/// - `scale`: number of tiles (x,y), must be integers for perfect tiling
/// - `seed`: scalar seed
/// Returns noise in **[0,1]**.
#[inline]
pub fn perlin_noise_2d(pos: Vec2<f32>, scale: Vec2<f32>, seed: f32) -> f32 {
    // Scale position
    let p = pos * scale;

    // i = floor(pos).xyxy + vec2(0,1).xxyy
    let pf = p.floor();

    let sx = scale.x as i32;
    let sy = scale.y as i32;
    let seed_i = seed as i32;
    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);
    let ix1 = (ix + 1).rem_euclid(sx);
    let iy1 = (iy + 1).rem_euclid(sy);

    let i =
        Vec4::new(ix as f32, iy as f32, ix1 as f32, iy1 as f32) + Vec4::broadcast(seed_i as f32);

    // Fractional offsets within the base cell
    let fx = p.x - pf.x;
    let fy = p.y - pf.y;

    // Grid gradients from hash (in [0,1]); shift to ~[-0.5,0.5]
    let (mut gx, mut gy) = multi_hash2d(i);
    gx = gx - Vec4::broadcast(0.49999);
    gy = gy - Vec4::broadcast(0.49999);

    // Perlin surflet: normalize gradients, dot with displacement per corner
    let inv_len = (gx * gx + gy * gy).map(|v| v.sqrt().recip());
    // Displacements per corner: (fx,fy), (fx-1,fy), (fx,fy-1), (fx-1,fy-1)
    let dx = Vec4::new(fx, fx - 1.0, fx, fx - 1.0);
    let dy = Vec4::new(fy, fy, fy - 1.0, fy - 1.0);
    let gradients = inv_len * (gx * dx + gy * dy);

    // Normalize amplitude
    let gradients = gradients * 2.3703703703703702; // 1.0 / 0.75^3

    // Compute fade weights from per-corner r^2 = dx^2 + dy^2
    let r2 = dx * dx + dy * dy;
    let mut w = Vec4::new(
        (1.0 - r2.x).max(0.0),
        (1.0 - r2.y).max(0.0),
        (1.0 - r2.z).max(0.0),
        (1.0 - r2.w).max(0.0),
    );
    w = w * w * w;

    // Weighted dot
    let val = dot4(w, gradients);

    // GLSL version returns [-1,1]; remap to [0,1]
    0.5 * (val + 1.0)
}

#[inline(always)]
pub fn voronoi_2d(
    pos: Vec2<f32>,
    scale: Vec2<f32>,
    jitter: f32,
    phase: f32,
    seed: f32,
) -> Vec3<f32> {
    // Voronoi based on Inigo Quilez: https://archive.is/Ta7dm
    const KPI2: f32 = 6.283_185_307_1;

    // Tile domain
    let p = pos * scale;
    let i = p.floor(); // integer cell
    let f = p - i; // fractional part inside cell

    // ---------- First pass: find nearest cell center ----------
    let mut min_pos = Vec2::zero(); // relative vector to closest cell center
    let mut tile_pos = Vec2::zero(); // "jittered" center position within the cell (after sin-rotate)
    let mut min_distance = 1.0e5f32;

    // Enumerate 8 neighbors via the GLSL pattern (k = 0,2,4,6)
    for k in (0..8).step_by(2) {
        let k1x = k as i32;
        let k1y = (k + 1) as i32;
        let kyx = k1x / 3;
        let kyy = k1y / 3;

        // n = vec4(k1 - ky*3, ky).xzyw - 1
        let ax = (k1x - kyx * 3) as f32;
        let ay = (k1y - kyy * 3) as f32;
        let n = Vec4::new(ax, kyx as f32, ay, kyy as f32) - Vec4::broadcast(1.0);

        // ni = mod(i.xyxy + n, scale.xyxy) + seed
        let ixyxy = Vec4::new(i.x, i.y, i.x, i.y);
        let sxyxy = Vec4::new(scale.x, scale.y, scale.x, scale.y);
        let mut ni = ixyxy + n;
        ni.x = ni.x.rem_euclid(sxyxy.x);
        ni.y = ni.y.rem_euclid(sxyxy.y);
        ni.z = ni.z.rem_euclid(sxyxy.z);
        ni.w = ni.w.rem_euclid(sxyxy.w);
        ni = ni + Vec4::broadcast(seed);

        // Derive two 2D randoms via the Vec4->(Vec4,Vec4) signature by duplicating coords
        let (hx0, hy0) = multi_hash2d(Vec4::new(ni.x, ni.y, ni.x, ni.y));
        let (hx1, hy1) = multi_hash2d(Vec4::new(ni.z, ni.w, ni.z, ni.w));
        // Pack them as (x0,y0,x1,y1)
        let mut cpos = Vec4::new(hx0.x, hy0.x, hx1.x, hy1.x) * jitter;
        // then rotate/warp: 0.5*sin(phase + 2π*cPos) + 0.5   (component-wise)
        cpos = (cpos.map(|x| (phase + KPI2 * x).sin()) * 0.5) + Vec4::broadcast(0.5);

        // rPos = n + cPos - f.xyxy
        let fxyxy = Vec4::new(f.x, f.y, f.x, f.y);
        let rpos = n + cpos - fxyxy;

        // distances for the two corners in this pair
        let d0 = rpos.x * rpos.x + rpos.y * rpos.y;
        let d1 = rpos.z * rpos.z + rpos.w * rpos.w;

        if d0 < min_distance {
            min_distance = d0;
            min_pos = Vec2::new(rpos.x, rpos.y);
            tile_pos = Vec2::new(cpos.x, cpos.y);
        }
        if d1 < min_distance {
            min_distance = d1;
            min_pos = Vec2::new(rpos.z, rpos.w);
            tile_pos = Vec2::new(cpos.z, cpos.w);
        }
    }

    // Last cell: (1,1) neighbor
    {
        let n = Vec2::broadcast(1.0);
        let mut ni = i + n;
        ni.x = ni.x.rem_euclid(scale.x);
        ni.y = ni.y.rem_euclid(scale.y);
        let (hx, hy) = multi_hash2d(Vec4::new(ni.x, ni.y, ni.x, ni.y));
        let mut cpos = Vec2::new(hx.x, hy.x) * jitter;
        cpos = (cpos.map(|x| (phase + KPI2 * x).sin()) * 0.5) + Vec2::broadcast(0.5);
        let rpos = n + cpos - f;

        let d = rpos.dot(rpos);
        if d < min_distance {
            // min_distance = d;
            min_pos = rpos;
            tile_pos = cpos;
        }
    }

    // ---------- Second pass: distance to edges ----------
    // We compute perpendicular distances to bisectors between the winner (min_pos) and neighboring centers
    min_distance = 1.0e5f32;

    for y in -2..=2 {
        for x in (-2..=2).step_by(2) {
            let n = Vec4::new(x as f32, y as f32, (x + 1) as f32, y as f32);

            // ni = mod(i.xyxy + n, scale.xyxy) + seed
            let ixyxy = Vec4::new(i.x, i.y, i.x, i.y);
            let sxyxy = Vec4::new(scale.x, scale.y, scale.x, scale.y);
            let mut ni = ixyxy + n;
            ni.x = ni.x.rem_euclid(sxyxy.x);
            ni.y = ni.y.rem_euclid(sxyxy.y);
            ni.z = ni.z.rem_euclid(sxyxy.z);
            ni.w = ni.w.rem_euclid(sxyxy.w);
            ni = ni + Vec4::broadcast(seed);

            // Use Vec4 signature; duplicate coords to get two independent 2D hashes
            let (hx0, hy0) = multi_hash2d(Vec4::new(ni.x, ni.y, ni.x, ni.y));
            let (hx1, hy1) = multi_hash2d(Vec4::new(ni.z, ni.w, ni.z, ni.w));
            let mut cpos = Vec4::new(hx0.x, hy0.x, hx1.x, hy1.x) * jitter;
            cpos = (cpos.map(|x| (phase + KPI2 * x).sin()) * 0.5) + Vec4::broadcast(0.5);

            // rPos = n + cPos - f.xyxy
            let fxyxy = Vec4::new(f.x, f.y, f.x, f.y);
            let rpos = n + cpos - fxyxy;

            // temp = minPos.xyxy - rPos; l = squared length per pair
            let mut tmp = Vec4::new(min_pos.x, min_pos.y, min_pos.x, min_pos.y) - rpos;
            tmp = tmp * tmp;
            let l0 = tmp.x + tmp.y; // pair (x,y)
            let l1 = tmp.z + tmp.w; // pair (z,w)

            // a = 0.5*(minPos.xyxy + rPos)
            let a = (Vec4::new(min_pos.x, min_pos.y, min_pos.x, min_pos.y) + rpos) * 0.5;

            // b = rPos - minPos.xyxy; normalize each pair separately
            let mut b = rpos - Vec4::new(min_pos.x, min_pos.y, min_pos.x, min_pos.y);
            // lengths per pair
            let len0 = (b.x * b.x + b.y * b.y).sqrt().max(1e-8);
            let len1 = (b.z * b.z + b.w * b.w).sqrt().max(1e-8);
            // normalize
            b.x /= len0;
            b.y /= len0;
            b.z /= len1;
            b.w /= len1;

            // temp = a * b; then d = temp.xz + temp.yw  (dot per pair)
            let temp = a * b;
            let d0 = temp.x + temp.y;
            let d1 = temp.z + temp.w;

            if l0 > 1e-5 {
                min_distance = min_distance.min(d0);
            }
            if l1 > 1e-5 {
                min_distance = min_distance.min(d1);
            }
        }
    }

    // Return distance-to-edges (x), plus the tile position of the winning cell (y,z)
    Vec3::new(min_distance, tile_pos.x, tile_pos.y)
}

#[inline(always)]
pub fn voronoi_position_2d(
    pos: Vec2<f32>,
    scale: Vec2<f32>,
    jitter: f32,
    phase: f32,
    seed: f32,
) -> Vec3<f32> {
    // Voronoi (position + min distance), tileable
    const KPI2: f32 = 6.283_185_307_1;

    // Tile domain
    let p = pos * scale;
    let i = p.floor(); // integer cell coords
    let f = p - i; // fractional offset

    // -------- First pass: find nearest center --------
    let mut tile_pos = Vec2::zero();
    let mut min_distance = 1.0e5f32;

    // Enumerate 8 neighbors via the GLSL pattern (k = 0,2,4,6)
    for k in (0..8).step_by(2) {
        let k1x = k as i32;
        let k1y = (k + 1) as i32;
        let kyx = k1x / 3;
        let kyy = k1y / 3;

        // n = vec4(k1 - ky*3, ky).xzyw - 1
        let ax = (k1x - kyx * 3) as f32;
        let ay = (k1y - kyy * 3) as f32;
        let n = Vec4::new(ax, kyx as f32, ay, kyy as f32) - Vec4::broadcast(1.0);

        // ni = mod(i.xyxy + n, scale.xyxy) + seed
        let ixyxy = Vec4::new(i.x, i.y, i.x, i.y);
        let sxyxy = Vec4::new(scale.x, scale.y, scale.x, scale.y);
        let mut ni = ixyxy + n;
        ni.x = ni.x.rem_euclid(sxyxy.x);
        ni.y = ni.y.rem_euclid(sxyxy.y);
        ni.z = ni.z.rem_euclid(sxyxy.z);
        ni.w = ni.w.rem_euclid(sxyxy.w);
        ni = ni + Vec4::broadcast(seed);

        // Get two 2D hashes via Vec4 signature (duplicate coords)
        let (hx0, hy0) = multi_hash2d(Vec4::new(ni.x, ni.y, ni.x, ni.y));
        let (hx1, hy1) = multi_hash2d(Vec4::new(ni.z, ni.w, ni.z, ni.w));
        // Pack cPos = (x0,y0,x1,y1)
        let mut cpos = Vec4::new(hx0.x, hy0.x, hx1.x, hy1.x) * jitter;
        cpos = (cpos.map(|x| (phase + KPI2 * x).sin()) * 0.5) + Vec4::broadcast(0.5);

        // rPos = n + cPos - f.xyxy
        let fxyxy = Vec4::new(f.x, f.y, f.x, f.y);
        let rpos = n + cpos - fxyxy;

        // distances for the two corners in this pair
        let d0 = rpos.x * rpos.x + rpos.y * rpos.y;
        let d1 = rpos.z * rpos.z + rpos.w * rpos.w;

        // choose closer and remember its (tile) position
        if d0 < min_distance {
            min_distance = d0;
            tile_pos = Vec2::new(cpos.x, cpos.y);
        }
        if d1 < min_distance {
            min_distance = d1;
            tile_pos = Vec2::new(cpos.z, cpos.w);
        }
    }

    // Last cell: (1,1) neighbor
    {
        let n = Vec2::broadcast(1.0);
        let mut ni = i + n;
        ni.x = ni.x.rem_euclid(scale.x);
        ni.y = ni.y.rem_euclid(scale.y);

        // single 2D hash via Vec4 duplication
        let (hx, hy) = multi_hash2d(Vec4::new(ni.x, ni.y, ni.x, ni.y));
        let mut cpos = Vec2::new(hx.x, hy.x) * jitter;
        cpos = (cpos.map(|x| (phase + KPI2 * x).sin()) * 0.5) + Vec2::broadcast(0.5);

        let rpos = n + cpos - f;
        let d = rpos.dot(rpos);
        if d < min_distance {
            min_distance = d;
            tile_pos = cpos;
        }
    }

    // Return: (tilePos.x, tilePos.y, minDistance)
    Vec3::new(tile_pos.x, tile_pos.y, min_distance)
}

#[inline(always)]
pub fn voronoi_combined_2d(
    pos: Vec2<f32>,
    scale: Vec2<f32>,
    jitter: f32,
    phase: f32,
    seed: f32,
) -> Vec3<f32> {
    let a = voronoi_2d(pos, scale, jitter, phase, seed);
    let b = voronoi_position_2d(pos, scale, jitter, phase, seed);

    // TODO Cracks
    Vec3::new(a.x, b.x, 0.0)
}

#[inline]
pub fn cellular_noise(pos: Vec2<f32>, scale: Vec2<f32>, jitter: f32, seed: f32) -> Vec2<f32> {
    // Wrap incoming UVs into [0,1) to guarantee seamless edges when baking textures
    let pos = Vec2::new(pos.x.rem_euclid(1.0), pos.y.rem_euclid(1.0));
    let p = pos * scale;

    // Base integer cell and fractional offset in [0,1)
    let pf = p.floor();
    let f = p - pf;

    // Wrap indices as integers
    let sx = scale.x as i32;
    let sy = scale.y as i32;
    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);

    // Helper: 2D center in [0,1]^2 for an integer cell (ix,iy)
    #[inline(always)]
    fn center_for_cell(ix: i32, iy: i32, sx: i32, sy: i32, seed: f32) -> Vec2<f32> {
        // Convert wrapped integer cell to float domain and hash once
        let x = (ix.rem_euclid(sx)) as f32 + seed;
        let y = (iy.rem_euclid(sy)) as f32 + seed;
        let (hx, hy) = crate::textures::multi_hash::multi_hash2d(vek::Vec4::new(x, y, x, y));
        vek::Vec2::new(hx.x, hy.x)
    }

    // Sweep the 3×3 neighborhood, gather the 2 smallest squared distances
    let mut f1 = 1.0e9f32;
    let mut f2 = 1.0e9f32;

    for dj in -1..=1 {
        for di in -1..=1 {
            let c = center_for_cell(ix + di, iy + dj, sx, sy, seed) * jitter
                + vek::Vec2::new(di as f32, dj as f32)
                - f;
            let d = c.dot(c);
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }

    // Match GLSL normalization: sqrt and scale (1/1.125)
    vek::Vec2::new(f1.sqrt(), f2.sqrt()) * (1.0 / 1.125)
}

/// Classic 3x3 Cellular noise with F1/F2 (Euclidean). Returns (F1, F2) in [0,1].
#[inline]
pub fn cellular_noise_(pos: Vec2<f32>, scale: Vec2<f32>, jitter: f32, seed: f32) -> Vec2<f32> {
    // Integer-wrapped variant to eliminate FP mod drift across many tiles (esp. vertical seams)
    let p = pos * scale;

    // Base integer cell and fractional in-cell coords
    let pf = p.floor();
    let f = p - pf;

    // Wrap indices in integer domain
    let sx = scale.x as i32;
    let sy = scale.y as i32;

    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);

    // Neighbors per GLSL: cells = mod(i.xyxy + offset.xxzz, scale.xyxy)
    // offset = (-1,0,1) → xxzz = (-1,-1, +1, +1) for (x,y,x,y)
    let ixm = (ix - 1).rem_euclid(sx);
    let iym = (iy - 1).rem_euclid(sy);
    let ixp = (ix + 1).rem_euclid(sx);
    let iyp = (iy + 1).rem_euclid(sy);

    // Convert to floats and add seed AFTER wrapping (matches GLSL ordering)
    let cells = Vec4::new(ixm as f32, iym as f32, ixp as f32, iyp as f32) + Vec4::broadcast(seed);
    let i = Vec2::new(ix as f32, iy as f32) + Vec2::broadcast(seed);

    // Hash 8 neighbors in two batches, matching GLSL argument packing
    let (dx0h, dy0h) = multi_hash2d(Vec4::new(cells.x, cells.y, i.x, cells.y)); // vec4(cells.xy, vec2(i.x, cells.y))
    let (dx0v, dy0v) = multi_hash2d(Vec4::new(cells.z, cells.y, cells.x, i.y)); // vec4(cells.zyx, i.y)
    let mut dx0 = Vec4::new(dx0h.x, dx0h.y, dx0v.x, dx0v.y);
    let mut dy0 = Vec4::new(dy0h.x, dy0h.y, dy0v.x, dy0v.y);

    let (dx1h, dy1h) = multi_hash2d(Vec4::new(cells.z, cells.w, cells.z, i.y)); // vec4(cells.zwz, i.y)
    let (dx1v, dy1v) = multi_hash2d(Vec4::new(cells.x, cells.w, i.x, cells.w)); // vec4(cells.xw, vec2(i.x, cells.w))
    let mut dx1 = Vec4::new(dx1h.x, dx1h.y, dx1v.x, dx1v.y);
    let mut dy1 = Vec4::new(dy1h.x, dy1h.y, dy1v.x, dy1v.y);

    // Offsets layout exactly as shader comments
    dx0 = Vec4::new(-1.0, 0.0, 1.0, -1.0) + dx0 * jitter - Vec4::broadcast(f.x);
    dy0 = Vec4::new(-1.0, -1.0, -1.0, 0.0) + dy0 * jitter - Vec4::broadcast(f.y);
    dx1 = Vec4::new(1.0, 1.0, -1.0, 0.0) + dx1 * jitter - Vec4::broadcast(f.x);
    dy1 = Vec4::new(1.0, 0.0, 1.0, 1.0) + dy1 * jitter - Vec4::broadcast(f.y);

    let d0 = dx0 * dx0 + dy0 * dy0;
    let d1 = dx1 * dx1 + dy1 * dy1;

    // Center (0,0) cell: wrap i and hash once; subtract f for relative vector
    let center = crate::textures::multi_hash::better_hash2d_vec2(i) * jitter - f;

    #[allow(non_snake_case)]
    // Shuffle to extract F1/F2 (vek-safe component-wise mins/maxes)
    let mut F = Vec4::new(
        d0.x.min(d1.x),
        d0.y.min(d1.y),
        d0.z.min(d1.z),
        d0.w.min(d1.w),
    );
    let mx = Vec4::new(
        d0.x.max(d1.x),
        d0.y.max(d1.y),
        d0.z.max(d1.z),
        d0.w.max(d1.w),
    );
    let mx_wzyx = Vec4::new(mx.w, mx.z, mx.y, mx.x);
    F = Vec4::new(
        F.x.min(mx_wzyx.x),
        F.y.min(mx_wzyx.y),
        F.z.min(mx_wzyx.z),
        F.w.min(mx_wzyx.w),
    );

    let a_xy = Vec2::new(F.x.min(F.z), F.y.min(F.w));
    let b_xy = Vec2::new(F.x.max(F.z), F.y.max(F.w));
    let b_yx = Vec2::new(b_xy.y, b_xy.x);
    let new_xy = Vec2::new(a_xy.x.min(b_yx.x), a_xy.y.min(b_yx.y));
    F.x = new_xy.x;
    F.y = new_xy.y;

    let center_d = center.dot(center);
    F.z = center_d;
    F.w = 1.0e5;

    let a2_xy = Vec2::new(F.x.min(F.z), F.y.min(F.w));
    let b2_xy = Vec2::new(F.x.max(F.z), F.y.max(F.w));
    let b2_yx = Vec2::new(b2_xy.y, b2_xy.x);
    let new2_xy = Vec2::new(a2_xy.x.min(b2_yx.x), a2_xy.y.min(b2_yx.y));
    F.x = new2_xy.x;
    F.y = new2_xy.y;

    let f12 = Vec2::new(F.x.min(F.y), F.x.max(F.y));
    f12.map(f32::sqrt) * (1.0 / 1.125)
}

/// Metaballs variation (product of distances to neighbors, square‑rooted), Euclidean.
#[inline]
pub fn metaballs(pos: Vec2<f32>, scale: Vec2<f32>, jitter: f32, seed: f32) -> f32 {
    // Seam‑free metaballs: integer‑wrapped 3×3 neighbor sweep (tile‑perfect at texture edges)
    // Wrap UVs first so baking at u/v==1.0 samples the same tile as 0.0
    let pos = Vec2::new(pos.x.rem_euclid(1.0), pos.y.rem_euclid(1.0));
    let p = pos * scale;

    // Base integer cell and fractional offset in [0,1)
    let pf = p.floor();
    let f = p - pf;

    // Wrap indices in integer domain
    let sx = scale.x as i32;
    let sy = scale.y as i32;
    let ix = (pf.x as i32).rem_euclid(sx);
    let iy = (pf.y as i32).rem_euclid(sy);

    // Helper: jittered center for integer cell (cx,cy) in [0,1]^2
    #[inline(always)]
    fn center_for_cell(cx: i32, cy: i32, sx: i32, sy: i32, seed: f32) -> Vec2<f32> {
        let x = (cx.rem_euclid(sx)) as f32 + seed;
        let y = (cy.rem_euclid(sy)) as f32 + seed;
        let (hx, hy) = crate::textures::multi_hash::multi_hash2d(vek::Vec4::new(x, y, x, y));
        vek::Vec2::new(hx.x, hy.x)
    }

    // Product‑of‑distances variant from the GLSL reference
    // Start with center cell distance (clamped to 1.0), then fold in the 8 neighbors.
    let mut d = {
        let c = center_for_cell(ix, iy, sx, sy, seed) * jitter - f; // (0,0)
        c.dot(c).min(1.0)
    };

    for dj in -1..=1 {
        for di in -1..=1 {
            if di == 0 && dj == 0 {
                continue;
            }
            let c = center_for_cell(ix + di, iy + dj, sx, sy, seed) * jitter
                + Vec2::new(di as f32, dj as f32)
                - f;
            let dist2 = c.dot(c);
            d = d.min(d * dist2);
        }
    }

    d.sqrt()
}

#[inline]
pub fn metaballs_smooth(
    pos: Vec2<f32>,
    scale: Vec2<f32>,
    jitter: f32,
    width: f32,
    smoothness: f32,
    seed: f32,
) -> f32 {
    let d = metaballs(pos, scale, jitter, seed);
    (d - width).clamp(0.0, smoothness) / smoothness
}
