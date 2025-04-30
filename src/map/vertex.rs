use crate::ValueContainer;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Vertex {
    pub id: u32,
    pub x: f32,
    pub y: f32,

    #[serde(default)]
    pub properties: ValueContainer,
}

impl Vertex {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self {
            id,
            x,
            y,
            properties: ValueContainer::default(),
        }
    }

    pub fn as_vec2(&self) -> vek::Vec2<f32> {
        vek::Vec2::new(self.x, self.y)
    }
}
