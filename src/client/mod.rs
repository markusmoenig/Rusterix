pub mod action;
pub mod command;
pub mod daylight;
pub mod draw2d;
pub mod interpolation;
pub mod widget;

use std::str::FromStr;

use crate::prelude::*;
use crate::{
    AccumBuffer, BrushPreview, Command, D2PreviewBuilder, EntityAction, Rect, RenderMode,
    ShapeFXGraph, Tracer, Value,
    client::action::ClientAction,
    client::widget::{
        Widget, deco::DecoWidget, game::GameWidget, messages::MessagesWidget, screen::ScreenWidget,
        text::TextWidget,
    },
};
use draw2d::Draw2D;
use fontdue::*;
use std::sync::{Arc, Mutex};
use theframework::prelude::*;
use toml::*;

pub struct Client {
    pub curr_map_id: Uuid,

    pub builder_d2: D2PreviewBuilder,

    pub camera_d3: Box<dyn D3Camera>,
    pub builder_d3: D3Builder,

    pub scene_d2: Scene,
    pub scene_d3: Scene,

    pub scene: Scene,

    pub animation_frame: usize,
    pub server_time: TheTime,

    pub brush_preview: Option<BrushPreview>,

    /// Global render graph
    pub global: ShapeFXGraph,

    pub messages_font: Option<Font>,
    pub messages_font_size: f32,
    pub messages_font_color: Pixel,

    pub draw2d: Draw2D,

    pub messages_to_draw: FxHashMap<u32, (Vec2<f32>, String, usize, TheTime)>,

    // Name of player entity templates
    player_entities: Vec<String>,

    pub current_map: String,
    current_screen: String,

    config: toml::Table,

    pub viewport: Vec2<i32>,
    grid_size: f32,
    pub target_fps: i32,
    pub game_tick_ms: i32,

    // The offset we copy the target into
    pub target_offset: Vec2<i32>,

    // The target we render into
    target: TheRGBABuffer,

    // The UI overlay
    overlay: TheRGBABuffer,

    // The widgets
    game_widgets: FxHashMap<Uuid, GameWidget>,
    button_widgets: FxHashMap<u32, Widget>,
    text_widgets: FxHashMap<Uuid, TextWidget>,
    deco_widgets: FxHashMap<Uuid, DecoWidget>,

    messages_widget: Option<MessagesWidget>,

    // Button widgets which are active (clicked)
    activated_widgets: Vec<u32>,

    // Button widgets which are permanently active
    permanently_activated_widgets: Vec<u32>,

    /// Client Action
    client_action: Arc<Mutex<ClientAction>>,

    /// Hidden widgets,
    widgets_to_hide: Vec<String>,

    // Intent
    intent: String,
    key_down_intent: Option<String>,

    currencies: Currencies,

    first_game_draw: bool,
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

            scene: Scene::default(),

            animation_frame: 0,
            server_time: TheTime::default(),

            brush_preview: None,

            global: ShapeFXGraph::default(),

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
            grid_size: 32.0,
            target_fps: 30,
            game_tick_ms: 250,

            target_offset: Vec2::zero(),
            target: TheRGBABuffer::default(),
            overlay: TheRGBABuffer::default(),

            game_widgets: FxHashMap::default(),
            button_widgets: FxHashMap::default(),
            text_widgets: FxHashMap::default(),
            deco_widgets: FxHashMap::default(),
            messages_widget: None,

            activated_widgets: vec![],
            permanently_activated_widgets: vec![],
            widgets_to_hide: vec![],

            client_action: Arc::new(Mutex::new(ClientAction::default())),
            currencies: Currencies::default(),
            intent: String::new(),
            key_down_intent: None,

            first_game_draw: false,
        }
    }

    /// Increase the anim counter.
    pub fn inc_animation_frame(&mut self) {
        self.animation_frame += 1;

        for widget in self.game_widgets.values_mut() {
            widget.scene.animation_frame += 1;
        }
    }

    /// Set the server time
    pub fn set_server_time(&mut self, time: TheTime) {
        self.server_time = time;
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
    pub fn build_custom_scene_d2(
        &mut self,
        screen_size: Vec2<f32>,
        map: &Map,
        assets: &Assets,
        values: &ValueContainer,
    ) {
        self.curr_map_id = map.id;
        self.scene_d2 = self.builder_d2.build(map, assets, screen_size, values);
        self.builder_d2
            .build_entities_items(map, assets, &mut self.scene_d2, screen_size);
    }

    /// Apply the entities to the 2D scene.
    pub fn apply_entities_items_d2(&mut self, screen_size: Vec2<f32>, map: &Map, assets: &Assets) {
        self.builder_d2
            .build_entities_items(map, assets, &mut self.scene, screen_size);
    }

    /// Build the 3D scene from the map.
    pub fn build_custom_scene_d3(&mut self, map: &Map, assets: &Assets, values: &ValueContainer) {
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
        self.builder_d3
            .build_entities_items(map, self.camera_d3.as_ref(), assets, &mut self.scene);
    }

    /// Process messages from the server to be displayed after drawing.
    pub fn process_messages(&mut self, map: &Map, messages: Vec<crate::server::Message>) {
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
        for (sender_entity_id, sender_item_id, _, message, _category) in messages {
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
    pub fn draw_custom_d2(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        assets: &Assets,
    ) {
        self.scene.animation_frame = self.animation_frame;
        let screen_size = Vec2::new(width as f32, height as f32);
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

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .render_mode(RenderMode::render_2d());
        rast.render_graph = self.global.clone();
        rast.hour = self.server_time.to_f32();
        rast.mapmini = self.scene.mapmini.clone();
        rast.rasterize(&mut self.scene_d2, pixels, width, height, 64, assets);
    }

    /// Draw the 2D scene.
    pub fn draw_d2(
        &mut self,
        map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        assets: &Assets,
    ) {
        pub fn map_grid_to_local(
            screen_size: Vec2<f32>,
            grid_pos: Vec2<f32>,
            map: &Map,
        ) -> Vec2<f32> {
            let grid_space_pos = grid_pos * map.grid_size;
            grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        }

        self.scene.animation_frame = self.animation_frame;
        let screen_size = Vec2::new(width as f32, height as f32);
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

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
            .render_mode(RenderMode::render_2d());
        rast.render_graph = self.global.clone();
        rast.hour = self.server_time.to_f32();
        rast.mapmini = self.scene.mapmini.clone();
        rast.rasterize(&mut self.scene, pixels, width, height, 64, assets);

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
    pub fn draw_d3(
        &mut self,
        _map: &Map,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        assets: &Assets,
    ) {
        self.scene.animation_frame = self.animation_frame;

        let mut rast = Rasterizer::setup(
            None,
            self.camera_d3.view_matrix(),
            self.camera_d3
                .projection_matrix(width as f32, height as f32),
        )
        .render_mode(RenderMode::render_3d());
        rast.brush_preview = self.brush_preview.clone();
        rast.render_graph = self.global.clone();
        rast.hour = self.server_time.to_f32();
        rast.mapmini = self.scene.mapmini.clone();
        rast.rasterize(&mut self.scene, pixels, width, height, 64, assets)
    }

    /// Trace the 3D scene.
    pub fn trace(&mut self, accum: &mut AccumBuffer, assets: &Assets) {
        self.scene.animation_frame = self.animation_frame;
        let mut tracer = Tracer::default();
        tracer.render_graph = self.global.clone();
        tracer.hour = self.server_time.to_f32();
        tracer.trace(self.camera_d3.as_ref(), &mut self.scene, accum, 64, assets);
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
        self.first_game_draw = true;
        self.intent = String::new();

        // Init config
        match assets.config.parse::<Table>() {
            Ok(data) => {
                self.config = data;
            }
            Err(err) => {
                eprintln!("Client: Error parsing config: {}", err);
            }
        }

        let mut currencies = Currencies::default();
        _ = currencies.add_currency(Currency {
            name: "Gold".into(),
            symbol: "G".into(),
            exchange_rate: 1.0,
            max_limit: None,
        });
        currencies.base_currency = "G".to_string();
        self.currencies = currencies;

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

        self.target_fps = self.get_config_i32_default("game", "target_fps", 30);
        self.game_tick_ms = self.get_config_i32_default("game", "game_tick_ms", 250);
        self.grid_size = self.get_config_i32_default("viewport", "grid_size", 32) as f32;

        // Create the target buffer
        self.target = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));
        // Create the overlay buffer
        self.overlay = TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y));

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
                            // Init scripting for this entity
                            self.client_action = Arc::new(Mutex::new(ClientAction::default()));
                            self.client_action.lock().unwrap().init(class_name, assets);
                            break;
                        }
                    }
                }
            }
        } else {
            eprintln!("Did not find start map");
        }

        if let Some(screen) = assets.screens.get(&self.current_screen) {
            self.init_screen(screen, assets);
        } else {
            eprintln!("Did not find start screen");
        }

        commands
    }

    /// Draw the game into the internal buffer
    pub fn draw_game(&mut self, map: &Map, assets: &Assets, messages: Vec<crate::server::Message>) {
        let mut player_entity = Entity::default();

        // Reset the intent to the server value
        for entity in map.entities.iter() {
            if entity.is_player() {
                self.intent = entity.get_attr_string("intent").unwrap_or_default();
                player_entity = entity.clone();
            }
        }

        self.target.fill([0, 0, 0, 255]);
        // First process the game widgets
        for widget in self.game_widgets.values_mut() {
            widget.apply_entities(map, assets);
            widget.draw(map, &self.server_time, assets);

            self.target
                .copy_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
        }

        if let Some(screen) = assets.screens.get(&self.current_screen) {
            let mut widget = ScreenWidget {
                buffer: TheRGBABuffer::new(TheDim::sized(self.viewport.x, self.viewport.y)),
                ..Default::default()
            };

            let (start_x, start_y) = crate::utils::align_screen_to_grid(
                self.viewport.x as f32,
                self.viewport.y as f32,
                self.grid_size,
            );

            widget.builder_d2.activated_widgets = self.activated_widgets.clone();

            // Add the current intent to the activated widgets
            for w in self.button_widgets.iter() {
                if w.1.intent.is_some() && w.1.intent.as_ref().unwrap() == &self.intent {
                    widget.builder_d2.activated_widgets.push(w.0.clone());
                }
            }

            widget.offset = Vec2::new(start_x, start_y);

            widget.build(screen, assets);
            widget.draw(screen, &self.server_time, assets);

            self.target.blend_into(0, 0, &widget.buffer);
        }

        // Draw the deco widgets on top
        for widget in self.deco_widgets.values_mut() {
            widget.update_draw(&mut self.target, map, &self.currencies, assets);
            self.target
                .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
        }

        // Draw the messages on top
        if let Some(widget) = &mut self.messages_widget {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(&mut self.target, assets, messages);
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            }
        }

        // Draw the text widgets on top
        for widget in self.text_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(&mut self.target, map, &self.currencies, assets);
                self.target
                    .blend_into(widget.rect.x as i32, widget.rect.y as i32, &widget.buffer);
            }
        }

        // Draw the button widgets which support inventory / gear on top
        for widget in self.button_widgets.values_mut() {
            let hide = self.widgets_to_hide.iter().any(|pattern| {
                if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    widget.name.starts_with(prefix)
                } else {
                    widget.name == *pattern
                }
            });

            if !hide {
                widget.update_draw(
                    &mut self.target,
                    map,
                    assets,
                    &player_entity,
                    &self.draw2d,
                    &self.animation_frame,
                );
            }
        }
    }

    /// Copy the game buffer into the external buffer
    pub fn insert_game_buffer(&mut self, buffer: &mut TheRGBABuffer) {
        buffer.fill([30, 30, 30, 255]);
        if self.first_game_draw {
            let dim = buffer.dim();
            if dim.width > self.viewport.x {
                self.target_offset.x = (dim.width - self.viewport.x) / 2;
            }
            if dim.height > self.viewport.y {
                self.target_offset.y = (dim.height - self.viewport.y) / 2;
            }
            self.first_game_draw = false;
        }
        buffer.copy_into(self.target_offset.x, self.target_offset.y, &self.target);
    }

    /// Click / touch down event
    pub fn touch_down(&mut self, coord: Vec2<i32>, map: &Map) -> Option<EntityAction> {
        let mut action = None;

        let p = coord - self.target_offset;

        for (id, widget) in self.button_widgets.iter() {
            if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                self.activated_widgets.push(*id);

                if let Some(intent) = &widget.intent {
                    self.intent = intent.clone();
                    action = Some(EntityAction::Intent(intent.clone()));
                    break;
                } else if let Ok(act) = EntityAction::from_str(&widget.action) {
                    action = Some(act);
                    break;
                }

                if let Some(hide) = &widget.hide {
                    self.widgets_to_hide.clear();
                    for h in hide {
                        self.widgets_to_hide.push(h.clone());
                    }
                }
                if let Some(show) = &widget.show {
                    for s in show {
                        self.widgets_to_hide.retain(|x| x != s);
                    }
                }
                if let Some(inventory_index) = &widget.inventory_index {
                    for entity in map.entities.iter() {
                        if entity.is_player() {
                            if let Some(item) = entity.inventory.get(*inventory_index) {
                                if let Some(item) = item {
                                    action = Some(EntityAction::ItemClicked(item.id, 0.0));
                                    break;
                                }
                            }
                        }
                    }
                }

                // Deactivate the widgets and activate this widget
                if !widget.deactivate.is_empty() {
                    for widget_to_deactivate in &widget.deactivate {
                        for (id, widget) in self.button_widgets.iter() {
                            if *widget_to_deactivate == widget.name {
                                self.activated_widgets.retain(|x| x != id);
                                self.permanently_activated_widgets.retain(|x| x != id);
                            }
                        }
                    }
                    self.activated_widgets.push(widget.id);
                    self.permanently_activated_widgets.push(widget.id);
                }
            }
        }

        if action.is_none() {
            let mut player_pos: Vec2<f32> = Vec2::zero();
            for entity in map.entities.iter() {
                if entity.is_player() {
                    player_pos = entity.get_pos_xz();
                }
            }

            for (_, widget) in self.game_widgets.iter() {
                if widget.rect.contains(Vec2::new(p.x as f32, p.y as f32)) {
                    let dx = p.x as f32 - widget.rect.x;
                    let dy = p.y as f32 - widget.rect.y;

                    let gx = widget.top_left.x + dx / widget.grid_size;
                    let gy = widget.top_left.y + dy / widget.grid_size;

                    let pos = Vec2::new(gx, gy);

                    for entity in map.entities.iter() {
                        let p = entity.get_pos_xz();
                        if pos.floor() == p.floor() {
                            let distance = player_pos.distance(p);
                            return Some(EntityAction::EntityClicked(entity.id, distance));
                        }
                    }

                    for item in map.items.iter() {
                        let p = item.get_pos_xz();
                        if pos.floor() == p.floor() {
                            let distance = player_pos.distance(p);
                            return Some(EntityAction::ItemClicked(item.id, distance));
                        }
                    }

                    return Some(EntityAction::TerrainClicked(pos));
                }
            }
        }

        action
    }

    /// Click / touch up event
    pub fn touch_up(&mut self, _coord: Vec2<i32>, _map: &Map) {
        self.activated_widgets = self.permanently_activated_widgets.clone();
    }

    pub fn user_event(&mut self, event: String, value: Value) -> EntityAction {
        // Make sure we do not send action events after a key down intent was handled
        // Otherwise the character would move a bit because "intent" is already cleared
        if event == "key_up" {
            self.key_down_intent = None;
        }

        if event == "key_down" {
            if let Some(key_down_intent) = &self.key_down_intent {
                if !key_down_intent.is_empty() {
                    return EntityAction::Off;
                }
            }
        }

        if self.key_down_intent.is_none() && event == "key_down" {
            self.key_down_intent = Some(self.intent.clone());
        }

        // ---

        let action = self.client_action.lock().unwrap().user_event(event, value);

        let action_str: String = action.to_string();
        if action_str == "none" {
            self.activated_widgets = self.permanently_activated_widgets.clone();
        } else {
            for (id, widget) in self.button_widgets.iter_mut() {
                if widget.action == action_str && !self.activated_widgets.contains(id) {
                    self.activated_widgets.push(*id);
                }
            }
        }

        action
    }

    // Init the screen
    pub fn init_screen(&mut self, screen: &Map, assets: &Assets) {
        self.game_widgets.clear();
        self.button_widgets.clear();
        self.text_widgets.clear();
        self.deco_widgets.clear();
        self.messages_widget = None;

        for widget in screen.sectors.iter() {
            let bb = widget.bounding_box(screen);

            let (start_x, start_y) = crate::utils::align_screen_to_grid(
                self.viewport.x as f32,
                self.viewport.y as f32,
                self.grid_size,
            );

            let x = (bb.min.x - start_x) * self.grid_size;
            let y = (bb.min.y - start_y) * self.grid_size;
            let width = bb.size().x * self.grid_size;
            let height = bb.size().y * self.grid_size;

            if let Some(crate::Value::Str(data)) = widget.properties.get("data") {
                if let Ok(table) = data.parse::<Table>() {
                    let grid_size = self.grid_size;

                    let mut role = "none";
                    if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                        if let Some(value) = ui.get("role") {
                            if let Some(v) = value.as_str() {
                                role = v;
                            }
                        }
                    }

                    if role == "game" {
                        let mut game_widget = GameWidget {
                            rect: Rect::new(x, y, width, height),
                            toml_str: data.clone(),
                            buffer: TheRGBABuffer::new(TheDim::sized(width as i32, height as i32)),
                            grid_size,
                            ..Default::default()
                        };

                        if let Some(map) = assets.maps.get(&self.current_map) {
                            game_widget.build(map, assets);
                        }
                        game_widget.init();
                        self.game_widgets.insert(widget.creator_id, game_widget);
                    } else if role == "button" {
                        let mut action = "";
                        let mut intent = None;
                        let mut show: Option<Vec<String>> = None;
                        let mut hide: Option<Vec<String>> = None;
                        let mut deactivate: Vec<String> = vec![];
                        let mut inventory_index: Option<usize> = None;

                        if let Some(ui) = table.get("ui").and_then(toml::Value::as_table) {
                            // Check for action
                            if let Some(value) = ui.get("action") {
                                if let Some(v) = value.as_str() {
                                    action = v;
                                }
                            }

                            // Check for intent
                            if let Some(value) = ui.get("intent") {
                                if let Some(v) = value.as_str() {
                                    intent = Some(v.to_string());
                                }
                            }

                            // Check for show
                            if let Some(value) = ui.get("show") {
                                if let Some(va) = value.as_array() {
                                    let mut c = vec![];
                                    for v in va {
                                        if let Some(v) = v.as_str() {
                                            c.push(v.to_string());
                                        }
                                    }
                                    if !c.is_empty() {
                                        show = Some(c);
                                    }
                                }
                            }

                            // Check for hide
                            if let Some(value) = ui.get("hide") {
                                if let Some(va) = value.as_array() {
                                    let mut c = vec![];
                                    for v in va {
                                        if let Some(v) = v.as_str() {
                                            c.push(v.to_string());
                                        }
                                    }
                                    if !c.is_empty() {
                                        hide = Some(c);
                                    }
                                }
                            }

                            // Check for deactivate
                            if let Some(value) = ui.get("deactivate") {
                                if let Some(va) = value.as_array() {
                                    let mut c = vec![];
                                    for v in va {
                                        if let Some(v) = v.as_str() {
                                            c.push(v.to_string());
                                        }
                                    }
                                    deactivate = c;
                                }
                            }

                            // Check for active
                            if let Some(value) = ui.get("active") {
                                if let Some(v) = value.as_bool()
                                    && v
                                {
                                    self.activated_widgets.push(widget.id);
                                    self.permanently_activated_widgets.push(widget.id);
                                    if let Some(hide) = &hide {
                                        self.widgets_to_hide = hide.clone();
                                    }
                                }
                            }

                            // Check for inventory
                            if let Some(value) = ui.get("inventory_index") {
                                if let Some(v) = value.as_integer() {
                                    inventory_index = Some(v as usize);
                                }
                            }
                        }

                        let button_widget = Widget {
                            name: widget.name.clone(),
                            id: widget.id,
                            rect: Rect::new(x, y, width, height),
                            action: action.into(),
                            intent,
                            show,
                            hide,
                            deactivate,
                            inventory_index,
                        };

                        self.button_widgets.insert(widget.id, button_widget);
                    } else if role == "messages" {
                        let mut widget = MessagesWidget {
                            name: widget.name.clone(),
                            rect: Rect::new(x, y, width, height),
                            toml_str: data.clone(),
                            buffer: TheRGBABuffer::new(TheDim::sized(width as i32, height as i32)),
                            ..Default::default()
                        };
                        widget.init(assets);
                        self.messages_widget = Some(widget);
                    } else if role == "text" {
                        let mut text_widget = TextWidget {
                            name: widget.name.clone(),
                            rect: Rect::new(x, y, width, height),
                            toml_str: data.clone(),
                            buffer: TheRGBABuffer::new(TheDim::sized(width as i32, height as i32)),
                            ..Default::default()
                        };
                        text_widget.init(assets);
                        self.text_widgets.insert(widget.creator_id, text_widget);
                    } else if role == "deco" {
                        let mut deco_widget = DecoWidget {
                            rect: Rect::new(x, y, width, height),
                            toml_str: data.clone(),
                            buffer: TheRGBABuffer::new(TheDim::sized(width as i32, height as i32)),
                            ..Default::default()
                        };
                        deco_widget.init(assets);
                        self.deco_widgets.insert(widget.creator_id, deco_widget);
                    }
                }
            }
        }
    }
}
