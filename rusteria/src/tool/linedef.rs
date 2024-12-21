use crate::Tool;
use rusterix::Map;
use vek::Vec2;

pub struct LinedefTool;

impl Tool for LinedefTool {
    fn new() -> Self {
        LinedefTool
    }

    fn touch_down(&mut self, _coord: Vec2<f32>, _map: &mut Map) {}
}
