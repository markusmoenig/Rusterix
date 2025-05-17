use crate::{Batch, CompiledLight, MapMini, Shader, Terrain, Tile};
use rayon::prelude::*;
use vek::{Mat3, Mat4};

/// A chunk of 2D and 3D batches which make up a Scene.
pub struct Chunk {
    pub coord: (i32, i32),
    pub batches2d: Vec<Batch<[f32; 2]>>,
    pub batches3d: Vec<Batch<[f32; 4]>>,
}

impl Chunk {
    /// Create an empty chunk at the given coordinate.
    pub fn new(coord: (i32, i32)) -> Self {
        Self {
            coord,
            batches2d: vec![],
            batches3d: vec![],
        }
    }
}
