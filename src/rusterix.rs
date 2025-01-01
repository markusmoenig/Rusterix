use crate::prelude::*;

/// Rusterix can server as a server or client or both for solo games.
pub struct Rusterix {
    pub assets: Assets,
    pub server: Server,
}

impl Default for Rusterix {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusterix {
    pub fn new() -> Self {
        Self {
            assets: Assets::default(),
            server: Server::default(),
        }
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets
    }

    /// Create the server regions.
    pub fn create_regions(&mut self) {
        for (name, map) in &self.assets.maps {
            self.server
                .create_region(name.clone(), map.clone(), &self.assets.entities);
        }
    }
}
