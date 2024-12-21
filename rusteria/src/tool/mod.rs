pub mod linedef;

use rusterix::Map;
use vek::Vec2;

/// The shader trait.
#[allow(unused)]
pub trait Tool: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn touch_down(&mut self, coord: Vec2<f32>, map: &mut Map) {}
}
