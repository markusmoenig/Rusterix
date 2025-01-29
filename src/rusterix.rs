use crate::prelude::*;
use crate::MapCamera;
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

    pub is_dirty_d2: bool,
    pub is_dirty_d3: bool,
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

            is_dirty_d2: true,
            is_dirty_d3: true,
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
        self.is_dirty_d2 = true;
        self.is_dirty_d3 = true;
    }

    /// Set the assets
    pub fn set_assets(&mut self, assets: Assets) {
        self.assets = assets
    }

    /// Create the server regions.
    pub fn create_regions(&mut self) {
        for (name, map) in &self.assets.maps {
            self.server
                .create_region_instance(name.clone(), map.clone(), &self.assets);
        }
        self.server.set_state(crate::ServerState::Running);
    }

    /// Build the client scene based on the maps camera mode.
    pub fn build_scene(&mut self, screen_size: Vec2<f32>, map: &Map, values: &ValueContainer) {
        if map.camera == MapCamera::TwoD {
            if self.is_dirty_d2 {
                self.client
                    .build_scene_d2(screen_size, map, &self.assets, values);
                self.is_dirty_d2 = false;
            }
            self.set_d2();
        } else {
            if self.is_dirty_d3 {
                self.client.build_scene_d3(map, &self.assets, values);
                self.is_dirty_d3 = false;
            }
            self.set_d3();
        }
    }

    /// Apply the entities to the 3D scene.
    pub fn apply_entities(&mut self, entities: &[Entity], map: &Map, assets: &Assets) {
        if map.camera != MapCamera::TwoD {
            self.client.apply_entities_d3(entities, assets);
        }
    }

    /// Build the client scene in D3.
    pub fn build_scene_d3(&mut self, map: &Map) {
        if self.is_dirty_d3 {
            self.client
                .build_scene_d3(map, &self.assets, &ValueContainer::default());
            self.is_dirty_d3 = false;
        }
        self.set_d3();
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
