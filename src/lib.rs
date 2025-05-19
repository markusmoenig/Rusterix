//! Rusterix is a fast software renderer for 2D and 3D triangles and lines.
//! Its goals are to provide an easy and portable alternative to hardware rasterization for retro and low-poly games.

pub mod batch;
pub mod camera;
pub mod chunk;
pub mod chunkbuilder;
pub mod client;
pub mod edge;
pub mod intodata;
pub mod map;
pub mod rasterizer;
pub mod rect;
pub mod rusterix;
pub mod scene;
pub mod scenebuilder;
pub mod scenemanager;
pub mod script;
pub mod server;
pub mod shader;
pub mod shapestack;
pub mod terrain;
pub mod texture;
pub mod tracer;
pub mod utils;
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
    let inv_255 = 1.0 / 255.0;
    vek::Vec4::new(
        pixel[0] as f32 * inv_255,
        pixel[1] as f32 * inv_255,
        pixel[2] as f32 * inv_255,
        pixel[3] as f32 * inv_255,
    )
}

/// Convert from Vec4<f32> to Pixel
#[inline(always)]
pub fn vec4_to_pixel(vec: &vek::Vec4<f32>) -> Pixel {
    let scale = 255.0;
    [
        (vec.x * scale) as u8,
        (vec.y * scale) as u8,
        (vec.z * scale) as u8,
        (vec.w * scale) as u8,
    ]
}

/// Get time in ms
pub fn get_time() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now() as u128
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let stop = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");
        stop.as_millis()
    }
}

pub const TRANSPARENT: Pixel = [0, 0, 0, 0];
pub const BLACK: Pixel = [0, 0, 0, 255];
pub const WHITE: Pixel = [255, 255, 255, 255];

// Re-exports
pub use crate::{
    batch::{CullMode, PrimitiveMode, batch2d::Batch2D, batch3d::Batch3D},
    camera::{D3Camera, d3firstp::D3FirstPCamera, d3iso::D3IsoCamera, d3orbit::D3OrbitCamera},
    chunk::Chunk,
    chunkbuilder::{ChunkBuilder, d2chunkbuilder::D2ChunkBuilder, d3chunkbuilder::D3ChunkBuilder},
    client::{Client, command::Command, daylight::Daylight},
    edge::Edges,
    intodata::IntoDataInput,
    map::{
        Map, MapCamera, MapToolType, bbox::BBox, light::CompiledLight, light::Light,
        light::LightType, linedef::CompiledLinedef, linedef::Linedef, meta::MapMeta, mini::MapMini,
        pixelsource::NoiseTarget, pixelsource::PixelSource, sector::Sector,
        state::AnimationVertexState, state::InterpolationType, state::VertexAnimationSystem,
        state::VertexState, tile::Tile, vertex::Vertex,
    },
    rasterizer::{BrushPreview, Rasterizer},
    rect::Rect,
    rusterix::Rusterix,
    scene::Scene,
    scenebuilder::{
        d2builder::D2Builder, d2material::D2MaterialBuilder, d2preview::D2PreviewBuilder,
    },
    scenemanager::*,
    script::mapscript::MapScript,
    server::{
        Server, ServerState,
        assets::Assets,
        currency::{Currencies, Currency, Wallet},
        entity::Entity,
        entity::EntityUpdate,
        item::{Item, ItemUpdate},
        message::EntityAction,
        message::PlayerCamera,
        message::RegionMessage,
        region::RegionInstance,
    },
    shader::{Shader, grid::GridShader, vgradient::VGrayGradientShader},
    shapestack::{
        ShapeStack,
        material::{Material, MaterialModifier, MaterialRole},
        shape::{Shape, ShapeType},
        shapecontext::ShapeContext,
        shapefx::{ShapeFX, ShapeFXParam, ShapeFXRole},
        shapefxgraph::ShapeFXGraph,
    },
    terrain::{
        Terrain, TerrainHit,
        chunk::{TerrainBlendMode, TerrainChunk},
    },
    texture::{RepeatMode, SampleMode, Texture},
    tracer::{HitInfo, Ray, buffer::AccumBuffer, trace::Tracer},
    value::{Value, ValueContainer},
};

// Prelude
pub mod prelude {
    pub use crate::Chunk;
    pub use crate::Client;
    pub use crate::IntoDataInput;
    pub use crate::MapScript;
    pub use crate::Rasterizer;
    pub use crate::scenebuilder::{
        d2builder::D2Builder, d2material::D2MaterialBuilder, d2preview::D2PreviewBuilder,
        d3builder::D3Builder,
    };
    pub use crate::{
        AnimationVertexState, Light, LightType, Map, MapMeta, MapToolType, NoiseTarget,
        PixelSource, Sector, Tile, Vertex, VertexAnimationSystem, VertexState,
    };
    pub use crate::{
        Assets, Currencies, Currency, Entity, EntityUpdate, Item, ItemUpdate, RegionInstance,
        RegionMessage, Server, Wallet,
    };
    pub use crate::{BLACK, Pixel, TRANSPARENT, WHITE};
    pub use crate::{Batch2D, Batch3D, CullMode, PrimitiveMode};
    pub use crate::{D3Camera, D3FirstPCamera, D3IsoCamera, D3OrbitCamera};
    pub use crate::{GridShader, Shader, VGrayGradientShader};
    pub use crate::{
        Rect, Scene, SceneManager, SceneManagerCmd, SceneManagerResult, Value, ValueContainer,
    };
    pub use crate::{RepeatMode, SampleMode, Texture};
    pub use crate::{pixel_to_vec4, vec4_to_pixel};
}
