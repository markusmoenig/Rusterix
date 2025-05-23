pub mod batch2d;
pub mod batch3d;

// pub mod batchd2;
// pub mod batchd3;

use crate::{Edges, Material, Pixel, Rect, RepeatMode, SampleMode};
use vek::{Mat3, Mat4, Vec3};

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

#[derive(Debug, Clone)]
pub struct Batch<T> {
    // Render mode: triangles or lines
    pub mode: PrimitiveMode,

    /// 2D or 3D input vertices which will get projected into 2D space. 2D and 3D vertices expect 3D and 4D vecs with the last component set to 1.0.
    pub vertices: Vec<T>,

    /// The indices of the vertices of the batch.
    pub indices: Vec<(usize, usize, usize)>,

    /// The UVs of the batch.
    pub uvs: Vec<[f32; 2]>,

    /// Projected vertices
    pub projected_vertices: Vec<T>,

    /// 2D Bounding box of the projected vertices of the batch.
    pub bounding_box: Option<Rect>,

    /// Precomputed edges
    pub edges: Vec<Edges>,

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

    // Output after clipping and projection
    pub clipped_indices: Vec<(usize, usize, usize)>,
    pub clipped_uvs: Vec<[f32; 2]>,

    /// 2D Transform matrix
    pub transform_2d: Mat3<f32>,

    /// 3D Transform matrix
    pub transform_3d: Mat4<f32>,

    /// Indicates whether the batch receives lighting. True by default. Turn off for skybox etc.
    pub receives_light: bool,

    /// Normals, only apply to the 3D case.
    pub normals: Vec<Vec3<f32>>,

    /// Clipped normals
    pub clipped_normals: Vec<Vec3<f32>>,

    // Material
    pub material: Option<Material>,
}
