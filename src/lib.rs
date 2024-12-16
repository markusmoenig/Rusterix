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

/// Convert from Pixel to Vec4<f32>
#[inline(always)]
pub fn pixel_to_vec4(pixel: &Pixel) -> vek::Vec4<f32> {
    vek::Vec4::new(
        pixel[0] as f32 / 255.0,
        pixel[1] as f32 / 255.0,
        pixel[2] as f32 / 255.0,
        pixel[3] as f32 / 255.0,
    )
}

/// Convert from Vec4<f32> to Pixel
#[inline(always)]
pub fn vec4_to_pixel(vec: &vek::Vec4<f32>) -> Pixel {
    [
        (vec.x * 255.0) as u8,
        (vec.y * 255.0) as u8,
        (vec.z * 255.0) as u8,
        (vec.w * 255.0) as u8,
    ]
}

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
    shader::{grid::GridShader, vgradient::VGrayGradientShader, Shader},
    texture::{RepeatMode, SampleMode, Texture},
};

// Prelude
pub mod prelude {
    pub use crate::scene::Scene;
    pub use crate::Edge;
    pub use crate::Rasterizer;
    pub use crate::Rect;
    pub use crate::{pixel_to_vec4, vec4_to_pixel};
    pub use crate::{Batch, CullMode, PrimitiveMode};
    pub use crate::{GridShader, Shader, VGrayGradientShader};
    pub use crate::{Linedef, Map, Sector, Vertex};
    pub use crate::{RepeatMode, SampleMode, Texture};
}
