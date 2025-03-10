pub mod daylight;
pub mod draw2d;

use crate::prelude::*;
use crate::D2PreviewBuilder;
use crate::Daylight;
use draw2d::Draw2D;
use fontdue::*;
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

    pub messages_font: Option<Font>,
    pub messages_font_size: f32,
    pub messages_font_color: Pixel,

    pub draw2d: Draw2D,

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, TheTime)>,
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

            messages_font: None,
            draw2d: Draw2D::default(),

            messages_font_size: 15.0,
            messages_font_color: [229, 229, 1, 255],

            messages_to_draw: FxHashMap::default(),
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
        self.scene_d2 = self.builder_d2.build(map, assets, screen_size, values);
    }

    /// Apply the entities to the 2D scene.
    pub fn apply_entities_items_d2(&mut self, screen_size: Vec2<f32>, map: &Map, assets: &Assets) {
        self.builder_d2
            .build_entities_items(map, assets, &mut self.scene_d2, screen_size);
    }

    /// Build the 3D scene from the map.
    pub fn build_scene_d3(&mut self, map: &Map, assets: &Assets, values: &ValueContainer) {
        self.curr_map_id = map.id;
        self.scene_d3 = self.builder_d3.build(
            map,
            assets,
            Vec2::zero(), // Only needed for 2D builders
            &self.camera_d3.id(),
            values,
        );
    }

    /// Apply the entities to the 3D scene.
    pub fn apply_entities_items_d3(&mut self, map: &Map, assets: &Assets) {
        for entity in &map.entities {
            if entity.is_player() {
                entity.apply_to_camera(&mut self.camera_d3);
            }
        }
        self.builder_d3.build_entities_items(
            map,
            self.camera_d3.as_ref(),
            assets,
            &mut self.scene_d3,
        );
    }

    /// Process messages from the server to be displayed after drawing.
    pub fn process_messages(
        &mut self,
        map: &Map,
        messages: Vec<(Option<u32>, Option<u32>, u32, String)>,
    ) {
        // Remove expired messages
        let expired_keys: Vec<_> = self
            .messages_to_draw
            .iter()
            .filter(|(_, (_, _, _, expire_time))| *expire_time < self.server_time)
            .map(|(id, _)| *id)
            .collect();

        for id in expired_keys {
            self.messages_to_draw.remove(&id);
        }

        // Add new messages
        for (sender_entity_id, sender_item_id, _, message) in messages {
            if let Some(sender_item_id) = sender_item_id {
                for item in &map.items {
                    if item.id == sender_item_id {
                        if let Some(font) = &self.messages_font {
                            let text_size =
                                self.draw2d
                                    .get_text_size(font, self.messages_font_size, &message);

                            let ticks = self.server_time.to_ticks(4);
                            let expire_time = TheTime::from_ticks(ticks + 4, 4);

                            self.messages_to_draw.insert(
                                sender_item_id,
                                (item.get_pos_xz(), message.clone(), text_size.0, expire_time),
                            );
                        }
                    }
                }
            } else if let Some(sender_entity_id) = sender_entity_id {
                for entity in &map.entities {
                    if entity.id == sender_entity_id {
                        if let Some(font) = &self.messages_font {
                            let text_size =
                                self.draw2d
                                    .get_text_size(font, self.messages_font_size, &message);

                            let ticks = self.server_time.to_ticks(4);
                            let expire_time = TheTime::from_ticks(ticks + 4, 4);

                            self.messages_to_draw.insert(
                                sender_entity_id,
                                (
                                    entity.get_pos_xz(),
                                    message.clone(),
                                    text_size.0,
                                    expire_time,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, map: &Map, pixels: &mut [u8], width: usize, height: usize) {
        pub fn map_grid_to_local(
            screen_size: Vec2<f32>,
            grid_pos: Vec2<f32>,
            map: &Map,
        ) -> Vec2<f32> {
            let grid_space_pos = grid_pos * map.grid_size;
            grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        }

        let screen_size = Vec2::new(width as f32, height as f32);

        self.scene_d2.animation_frame = self.animation_frame;
        let ac = self
            .daylight
            .daylight(self.server_time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::AmbientDaylight);
        light.set_color([ac.x, ac.y, ac.z]);
        light.set_intensity(1.0);
        self.scene_d2.dynamic_lights.push(light);

        let translation_matrix = Mat3::<f32>::translation_2d(Vec2::new(
            map.offset.x + screen_size.x / 2.0,
            -map.offset.y + screen_size.y / 2.0,
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
        rast.mapmini = self.scene_d2.mapmini.clone();

        rast.rasterize(&mut self.scene_d2, pixels, width, height, 200);

        // Draw Messages

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
        }
    }

    /// Draw the 3D scene.
    pub fn draw_d3(&mut self, pixels: &mut [u8], width: usize, height: usize) {
        self.scene_d3.animation_frame = self.animation_frame;
        let ac = self
            .daylight
            .daylight(self.server_time.total_minutes(), 0.0, 1.0);

        let mut light = Light::new(LightType::AmbientDaylight);
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
        rast.background_color = Some(vec4_to_pixel(&Vec4::new(ac.x, ac.y, ac.z, 1.0)));
        rast.rasterize(&mut self.scene_d3, pixels, width, height, 64);
    }
}
