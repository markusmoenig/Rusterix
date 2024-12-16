//! Rusterix is a fast software renderer for 2D and 3D triangles and lines.
//! Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

pub mod batch;
pub mod edge;
pub mod intodata;
pub mod map;
pub mod rasterizer;
pub mod rect;
pub mod scene;
pub mod shader;
pub mod texture;
pub mod wavefront;

pub type Pixel = [u8; 4];
pub const TRANSPARENT: Pixel = [0, 0, 0, 0];
pub const BLACK: Pixel = [0, 0, 0, 255];
pub const WHITE: Pixel = [255, 255, 255, 255];

// Re-exports
pub use crate::{
    batch::{Batch, CullMode, PrimitiveMode},
    edge::Edge,
    intodata::IntoDataInput,
    map::{linedef::Linedef, sector::Sector, vertex::Vertex, Map},
    rasterizer::Rasterizer,
    rect::Rect,
    scene::Scene,
    shader::{vgradient::VGrayGradientShader, Shader},
    texture::{RepeatMode, SampleMode, Texture},
};

// Prelude
pub mod prelude {
    pub use crate::scene::Scene;
    pub use crate::Edge;
    pub use crate::Rasterizer;
    pub use crate::Rect;
    pub use crate::{Batch, CullMode, PrimitiveMode};
    pub use crate::{Linedef, Map, Sector, Vertex};
    pub use crate::{RepeatMode, SampleMode, Texture};
    pub use crate::{Shader, VGrayGradientShader};
}
