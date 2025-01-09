use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
pub enum PixelSource {
    #[default]
    Off,
    TileId(Uuid),
    MaterialId(Uuid),
    Color(TheColor),
}
