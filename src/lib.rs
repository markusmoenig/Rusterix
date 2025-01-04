//! Rusterix is a fast software renderer for 2D and 3D triangles and lines.
//! Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

pub mod batch;
pub mod camera;
pub mod edge;
pub mod intodata;
pub mod map;
pub mod rasterizer;
pub mod rect;
pub mod rusterix;
pub mod scene;
pub mod scenebuilder;
pub mod script;
pub mod server;
pub mod shader;
pub mod texture;
pub mod value;
pub mod wavefront;

use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "embedded/"]
#[exclude = "*.txt"]
#[exclude = "*.DS_Store"]
pub struct Embedded;

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
    camera::{d3firstp::D3FirstPCamera, d3iso::D3IsoCamera, d3orbit::D3OrbitCamera, D3Camera},
    edge::Edges,
    intodata::IntoDataInput,
    map::{
        light::Light, linedef::Linedef, meta::MapMeta, sector::Sector, tile::Tile, vertex::Vertex,
        Map, MapToolType,
    },
    rasterizer::Rasterizer,
    rect::Rect,
    scene::Scene,
    scenebuilder::{d2preview::D2PreviewBuilder, SceneBuilder},
    script::mapscript::MapScript,
    server::{
        assets::Assets, entity::Entity, entity::EntityUpdate, message::EntityAction,
        message::RegionMessage, region::Region, Server,
    },
    shader::{grid::GridShader, vgradient::VGrayGradientShader, Shader},
    texture::{RepeatMode, SampleMode, Texture},
    value::Value,
};

// Prelude
pub mod prelude {
    pub use crate::scenebuilder::{
        d2preview::D2PreviewBuilder, d3builder::D3Builder, SceneBuilder,
    };
    pub use crate::IntoDataInput;
    pub use crate::MapScript;
    pub use crate::Rasterizer;
    pub use crate::{pixel_to_vec4, vec4_to_pixel};
    pub use crate::{Assets, Entity, EntityUpdate, Region, RegionMessage, Server};
    pub use crate::{Batch, CullMode, PrimitiveMode};
    pub use crate::{D3Camera, D3FirstPCamera, D3IsoCamera, D3OrbitCamera};
    pub use crate::{GridShader, Shader, VGrayGradientShader};
    pub use crate::{Light, Linedef, Map, MapMeta, MapToolType, Sector, Tile, Vertex};
    pub use crate::{Pixel, BLACK, TRANSPARENT, WHITE};
    pub use crate::{Rect, Scene, Value};
    pub use crate::{RepeatMode, SampleMode, Texture};
}
