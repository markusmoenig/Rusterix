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
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, properties: &ValueContainer) {
        self.scene = self.builder_d2.build(map, assets, self.size, properties);
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
    pub fn draw_d2(&mut self, map: &Map, time: &TheTime) {
        pub fn map_grid_to_local(
            screen_size: Vec2<f32>,
            grid_pos: Vec2<f32>,
            map: &Map,
        ) -> Vec2<f32> {
            let grid_space_pos = grid_pos * map.grid_size;
            grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        }

        let width = self.buffer.dim().width as usize;
        let height = self.buffer.dim().height as usize;

        let screen_size = Vec2::new(width as f32, height as f32);

        let ac = self.daylight.daylight(time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::AmbientDaylight);
        light.set_color([ac.x, ac.y, ac.z]);
        light.set_intensity(1.0);
        self.scene.dynamic_lights.push(light);

        let player_offset =
            map_grid_to_local(screen_size, self.player_pos + Vec2::new(-0.5, 0.5), map);

        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            -player_offset.x + screen_size.x / 2.0,
            -player_offset.y + screen_size.y / 1.0,
        ));
        let scale_matrix = Mat3::new(
            map.grid_size,
            0.0,
            0.0,
            0.0,
            map.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity());
        rast.mapmini = self.scene.mapmini.clone();

        rast.rasterize(
            &mut self.scene,
            self.buffer.pixels_mut(),
            width,
            height,
            200,
        );

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
