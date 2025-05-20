use crate::{BBox, Batch2D, Batch3D, CompiledLight};
use vek::Vec2;

/// A chunk of 2D and 3D batches which make up a Scene.
pub struct Chunk {
    pub origin: Vec2<i32>,
    pub size: i32,
    pub bbox: BBox,

    pub batches2d: Vec<Batch2D>,
    pub batches3d: Vec<Batch3D>,

    pub lights: Vec<CompiledLight>,
}

impl Chunk {
    /// Create an empty chunk at the given coordinate.
    pub fn new(origin: Vec2<i32>, size: i32) -> Self {
        let bbox = BBox::from_pos_size(origin.map(|v| v as f32), Vec2::broadcast(size as f32));
        Self {
            origin,
            size,
            bbox,
            batches2d: vec![],
            batches3d: vec![],
            lights: vec![],
        }
    }
}
