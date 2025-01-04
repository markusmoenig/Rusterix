use rusterix::{prelude::*, rusterix::Rusterix};
use std::path::Path;
use std::time::{Duration, Instant};
use theframework::*;
use vek::{Vec2, Vec3};

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum Movement {
    Off,
    MoveForward,
    MoveBackward,
    TurnLeft,
    TurnRight,
}

use Movement::*;

fn main() {
    let game = MiniGame::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(game));
}

// This example executes the minigame in the Rusterix game API.

pub struct MiniGame {
    camera: Box<dyn D3Camera>,
    scene: Scene,
    entity: Entity,
    movement: Movement,
    rusterix: Rusterix,
    builder: D3Builder,
    last_entities: Vec<Entity>,

    last_redraw_update: Instant,
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
        let mut scene = Scene::default();

        let builder = D3Builder::new();
        if let Some(map) = rusterix.assets.get_map("world") {
            // Build the 3D scene from the map meta data
            scene = builder.build(
                map,
                &rusterix.assets.tiles,
                Texture::from_color(BLACK),
                Vec2::zero(), // Only needed for 2D builders
                &camera.id(),
            );
        }

        // Create an entity with a default position / orientation.
        let entity = rusterix::Entity {
            position: Vec3::new(6.0600824, 1.0, 4.5524735),
            orientation: Vec2::new(0.03489969, 0.99939084),
            ..Default::default()
        };

        // Add logo on top of the scene
        scene.d2 =
            vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0).texture_index(scene.textures.len())];
        scene
            .textures
            .push(Texture::from_image(Path::new("images/logo.png")));

        Self {
            camera,
            scene,
            entity,
            movement: Off,
            rusterix,
            builder,
            last_entities: vec![],

            last_redraw_update: Instant::now(),
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        // Get the entities and build their scene representation
        self.rusterix
            .server
            .update_entities(&mut self.last_entities);

        for entity in &self.last_entities {
            if entity.is_player() {
                entity.apply_to_camera(&mut self.camera);
            }
        }

        self.builder.build_entities_d3(
            &self.last_entities,
            self.camera.as_ref(),
            &self.rusterix.assets.tiles,
            &mut self.scene,
        );

        // Set it up
        Rasterizer::setup(
            None,
            self.camera.view_matrix(),
            self.camera
                .projection_matrix(ctx.width as f32, ctx.height as f32),
        )
        .rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            200,        // Tile size
        );

        let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Query if the widget needs a redraw, limit redraws to 30fps
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        let target_fps = 60;
        let mut redraw_update = false;

        if self.last_redraw_update.elapsed() >= Duration::from_millis(1000 / target_fps) {
            self.last_redraw_update = Instant::now();
            redraw_update = true;
        }

        redraw_update
    }

    fn window_title(&self) -> String {
        "Rusterix Map Demo".to_string()
    }

    fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        if self.camera.id() == "orbit" {
            self.camera.set_parameter_vec2(
                "from_normalized",
                Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
            );
        } else if self.camera.id() == "firstp" {
            self.entity
                .set_tilt_from_screen_coordinate(1.0 - y / ctx.height as f32);
        }
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
            match char {
                'p' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                }
                'f' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                }
                'i' => {
                    self.camera = Box::new(D3IsoCamera::new());
                }
                'o' => {
                    self.camera = Box::new(D3OrbitCamera::new());
                }
                'w' => {
                    self.movement = MoveForward;
                }
                's' => {
                    self.movement = MoveBackward;
                }
                'a' => {
                    self.movement = TurnLeft;
                }
                'd' => {
                    self.movement = TurnRight;
                }
                _ => {}
            }
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
            match char {
                // 'p' => {
                //     self.camera = Box::new(D3FirstPCamera::new());
                // }
                // 'f' => {
                //     self.camera = Box::new(D3FirstPCamera::new());
                // }
                // 'i' => {
                //     self.camera = Box::new(D3IsoCamera::new());
                // }
                // 'o' => {
                //     self.camera = Box::new(D3OrbitCamera::new());
                // }
                'w' => {
                    if self.movement == MoveForward {
                        self.movement = Off;
                    }
                }
                's' => {
                    if self.movement == MoveBackward {
                        self.movement = Off;
                    }
                }
                'a' => {
                    if self.movement == TurnLeft {
                        self.movement = Off;
                    }
                }
                'd' => {
                    if self.movement == TurnRight {
                        self.movement = Off;
                    }
                }
                _ => {}
            }
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
