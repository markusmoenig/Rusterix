use crate::{Assets, Chunk, ChunkBuilder, Map};
use vek::{Vec2, Vec4};

pub struct D2PreviewChunkBuilder {}

impl ChunkBuilder for D2PreviewChunkBuilder {
    fn new() -> Self {
        Self {}
    }

    fn build(&mut self, map: &Map, assets: &Assets, chunk: &mut Chunk) {}
}
