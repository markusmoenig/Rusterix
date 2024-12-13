pub mod batch;
pub mod edge;
pub mod map;
pub mod rasterizer;
pub mod rect;
pub mod texture;

pub type Pixel = [u8; 4];
pub const TRANSPARENT: Pixel = [0, 0, 0, 0];
pub const BLACK: Pixel = [0, 0, 0, 255];
pub const WHITE: Pixel = [255, 255, 255, 255];

// Re-exports
pub use crate::{
    batch::{Batch, PrimitiveMode},
    edge::Edge,
    map::{linedef::Linedef, sector::Sector, vertex::Vertex, Map},
    rasterizer::Rasterizer,
    rect::Rect,
    texture::{SampleMode, Texture},
};

// Prelude
pub mod prelude {
    pub use crate::Edge;
    pub use crate::Rasterizer;
    pub use crate::Rect;
    pub use crate::{Batch, PrimitiveMode};
    pub use crate::{Linedef, Map, Sector, Vertex};
    pub use crate::{SampleMode, Texture};
}
