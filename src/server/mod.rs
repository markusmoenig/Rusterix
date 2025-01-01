pub mod assets;
pub mod entity;
pub mod region;

use theframework::prelude::FxHashMap;

use crate::prelude::*;

pub struct Server {
    pub regions: Vec<Region>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        Self { regions: vec![] }
    }

    /// Create the given region.
    pub fn create_region(&mut self, name: String, map: Map, entities: &FxHashMap<String, String>) {
        let mut region = Region::default();
        region.init(name, map, entities);
        self.regions.push(region);
    }
}
