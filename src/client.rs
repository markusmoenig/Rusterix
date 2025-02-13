pub mod daylight;

use crate::prelude::*;
use crate::D2PreviewBuilder;
use crate::Daylight;
use theframework::prelude::*;

pub struct Client {
    pub curr_map_id: Uuid,

    pub builder_d2: D2PreviewBuilder,

    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub scene_d2: Scene,
    pub scene_d3: Scene,

    pub animation_frame: usize,
    pub server_time: TheTime,

    pub daylight: Daylight,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            curr_map_id: Uuid::default(),

            builder_d2: D2PreviewBuilder::new(),

            camera_d3: Box::new(D3FirstPCamera::new()),
            builder_d3: D3Builder::new(),

            scene_d2: Scene::default(),
            scene_d3: Scene::default(),

            animation_frame: 0,
            server_time: TheTime::default(),

            daylight: Daylight::default(),
        }
    }

    /// Increase the anim counter.
    pub fn inc_animation_frame(&mut self) {
        self.animation_frame += 1;
    }

    /// Set the current map id.
    pub fn set_curr_map_id(&mut self, id: Uuid) {
        self.curr_map_id = id;
    }

    /// Set the D3 Camera
    pub fn set_camera_d3(&mut self, camera: Box<dyn D3Camera>) {
        self.camera_d3 = camera;
    }

    /// Build the 2D scene from the map.
    pub fn build_scene_d2(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        assets: &Assets,
        values: &ValueContainer,
    ) {
        self.curr_map_id = map.id;
        self.scene_d2 = self.builder_d2.build(
            map,
            &assets.tiles,
            Texture::from_color(crate::BLACK),
            screen_size,
            &self.camera_d3.id(),
            values,
        );
    }

    /// Build the 3D scene from the map.
    pub fn build_scene_d3(&mut self, map: &Map, assets: &Assets, values: &ValueContainer) {
        self.curr_map_id = map.id;
        self.scene_d3 = self.builder_d3.build(
            map,
            &assets.tiles,
            Texture::from_color(crate::BLACK),
            Vec2::zero(), // Only needed for 2D builders
            &self.camera_d3.id(),
            values,
        );
    }

    /// Apply the entities to the 3D scene.
    pub fn apply_entities_items_d3(
        &mut self,
        entities: &[Entity],
        items: &[Item],
        assets: &Assets,
        values: &ValueContainer,
    ) {
        for entity in entities {
            if entity.is_player() {
                entity.apply_to_camera(&mut self.camera_d3);
            }
        }
        self.builder_d3.build_entities_items_d3(
            entities,
            items,
            self.camera_d3.as_ref(),
            &assets.tiles,
            &mut self.scene_d3,
            values,
        );
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, pixels: &mut [u8], width: usize, height: usize) {
        self.scene_d2.animation_frame = self.animation_frame;
        let ac = self
            .daylight
            .daylight(self.server_time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::Ambient);
        light.set_color([ac.x, ac.y, ac.z]);
        light.set_intensity(1.0);
        self.scene_d2.dynamic_lights.push(light);

        let mut rast = Rasterizer::setup(None, Mat4::identity(), Mat4::identity());
        rast.mapmini = self.scene_d2.mapmini.clone();

        rast.rasterize(&mut self.scene_d2, pixels, width, height, 200);
    }

    /// Draw the 3D scene.
    pub fn draw_d3(&mut self, pixels: &mut [u8], width: usize, height: usize) {
        self.scene_d3.animation_frame = self.animation_frame;
        let ac = self
            .daylight
            .daylight(self.server_time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::Ambient);
        light.set_color([ac.x, ac.y, ac.z]);
        light.set_intensity(1.0);

        self.scene_d3.dynamic_lights.push(light);
        let mut rast = Rasterizer::setup(
            None,
            self.camera_d3.view_matrix(),
            self.camera_d3
                .projection_matrix(width as f32, height as f32),
        );
        rast.mapmini = self.scene_d2.mapmini.clone();
        rast.rasterize(&mut self.scene_d3, pixels, width, height, 64);
    }
}
