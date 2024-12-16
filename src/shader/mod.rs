pub mod vgradient;

use crate::{Pixel, BLACK};
use vek::Vec2;

#[allow(unused)]
pub trait Shader: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn shade_pixel(&self, uv: Vec2<f32>, screen: Vec2<f32>) -> Pixel {
        BLACK
    }
}
