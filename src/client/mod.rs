pub mod command;
pub mod daylight;
pub mod draw2d;
pub mod meta;

use crate::prelude::*;
use crate::{client::meta::WidgetGameMeta, Command, D2PreviewBuilder, Daylight};
use draw2d::Draw2D;
use fontdue::*;
use theframework::prelude::*;
use toml::*;

pub struct Client {
    pub curr_map_id: Uuid,

    pub builder_d2: D2PreviewBuilder,

    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub scene_d2: Scene,
    pub scene_d3: Scene,

    pub scene_game: Scene,

    pub animation_frame: usize,
    pub server_time: TheTime,

    pub daylight: Daylight,

    pub messages_font: Option<Font>,
    pub messages_font_size: f32,
    pub messages_font_color: Pixel,

    pub draw2d: Draw2D,

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, TheTime)>,

    // Name of player entity templates
    player_entities: Vec<String>,

    current_map: String,
    current_screen: String,

    config: toml::Table,

    viewport: Vec2<i32>,

    // The offset we copy the target into
    pub target_offset: Vec2<i32>,

    // The target we render into
    target: TheRGBABuffer,

    // The meta data for widgets
    game_widgets: FxHashMap<Uuid, WidgetGameMeta>,
    widgets: FxHashMap<Uuid, bool>,
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
            scene_game: Scene::default(),

            animation_frame: 0,
            server_time: TheTime::default(),

            daylight: Daylight::default(),

            messages_font: None,
            draw2d: Draw2D::default(),

            messages_font_size: 15.0,
            messages_font_color: [229, 229, 1, 255],

            messages_to_draw: FxHashMap::default(),

            player_entities: Vec::new(),

            current_map: String::new(),
            current_screen: String::new(),

            config: toml::Table::default(),
            viewport: Vec2::zero(),

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            widgets: FxHashMap::default(),
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

    /// Get an i32 config value
    fn get_config_i32_default(&self, table: &str, key: &str, default: i32) -> i32 {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_integer() {
                    return v as i32;
                }
            }
        }
        default
    }

    fn _get_config_f32_default(&self, table: &str, key: &str, default: f32) -> f32 {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_float() {
                    return v as f32;
                }
            }
        }
        default
    }

    fn get_config_bool_default(&self, table: &str, key: &str, default: bool) -> bool {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_bool() {
                    return v;
                }
            }
        }
        default
    }

    fn get_config_string_default(&self, table: &str, key: &str, default: &str) -> String {
        if let Some(game) = self.config.get(table).and_then(toml::Value::as_table) {
            if let Some(value) = game.get(key) {
                if let Some(v) = value.as_str() {
                    return v.to_string();
                }
            }
        }
        default.to_string()
    }

    /// Setup the client with the given assets.
    pub fn setup(&mut self, assets: &Assets) -> Vec<Command> {
        let mut commands = vec![];

        // Init config
        match assets.config.parse::<Table>() {
            Ok(data) => {
                self.config = data;
            }
            Err(err) => {
                eprintln!("Client: Error parsing config: {}", err);
            }
        }

        // Get all player entities
        for (name, character) in assets.entities.iter() {
            match character.1.parse::<Table>() {
                Ok(data) => {
                    if let Some(game) = data.get("attributes").and_then(toml::Value::as_table) {
                        if let Some(value) = game.get("player") {
                            if let Some(v) = value.as_bool() {
                                if v {
                                    self.player_entities.push(name.to_string());
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Client: Error parsing entity {}: {}", name, err);
                }
            }
        }

        self.viewport = Vec2::new(
            self.get_config_i32_default("viewport", "width", 1280),
            self.get_config_i32_default("viewport", "height", 720),
        );

        // Create the target buffer
        self.target = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));

        // Find the start region
        self.current_map = self.get_config_string_default("game", "start_region", "");

        // Find the start screen
        self.current_screen = self.get_config_string_default("game", "start_screen", "");

        // Auto Init Players
        let auto_init_player = self.get_config_bool_default("game", "auto_create_player", false);
        if let Some(map) = assets.maps.get(&self.current_map) {
            if auto_init_player {
                for entity in map.entities.iter() {
                    if let Some(class_name) = entity.get_attr_string("class_name") {
                        if self.player_entities.contains(&class_name) {
                            commands.push(Command::CreateEntity(map.id, entity.clone()));
                            break;
                        }
                    }
                }
            }
        }

        // Init the meta data for widgets
        self.game_widgets.clear();
        self.widgets.clear();
        if let Some(screen) = assets.screens.get(&self.current_screen) {
            for widget in screen.sectors.iter() {
                if let Some(crate::Value::Str(data)) = widget.properties.get("data") {
                    if let Ok(table) = data.parse::<Table>() {
                        let mut role = "none";
                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            if let Some(value) = ui.get("role") {
                                if let Some(v) = value.as_str() {
                                    role = v;
                                }
                            }
                        }

                        if role == "game" {
                            let mut game_meta = WidgetGameMeta {};

                            self.game_widgets.insert(widget.creator_id, game_meta);
                        }
                    }
                }
            }
        }

        commands
    }

    /// Draw the game into the internal buffer
    pub fn draw_game(&mut self, assets: &Assets) {
        if let Some(screen) = assets.screens.get(&self.current_screen) {
            for sector in screen.sectors.iter() {
                let bb = sector.bounding_box(screen);
                let x = ((bb.min.x + self.viewport.x as f32 / screen.grid_size / 2.0)
                    * screen.grid_size)
                    .floor() as i32;
                let y = ((bb.min.y + self.viewport.y as f32 / screen.grid_size / 2.0)
                    * screen.grid_size)
                    .floor() as i32;
                let width = ((bb.max.x - bb.min.x) * screen.grid_size).floor() as i32;
                let height = ((bb.max.y - bb.min.y) * screen.grid_size).floor() as i32;

                let mut buffer = TheRGBABuffer::new(TheDim::sized(width, height));
                // buffer.fill([255, 255, 255, 255]);

                // First process the game widgets
                if let Some(widget) = self.game_widgets.get(&sector.creator_id) {
                    if let Some(map) = assets.maps.get(&self.current_map) {
                        self.draw_d2(map, buffer.pixels_mut(), width as usize, height as usize);
                    }
                }

                self.target.copy_into(x, y, &buffer);
                // println!("{} {} {} {}", x, y, width, height);
            }
        }
    }

    /// Copy the game buffer into the external buffer
    pub fn insert_game_buffer(&mut self, buffer: &mut TheRGBABuffer) {
        buffer.fill([0, 0, 0, 255]);
        buffer.copy_into(self.target_offset.x, self.target_offset.y, &self.target);
    }
}
