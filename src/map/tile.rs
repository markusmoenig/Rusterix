use crate::Texture;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub uvs: Vec<vek::Vec4<i32>>,
    pub textures: Vec<Texture>,
    pub role: u8,
    pub blocking: bool,
    pub billboard: bool,
}

impl Tile {
    pub fn create_uv(&self, _index: usize, _atlas: &Texture) {}
}
