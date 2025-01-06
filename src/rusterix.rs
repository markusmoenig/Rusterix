use crate::prelude::*;
use vek::Vec2;

pub enum ClientDrawMode {
    D2,
    D3,
}

use ClientDrawMode::*;

/// Rusterix can server as a server or client or both for solo games.
pub struct Rusterix {
    pub assets: Assets,
    pub server: Server,
    pub client: Client,

    pub is_dirty: bool,
    pub draw_mode: ClientDrawMode,
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
            client: Client::default(),

            is_dirty: true,
            draw_mode: ClientDrawMode::D3,
        }
    }

    /// Set to 2D mode.
    pub fn set_d2(&mut self) {
        self.draw_mode = D2;
    }

    /// Set to 3D mode.
    pub fn set_d3(&mut self) {
        self.draw_mode = D3;
    }

    /// Set the dirty flag, i.e. scene needs to be rebuild.
    pub fn set_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets
    }

    /// Create the server regions.
    pub fn create_regions(&mut self) {
        for (name, map) in &self.assets.maps {
            self.server
                .create_region_instance(name.clone(), map.clone(), &self.assets.entities);
        }
    }

    /// Build the client scene.
    pub fn build_scene(&mut self, screen_size: Vec2<f32>, map: &Map) {
        if self.is_dirty {
            match self.draw_mode {
                D2 => {
                    self.client.build_scene_d2(screen_size, map, &self.assets);
                }
                D3 => {
                    self.client.build_scene_d3(map, &self.assets);
                }
            }
        }
        self.is_dirty = false;
    }

    /// Draw the client scene.
    pub fn draw_scene(&mut self, pixels: &mut [u8], width: usize, height: usize) {
        match self.draw_mode {
            D2 => {
                self.client.draw_d2(pixels, width, height);
            }
            D3 => {
                self.client.draw_d3(pixels, width, height);
            }
        }
    }
}
