use vek::{Vec2, Vec3, Vec4};

#[inline(always)]
pub fn noise_interpolate(x: f32) -> f32 {
    // u = x^3 * (x * (6x - 15) + 10)
    let x2 = x * x;
    x2 * x * (x * (x * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
pub fn noise_interpolate2(x: Vec2<f32>) -> Vec2<f32> {
    let x2 = x * x;
    x2 * x * (x * (x * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
pub fn noise_interpolate3(x: Vec3<f32>) -> Vec3<f32> {
    let x2 = x * x;
    x2 * x * (x * (x * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
pub fn noise_interpolate4(x: Vec4<f32>) -> Vec4<f32> {
    let x2 = x * x;
    x2 * x * (x * (x * 6.0 - 15.0) + 10.0)
}

#[inline(always)]
pub fn noise_interpolate_du(x: f32) -> (f32, f32) {
    // u = x^3 * (x * (6x - 15) + 10)
    // du = 30 x^2 * (x(x - 2) + 1)
    let x2 = x * x;
    let u = x2 * x * (x * (x * 6.0 - 15.0) + 10.0);
    let du = 30.0 * x2 * (x * (x - 2.0) + 1.0);
    (u, du)
}

#[inline(always)]
pub fn noise_interpolate_du2(x: Vec2<f32>) -> (Vec2<f32>, Vec2<f32>) {
    let x2 = x * x;
    let u = x2 * x * (x * (x * 6.0 - 15.0) + 10.0);
    let du = (x * (x - 2.0) + 1.0) * (30.0 * x2); // reorder for a smidge fewer ops
    (u, du)
}

#[inline(always)]
pub fn noise_interpolate_du3(x: Vec3<f32>) -> (Vec3<f32>, Vec3<f32>) {
    let x2 = x * x;
    let u = x2 * x * (x * (x * 6.0 - 15.0) + 10.0);
    let du = (x * (x - 2.0) + 1.0) * (30.0 * x2);
    (u, du)
}
