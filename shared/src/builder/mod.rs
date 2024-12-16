use rusterix::{Map, Scene};

#[allow(unused)]
pub trait MapBuilder: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    fn build(&self, map: &Map) -> Scene;
}
