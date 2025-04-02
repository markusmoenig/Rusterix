use crate::prelude::*;
use crate::{D2Builder, Daylight};
use theframework::prelude::*;
use vek::Vec2;

pub struct GameWidget {
    pub builder_d2: D2Builder,
    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub position: Vec2<i32>,
    pub size: Vec2<f32>,

    pub scene: Scene,
    pub daylight: Daylight,

    pub buffer: TheRGBABuffer,

    pub player_pos: Vec2<f32>,

    pub map_bbox: Vec4<f32>,

    pub grid_size: f32,
}

impl Default for GameWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GameWidget {
    pub fn new() -> Self {
        Self {
            builder_d2: D2Builder::new(),

            camera_d3: Box::new(D3FirstPCamera::new()),
            builder_d3: D3Builder::new(),

            position: Vec2::zero(),
            size: Vec2::zero(),

            scene: Scene::default(),
            daylight: Daylight::default(),

            buffer: TheRGBABuffer::default(),

            player_pos: Vec2::zero(),

            map_bbox: Vec4::zero(),

            grid_size: 32.0,
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }
        self.scene = self.builder_d2.build(map, assets, self.size);
    }

    pub fn apply_entities(&mut self, map: &Map, assets: &Assets) {
        for entity in map.entities.iter() {
            if entity.is_player() {
                self.player_pos = entity.get_pos_xz();
                break;
            }
        }
        self.builder_d2
            .build_entities_items(map, assets, &mut self.scene, self.size);
    }

    pub fn draw(&mut self, map: &Map, time: &TheTime) {
        self.draw_d2(map, time);
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, _map: &Map, time: &TheTime) {
        let width = self.buffer.dim().width as usize;
        let height = self.buffer.dim().height as usize;

        let screen_size = Vec2::new(width as f32, height as f32);

        let ac = self.daylight.daylight(time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::AmbientDaylight);
        light.set_color([ac.x, ac.y, ac.z]);
        light.set_intensity(1.0);
        self.scene.dynamic_lights.push(light);

        //println!("draw grid_size {}", map.grid_size);

        // let player_world_pos = self.player_pos * map.grid_size;
        // let translation_matrix =
        //     Mat3::<f32>::translation_2d((screen_size / 2.0 - player_world_pos).floor());

        let bbox = self.map_bbox;

        let start = Vec2::new(bbox.x, bbox.y);
        let end = Vec2::new(bbox.x + bbox.z, bbox.y + bbox.w);

        let start_pixels = start * self.grid_size;
        let end_pixels = end * self.grid_size;

        // Ensure min < max even if grid_size has negative components
        let min_world = Vec2::new(
            start_pixels.x.min(end_pixels.x),
            start_pixels.y.min(end_pixels.y),
        );
        let max_world = Vec2::new(
            start_pixels.x.max(end_pixels.x),
            start_pixels.y.max(end_pixels.y),
        );

        let half_screen = screen_size / 2.0;

        // Compute unclamped camera center in world space
        let mut camera_pos = self.player_pos * self.grid_size;

        let map_width_px = max_world.x - min_world.x;
        let map_height_px = max_world.y - min_world.y;

        if map_width_px > screen_size.x {
            camera_pos.x = camera_pos
                .x
                .clamp(min_world.x + half_screen.x, max_world.x - half_screen.x);
        } else {
            // Center map horizontally
            camera_pos.x = (min_world.x + max_world.x) / 2.0;
        }

        if map_height_px > screen_size.y {
            camera_pos.y = camera_pos
                .y
                .clamp(min_world.y + half_screen.y, max_world.y - half_screen.y);
        } else {
            // Center map vertically
            camera_pos.y = (min_world.y + max_world.y) / 2.0;
        }

        let translation_matrix =
            Mat3::<f32>::translation_2d((screen_size / 2.0 - camera_pos).floor());

        let scale_matrix = Mat3::new(
            self.grid_size,
            0.0,
            0.0,
            0.0,
            self.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity());
        rast.mapmini = self.scene.mapmini.clone();

        rast.rasterize(&mut self.scene, self.buffer.pixels_mut(), width, height, 40);

        // Draw Messages

        /*
        if let Some(font) = &self.messages_font {
            for (grid_pos, message, text_size, _) in self.messages_to_draw.values() {
                let position = map_grid_to_local(screen_size, *grid_pos, map);

                let tuple = (
                    position.x as isize - *text_size as isize / 2 - 5,
                    position.y as isize - self.messages_font_size as isize - map.grid_size as isize,
                    *text_size as isize + 10,
                    22,
                );

                self.draw2d.blend_rect_safe(
                    pixels,
                    &tuple,
                    width,
                    &[0, 0, 0, 128],
                    &(0, 0, width as isize, height as isize),
                );

                self.draw2d.text_rect_blend_safe(
                    pixels,
                    &tuple,
                    width,
                    font,
                    self.messages_font_size,
                    message,
                    &self.messages_font_color,
                    draw2d::TheHorizontalAlign::Center,
                    draw2d::TheVerticalAlign::Center,
                    &(0, 0, width as isize, height as isize),
                );
            }
        }*/
    }
}
