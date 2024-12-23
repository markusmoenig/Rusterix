pub mod batchd2;
pub mod batchd3;

use crate::{Edge, Pixel, Rect, RepeatMode, SampleMode};

/// The primitive mode. The rasterizer can draw triangles and lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrimitiveMode {
    /// Draw as triangles.
    Triangles,
    /// Draw connected vertices / points.
    Lines,
    /// Draw a line strip around the triangles.
    LineStrip,
    /// Draw a closed line strip around the triangles.
    LineLoop,
}

/// The CullMode of the batch, Off by default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CullMode {
    /// Render all faces
    Off,
    /// Cull front-facing triangles
    Front,
    /// Cull back-facing triangles
    Back,
}

pub struct Batch<T> {
    // Render mode: triangles or lines
    pub mode: PrimitiveMode,

    /// 2D or 3D input vertices which will get projected into 2D space. 2D and 3D vertices expect 3D and 4D vecs with the last component set to 1.0.
    vertices: Vec<T>,

    /// The indices of the vertices of the batch.
    pub indices: Vec<(usize, usize, usize)>,

    /// The UVs of the batch.
    pub uvs: Vec<[f32; 2]>,

    /// Projected vertices
    pub projected_vertices: Vec<T>,

    /// 2D Bounding box of the projected vertices of the batch.
    pub bounding_box: Option<Rect>,

    /// Precomputed edges
    pub edges: Vec<[Edge; 3]>,

    /// Color, used for lines.
    pub color: Pixel,

    /// SampleMode, default is Nearest.
    pub sample_mode: SampleMode,

    /// RepeatMode, default is ClampXY.
    pub repeat_mode: RepeatMode,

    /// CullMode, default is None.
    pub cull_mode: CullMode,

    /// Texture index. Specifies the texture index into the texture array during rasterization for this batch. Default is 0.
    pub texture_index: usize,

    pub clipped_indices: Vec<(usize, usize, usize)>,
    pub clipped_uvs: Vec<[f32; 2]>,
}
