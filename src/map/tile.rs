use crate::Texture;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub uvs: Vec<vek::Vec4<i32>>,
    pub textures: Vec<Texture>,
}

impl Tile {
    pub fn from_texture(name: &str, texture: Texture) -> Self {
        let uv = Vec4::new(0, 0, texture.width as i32, texture.height as i32);
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            uvs: vec![uv],
            textures: vec![texture],
        }
    }
}
