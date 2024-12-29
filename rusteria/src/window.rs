use rusterix::prelude::*;
use theframework::prelude::*;
use vek::Vec2;

use crate::{Cmd::*, FROM_WINDOW_TX, TO_WINDOW_RX};

#[derive(Debug, Clone)]
#[allow(dead_code, clippy::large_enum_variant)]
enum Content {
    NoContent,
    MapPreview(MapMeta),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PreviewMode {
    D2,
    D3,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum Movement {
    Off,
    MoveForward,
    MoveBackward,
    TurnLeft,
    TurnRight,
}

use Content::*;
use Movement::*;
use PreviewMode::*;

pub struct Editor {
    camera: Box<dyn D3Camera>,
    entity: Entity,
    content: Content,
    preview_mode: PreviewMode,
    scene: Scene,
    movement: Movement,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            camera: Box::new(D3IsoCamera::new()),
            entity: Entity::default(),
            content: NoContent,
            preview_mode: D2,
            scene: Scene::default(),
            movement: Off,
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        // let _start = get_time();

        if let Some(rx) = TO_WINDOW_RX.get() {
            if let Ok(r) = rx.lock() {
                while let Ok(command) = r.try_recv() {
                    match command {
                        FocusMap(meta) => {
                            self.content = MapPreview(meta.clone());
                            let builder = D3Builder::new();

                            self.scene = builder.build(
                                &meta.map,
                                &meta.tiles,
                                Texture::from_color(BLACK),
                                vek::Vec2::new(ctx.width as f32, ctx.height as f32),
                                &self.camera.id(),
                            );
                        }
                        Exit => {
                            // TODO
                        }
                        _ => {}
                    }
                }
            }
        }

        #[allow(clippy::single_match)]
        match &self.content {
            MapPreview(meta) => match &self.preview_mode {
                D2 => {
                    let mut builder = D2PreviewBuilder::new();
                    builder
                        .set_camera_info(Some(self.entity.position), self.entity.camera_look_at());
                    builder.set_map_tool_type(MapToolType::Selection);

                    let mut scene = builder.build(
                        &meta.map,
                        &meta.tiles,
                        Texture::from_color(BLACK),
                        vek::Vec2::new(ctx.width as f32, ctx.height as f32),
                        &self.camera.id(),
                    );

                    Rasterizer::setup(None, vek::Mat4::identity(), vek::Mat4::identity())
                        .rasterize(&mut scene, pixels, ctx.width, ctx.height, 100);
                }
                D3 => {
                    match &self.movement {
                        MoveForward => {
                            self.entity.move_forward(0.05);
                        }
                        MoveBackward => {
                            self.entity.move_backward(0.05);
                        }
                        TurnLeft => {
                            self.entity.turn_left(1.0);
                        }
                        TurnRight => {
                            self.entity.turn_right(1.0);
                        }
                        Off => {}
                    }

                    self.entity.apply_to_camera(&mut self.camera);

                    if self.camera.id() == "iso" {
                        let position = vek::Vec3::new(
                            self.entity.position.x - 10.0,
                            self.entity.position.y + 10.0,
                            self.entity.position.z + 10.0,
                        );
                        self.camera.set_parameter_vec3("position", position);
                    }

                    self.camera.set_parameter_f32("distance", -8.0);

                    let view_matrix = self.camera.view_matrix();

                    let projection_matrix = self
                        .camera
                        .projection_matrix(ctx.width as f32, ctx.height as f32);

                    let _start = get_time();

                    // Set it up
                    Rasterizer::setup(None, view_matrix, projection_matrix).rasterize(
                        &mut self.scene,
                        pixels,
                        ctx.width,
                        ctx.height,
                        200,
                    );

                    let _stop = get_time();
                    // println!("Execution time: {:?} ms.", _stop - _start);
                }
            },
            _ => {}
        }
    }

    // Hover event
    fn touch_down(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        // if let Some(tx) = FROM_WINDOW_TX.get() {
        //     tx.send(MouseDown(Vec2::new(x, y))).unwrap();
        // }

        // self.camera.set_parameter_vec2(
        //     "from_normalized",
        //     Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        // );

        #[allow(clippy::single_match)]
        match &self.content {
            MapPreview(meta) => {
                let pos = local_to_map_grid(
                    Vec2::new(ctx.width as f32, ctx.height as f32),
                    Vec2::new(x, y),
                    &meta.map,
                    1.0,
                );
                self.entity.position.x = pos.x;
                self.entity.position.y = 1.0;
                self.entity.position.z = pos.y;
            }
            _ => {}
        }

        true
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

    fn mouse_wheel(&mut self, delta: (isize, isize), _ctx: &mut TheContext) -> bool {
        #[allow(clippy::single_match)]
        match &mut self.content {
            MapPreview(meta) => {
                meta.map.offset += Vec2::new(delta.0 as f32 * 0.2, -delta.1 as f32 * 0.2);
            }
            _ => {}
        }
        true
    }

    // fn key_up(
    //     &mut self,
    //     char: Option<char>,
    //     _key: Option<TheKeyCode>,
    //     _ctx: &mut TheContext,
    // ) -> bool {
    // }

    fn key_down(
        &mut self,
        char: Option<char>,
        _key: Option<TheKeyCode>,
        _ctx: &mut TheContext,
    ) -> bool {
        if let Some(char) = char {
            match char {
                'p' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                    self.preview_mode = D2;
                }
                'f' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                    self.preview_mode = D3;
                }
                'i' => {
                    self.camera = Box::new(D3IsoCamera::new());
                    self.preview_mode = D3;
                }
                'o' => {
                    self.camera = Box::new(D3OrbitCamera::new());
                    self.preview_mode = D3;
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
        //println!("pos {} look at {}", self.camera_pos, self.camera_look_at);
        true
    }

    fn key_up(
        &mut self,
        char: Option<char>,
        _key: Option<TheKeyCode>,
        _ctx: &mut TheContext,
    ) -> bool {
        if let Some(char) = char {
            match char {
                'p' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                    self.preview_mode = D2;
                }
                'f' => {
                    self.camera = Box::new(D3FirstPCamera::new());
                    self.preview_mode = D3;
                }
                'i' => {
                    self.camera = Box::new(D3IsoCamera::new());
                    self.preview_mode = D3;
                }
                'o' => {
                    self.camera = Box::new(D3OrbitCamera::new());
                    self.preview_mode = D3;
                }
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
        //println!("pos {} look at {}", self.camera_pos, self.camera_look_at);
        true
    }

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }

    fn window_title(&self) -> String {
        "Rusteria Editor".to_string()
    }

    fn default_window_size(&self) -> (usize, usize) {
        (640, 403)
    }

    fn closing(&self) -> bool {
        if let Some(tx) = FROM_WINDOW_TX.get() {
            tx.send(ClosingWindow).unwrap();
        }
        false
    }
}

/// Convert local screen position to a map grid position
pub fn local_to_map_grid(
    screen_size: Vec2<f32>,
    coord: Vec2<f32>,
    map: &Map,
    subdivisions: f32,
) -> Vec2<f32> {
    let grid_space_pos = coord - screen_size / 2.0 - Vec2::new(map.offset.x, -map.offset.y);
    let snapped = grid_space_pos / map.grid_size;
    let rounded = snapped.map(|x| x.round());

    if subdivisions > 1.0 {
        let subdivision_size = 1.0 / subdivisions;

        // Calculate fractional part of the snapped position
        let fractional = snapped - rounded;

        // Snap the fractional part to the nearest subdivision
        rounded + fractional.map(|x| (x / subdivision_size).round() * subdivision_size)
    } else {
        rounded
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
