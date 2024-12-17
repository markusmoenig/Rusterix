use crate::Texture;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, PartialOrd, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub buffer: Vec<Texture>,
    pub role: u8,
    pub blocking: bool,
    pub billboard: bool,
}
