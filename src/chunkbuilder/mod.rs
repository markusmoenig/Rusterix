pub mod d2preview;

use crate::{Assets, Chunk, Map};

/// The ChunkBuilder Trait
#[allow(unused)]
pub trait ChunkBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(&mut self, map: &Map, assets: &Assets, chunk: &mut Chunk) {}
}
