pub mod entity;
pub mod region;

use crate::Region;

///
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
}
