use crate::client::interpolation::*;
use crate::prelude::*;
use crate::{D2Builder, Daylight, PlayerCamera, Rect};
use theframework::prelude::*;
use vek::Vec2;

pub struct GameWidget {
    pub builder_d2: D2Builder,
    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub rect: Rect,

    pub scene: Scene,
    pub daylight: Daylight,

    pub buffer: TheRGBABuffer,

    pub map_bbox: Vec4<f32>,

    pub grid_size: f32,
    pub top_left: Vec2<f32>,

    pub interpolation: InterpolationBuffer,

    pub toml_str: String,
    pub table: toml::Table,

    pub camera: PlayerCamera,

    // Used to detect region changes (have to rebuild the geometry)
    pub build_region_name: String,
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

            rect: Rect::default(),

            scene: Scene::default(),
            daylight: Daylight::default(),

            buffer: TheRGBABuffer::default(),

            map_bbox: Vec4::zero(),

            grid_size: 32.0,
            top_left: Vec2::zero(),

            interpolation: InterpolationBuffer::default(),

            toml_str: String::new(),
            table: toml::Table::default(),

            camera: PlayerCamera::D2,

            build_region_name: String::new(),
        }
    }

    pub fn init(&mut self) {
        if let Ok(table) = self.toml_str.parse::<toml::Table>() {
            if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                if let Some(value) = ui.get("grid_size") {
                    if let Some(v) = value.as_integer() {
                        self.grid_size = v as f32;
                    }
                }
            }
            self.table = table;
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets) {
        if let Some(bbox) = map.bounding_box() {
            self.map_bbox = bbox;
        }

        if self.camera == PlayerCamera::D2 {
            self.scene = self.builder_d2.build(map, assets, self.rect.size());
        } else {
            self.scene = self.builder_d3.build(
                map,
                assets,
                Vec2::zero(),
                &self.camera_d3.id(),
                &ValueContainer::default(),
            );
        }
        self.build_region_name = map.name.clone();
    }

    pub fn apply_entities(&mut self, map: &Map, assets: &Assets) {
        for entity in map.entities.iter() {
            if entity.is_player() {
                if let Some(Value::PlayerCamera(camera)) = entity.attributes.get("player_camera") {
                    if *camera != self.camera {
                        self.camera = camera.clone();
                        if self.camera == PlayerCamera::D3Iso {
                            self.camera_d3 = Box::new(D3IsoCamera::new())
                        } else if self.camera == PlayerCamera::D3FirstP {
                            self.camera_d3 = Box::new(D3FirstPCamera::new());
                        }
                        self.build(map, assets);
                    }
                }

                if self.camera != PlayerCamera::D2 {
                    entity.apply_to_camera(&mut self.camera_d3);
                }

                self.interpolation.add_position(entity.get_pos_xz());
                break;
            }
        }

        if self.camera == PlayerCamera::D2 {
            self.builder_d2
                .build_entities_items(map, assets, &mut self.scene, self.rect.size());
        } else {
            self.builder_d3.build_entities_items(
                map,
                self.camera_d3.as_ref(),
                assets,
                &mut self.scene,
            );
        }
    }

    pub fn draw(&mut self, map: &Map, time: &TheTime, assets: &Assets) {
        if map.name != self.build_region_name {
            self.build(map, assets);
        }

        if self.camera == PlayerCamera::D2 {
            self.draw_d2(map, time, assets);
        } else {
            let width = self.buffer.dim().width as usize;
            let height = self.buffer.dim().height as usize;
            // let ac = self.daylight.daylight(time.total_minutes(), 0.0, 1.0);

            // let mut light = Light::new(LightType::AmbientDaylight);
            // light.set_color([ac.x, ac.y, ac.z]);
            // light.set_intensity(1.0);

            // self.scene.dynamic_lights.push(light);
            let mut rast = Rasterizer::setup(
                None,
                self.camera_d3.view_matrix(),
                self.camera_d3
                    .projection_matrix(width as f32, height as f32),
            );
            rast.mapmini = self.scene.mapmini.clone();
            // rast.background_color = Some(vec4_to_pixel(&Vec4::new(ac.x, ac.y, ac.z, 1.0)));
            rast.rasterize(
                &mut self.scene,
                self.buffer.pixels_mut(),
                width,
                height,
                40,
                assets,
            );
        }
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, _map: &Map, _time: &TheTime, assets: &Assets) {
        let width = self.buffer.dim().width as usize;
        let height = self.buffer.dim().height as usize;

        let screen_size = Vec2::new(width as f32, height as f32);

        // let ac = self.daylight.daylight(time.total_minutes(), 0.0, 1.0);

        // let mut light = Light::new(LightType::AmbientDaylight);
        // light.set_color([ac.x, ac.y, ac.z]);
        // light.set_intensity(1.0);
        // self.scene.dynamic_lights.push(light.compile());

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
        let mut camera_pos = self.interpolation.get_interpolated() * self.grid_size;

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

        self.top_left = (camera_pos - screen_size / 2.0).floor() / self.grid_size;

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

        rast.rasterize(
            &mut self.scene,
            self.buffer.pixels_mut(),
            width,
            height,
            40,
            assets,
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
