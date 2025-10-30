use crate::Texture;
use codegridfx::Module;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug, Default)]
pub enum TileRole {
    Character, // #d9ac8b
    Nature,    // #3e6958
    Mountain,  // #b1a58d
    Road,      // #624c3c
    Water,     // #243d5c
    #[default]
    ManMade, // #e0c872
    Dungeon,   // #b03a48
    Effect,    // #d4804d
    Icon,      // #5c8b93
    UI,        // #e3cfb4
}

impl TileRole {
    pub fn to_color(self) -> TheColor {
        match self {
            TileRole::Character => TheColor::from_hex("#d9ac8b"),
            TileRole::Nature => TheColor::from_hex("#3e6958"),
            TileRole::Mountain => TheColor::from_hex("#b1a58d"),
            TileRole::Road => TheColor::from_hex("#624c3c"),
            TileRole::Water => TheColor::from_hex("#243d5c"),
            TileRole::ManMade => TheColor::from_hex("#e0c872"),
            TileRole::Dungeon => TheColor::from_hex("#b03a48"),
            TileRole::Effect => TheColor::from_hex("#d4804d"),
            TileRole::Icon => TheColor::from_hex("#5c8b93"),
            TileRole::UI => TheColor::from_hex("#e3cfb4"),
        }
    }
    pub fn to_string(self) -> &'static str {
        match self {
            TileRole::Character => "Character",
            TileRole::Nature => "Nature",
            TileRole::Mountain => "Mountain",
            TileRole::Road => "Road",
            TileRole::Water => "Water",
            TileRole::ManMade => "Man Made",
            TileRole::Dungeon => "Dungeon",
            TileRole::Effect => "Effect",
            TileRole::Icon => "Icon",
            TileRole::UI => "UI",
        }
    }
    pub fn iterator() -> impl Iterator<Item = TileRole> {
        [
            TileRole::Character,
            TileRole::Nature,
            TileRole::Mountain,
            TileRole::Road,
            TileRole::Water,
            TileRole::ManMade,
            TileRole::Dungeon,
            TileRole::Effect,
            TileRole::Icon,
            TileRole::UI,
        ]
        .iter()
        .copied()
    }
    pub fn from_index(index: u8) -> TileRole {
        match index {
            0 => TileRole::Character,
            1 => TileRole::Nature,
            2 => TileRole::Mountain,
            3 => TileRole::Road,
            4 => TileRole::Water,
            5 => TileRole::ManMade,
            6 => TileRole::Dungeon,
            7 => TileRole::Effect,
            8 => TileRole::Icon,
            9 => TileRole::UI,
            _ => TileRole::ManMade,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub struct Tile {
    pub id: Uuid,
    pub role: TileRole,
    /// The textures of the tiles. Primary source.
    pub textures: Vec<Texture>,
    /// The module if the textures are shader generated
    pub module: Option<Module>,
    /// For top down 2D scenarios
    pub blocking: bool,
    /// The scale of the tile (mostly used for billboard rendering)
    pub scale: f32,
    /// Tags
    pub tags: String,
}

impl Tile {
    /// Create a tile from a single texture.
    pub fn from_texture(texture: Texture) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: TileRole::ManMade,
            textures: vec![texture],
            module: None,
            blocking: false,
            scale: 1.0,
            tags: String::new(),
        }
    }

    /// Create a tile from a vector of textures.
    pub fn from_textures(textures: Vec<Texture>) -> Self {
        Self {
            id: Uuid::new_v4(),
            textures,
            module: None,
            blocking: false,
            scale: 1.0,
            tags: String::new(),
            ..Default::default()
        }
    }

    /// Create an empty tile.
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            textures: vec![],
            module: None,
            blocking: false,
            scale: 1.0,
            tags: String::new(),
            ..Default::default()
        }
    }

    /// Append a texture to the Tile.
    pub fn append(&mut self, texture: Texture) {
        self.textures.push(texture);
    }
}
