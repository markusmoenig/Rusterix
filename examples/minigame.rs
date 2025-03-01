use rusterix::{prelude::*, rusterix::Rusterix};
use std::path::Path;
use theframework::prelude::*;

fn main() {
    let game = MiniGame::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(game));
}

// This example executes the minigame in the Rusterix game API.

pub struct MiniGame {
    rusterix: Rusterix,
}

impl TheTrait for MiniGame {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut assets = Assets::default();
        assets.collect_from_directory("minigame".to_string());
        assets.compile_source_maps();

        let mut rusterix = Rusterix::default();
        rusterix.set_assets(assets);
        rusterix.create_regions();

        let camera = Box::new(D3FirstPCamera::new());
        rusterix.client.set_camera_d3(camera);

        if let Some(map) = rusterix.assets.get_map("world") {
            // Build the 3D scene from the map meta data
            rusterix
                .client
                .build_scene_d3(map, &rusterix.assets, &ValueContainer::default());
        }

        // Add logo on top of the scene
        rusterix.client.scene_d3.d2_static = vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)
            .receives_light(false)
            .texture_index(rusterix.client.scene_d3.textures.len())];
        rusterix
            .client
            .scene_d3
            .textures
            .push(Tile::from_texture(Texture::from_image(Path::new(
                "images/logo.png",
            ))));

        Self { rusterix }
    }

    /// Draw the game.
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        // Update the entities on the server.
        self.rusterix.server.update();

        let (entities, items) = self
            .rusterix
            .server
            .get_entities_items(&self.rusterix.client.curr_map_id);

        self.rusterix.client.apply_entities_items_d3(
            entities.unwrap_or(&vec![]),
            items.unwrap_or(&vec![]),
            &self.rusterix.assets,
            &ValueContainer::default(),
        );

        self.rusterix.draw_scene(pixels, ctx.width, ctx.height);

        let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Set the target fps to 60
    fn target_fps(&self) -> f64 {
        60.0
    }

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }

    fn window_title(&self) -> String {
        "Rusterix Map Demo".to_string()
    }

    fn hover(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        // self.entity
        //     .set_tilt_from_screen_coordinate(1.0 - y / ctx.height as f32);
        true
    }

    fn key_down(
        &mut self,
        char: Option<char>,
        _key: Option<TheKeyCode>,
        _ctx: &mut TheContext,
    ) -> bool {
        if let Some(char) = char {
            self.rusterix
                .server
                .local_player_event("key_down".into(), Value::Str(char.to_string()));
        }
        true
    }

    fn key_up(
        &mut self,
        char: Option<char>,
        _key: Option<TheKeyCode>,
        _ctx: &mut TheContext,
    ) -> bool {
        if let Some(char) = char {
            self.rusterix
                .server
                .local_player_event("key_up".into(), Value::Str(char.to_string()));
        }
        true
    }
}

fn get_time() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now() as u128
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let stop = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards");
        stop.as_millis()
    }
}
