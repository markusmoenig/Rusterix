pub mod action;
pub mod d2chunkbuilder;
pub mod d3chunkbuilder;
pub mod surface_mesh_builder;

use crate::{Assets, Chunk, Map};

/// The ChunkBuilder Trait
#[allow(unused)]
pub trait ChunkBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder>;
}
