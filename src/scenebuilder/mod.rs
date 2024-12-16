pub mod d2preview;

use crate::{Map, Scene};

#[allow(unused)]
pub trait SceneBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(&self, map: &Map) -> Scene;
}
