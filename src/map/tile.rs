use crate::Texture;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub uvs: Vec<vek::Vec4<i32>>,
    pub textures: Vec<Texture>,
    /// For top down 2D scenarios
    pub blocking: bool,
}

impl Tile {
    /// Create a tile from a single texture.
    pub fn from_texture(texture: Texture) -> Self {
        let uv = Vec4::new(0, 0, texture.width as i32, texture.height as i32);
        Self {
            id: Uuid::new_v4(),
            uvs: vec![uv],
            textures: vec![texture],
            blocking: false,
        }
    }

    /// Create an empty tile.
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            uvs: vec![],
            textures: vec![],
            blocking: false,
        }
    }

    /// Append a texture to the Tile.
    pub fn append(&mut self, texture: Texture) {
        self.uvs
            .push(Vec4::new(0, 0, texture.width as i32, texture.height as i32));
        self.textures.push(texture);
    }
}
