pub mod trace;

use vek::{Vec2, Vec3};

pub struct Ray {
    pub origin: Vec3<f32>,
    pub dir: Vec3<f32>,
}

impl Default for Ray {
    fn default() -> Self {
        Ray::new()
    }
}

impl Ray {
    pub fn new() -> Self {
        Self {
            origin: Vec3::zero(),
            dir: Vec3::zero(),
        }
    }
}

#[derive(Debug)]
pub struct HitInfo {
    pub t: f32,
    pub uv: Vec2<f32>,
    pub normal: Option<Vec3<f32>>,
    pub triangle_index: usize,
}

impl Default for HitInfo {
    fn default() -> Self {
        HitInfo::new()
    }
}

impl HitInfo {
    pub fn new() -> Self {
        Self {
            t: f32::MAX,
            uv: Vec2::zero(),
            normal: None,
            triangle_index: 0,
        }
    }
}
