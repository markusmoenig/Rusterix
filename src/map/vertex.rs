use crate::ValueContainer;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct Vertex {
    pub id: u32,

    #[serde(default)]
    pub name: String,

    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub z: f32,

    #[serde(default)]
    pub properties: ValueContainer,
}

impl Vertex {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self {
            id,
            name: "".into(),
            x,
            y,
            z: 0.0,
            properties: ValueContainer::default(),
        }
    }

    pub fn new_3d(id: u32, x: f32, y: f32, z: f32) -> Self {
        Self {
            id,
            name: "".into(),
            x,
            y,
            z,
            properties: ValueContainer::default(),
        }
    }

    pub fn as_vec2(&self) -> vek::Vec2<f32> {
        vek::Vec2::new(self.x, self.y)
    }

    /// Returns a Vec3 in world coordinates, that means z is used as y-up.
    pub fn as_vec3_world(&self) -> vek::Vec3<f32> {
        vek::Vec3::new(self.x, self.z, self.y)
    }
}
