use crate::Texture;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub textures: Vec<Texture>,
    /// For top down 2D scenarios
    pub blocking: bool,
    /// The scale of the tile (mostly used for billboard rendering)
    pub scale: f32,
    /// For Rect Tool rendering in 3D
    pub render_mode: u8,
}

impl Tile {
    /// Create a tile from a single texture.
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            id: Uuid::new_v4(),
            textures: vec![texture],
            blocking: false,
            scale: 1.0,
            render_mode: 0,
        }
    }

    /// Create an empty tile.
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            textures: vec![],
            blocking: false,
            scale: 1.0,
            render_mode: 0,
        }
    }

    /// Append a texture to the Tile.
    pub fn append(&mut self, texture: Texture) {
        self.textures.push(texture);
    }
}
