use rusterix::prelude::*;
use theframework::prelude::*;
use vek::Vec2;

use crate::{Cmd::*, FROM_WINDOW_TX, TO_WINDOW_RX};

#[derive(Debug, Clone)]
#[allow(dead_code, clippy::large_enum_variant)]
enum Content {
    Off,
    MapPreview(MapMeta),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PreviewMode {
    D2,
    D3,
}

use Content::*;
use PreviewMode::*;

pub struct Editor {
    camera: Box<dyn D3Camera>,
    content: Content,
    preview_mode: PreviewMode,
    camera_pos: vek::Vec3<f32>,
    camera_look_at: vek::Vec3<f32>,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            camera: Box::new(D3IsoCamera::new()),
            content: Off,
            preview_mode: D2,
            camera_pos: vek::Vec3::new(0.0, 1.0, -3.0),
            camera_look_at: vek::Vec3::new(0.0, 0.0, 0.0),
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        // let _start = get_time();

        if let Some(rx) = TO_WINDOW_RX.get() {
            if let Ok(r) = rx.lock() {
                while let Ok(command) = r.try_recv() {
                    match command {
                        FocusMap(map) => self.content = MapPreview(map),
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
                    builder.set_map_tool_type(MapToolType::Selection);

                    let mut scene = builder.build(
                        &meta.map,
                        &meta.tiles,
                        Texture::from_color(BLACK),
                        vek::Vec2::new(ctx.width as f32, ctx.height as f32),
                    );

                    Rasterizer {}.rasterize(
                        &mut scene,
                        pixels,
                        ctx.width,
                        ctx.height,
                        100,
                        None,
                        vek::Mat4::identity(),
                        vek::Mat4::identity(),
                    );
                }
                D3 => {
                    let builder = D3Builder::new();

                    let mut scene = builder.build(
                        &meta.map,
                        &meta.tiles,
                        Texture::from_color(BLACK),
                        vek::Vec2::new(ctx.width as f32, ctx.height as f32),
                    );

                    // let look_at = vek::Vec3::new(2.0, 0.0, 2.0);
                    // let position = vek::Vec3::new(2.0, 0.2, look_at.z + 1.0);

                    let mut position = self.camera_pos;

                    if self.camera.id() == "iso" {
                        position = vek::Vec3::new(
                            self.camera_look_at.x - 10.0,
                            self.camera_look_at.y + 10.0,
                            self.camera_look_at.z + 10.0,
                        );
                    }

                    self.camera.set_parameter_vec3("position", position);
                    self.camera
                        .set_parameter_vec3("look_at", self.camera_look_at);

                    self.camera.set_parameter_f32("distance", -8.0);

                    let view_matrix = self.camera.view_matrix();

                    let projection_matrix = self.camera.projection_matrix(
                        75.0,
                        ctx.width as f32,
                        ctx.height as f32,
                        0.1,
                        100.0,
                    );

                    Rasterizer {}.rasterize(
                        &mut scene,
                        pixels,
                        ctx.width,
                        ctx.height,
                        80,
                        None,
                        view_matrix,
                        projection_matrix,
                    );
                }
            },
            _ => {}
        }

        /*
        let projection_matrix_2d = None;

        // Rasterize the batches
        Rasterizer {}.rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            200,        // Tile size
            projection_matrix_2d,
            self.camera.view_projection_matrix(
                75.0,
                ctx.width as f32,
                ctx.height as f32,
                0.1,
                100.0,
            ),
        );*/

        //let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Hover event
    fn touch_down(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        if let Some(tx) = FROM_WINDOW_TX.get() {
            tx.send(MouseDown(Vec2::new(x, y))).unwrap();
        }

        self.camera.set_parameter_vec2(
            "from_normalized",
            Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        );
        true
    }

    fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        if self.camera.id() == "orbit" {
            self.camera.set_parameter_vec2(
                "from_normalized",
                Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
            );
        } else if self.camera.id() == "firstp" {
            // First-person camera logic
            let mouse_sensitivity = 0.2; // Adjust sensitivity as needed
            let delta_x = x / ctx.width as f32 - 0.5; // Normalize to range [-0.5, 0.5]
            let delta_y = -y / ctx.height as f32; // - 0.5;

            // Update yaw and pitch based on mouse movement
            let yaw = delta_x * mouse_sensitivity * std::f32::consts::PI;
            let pitch = delta_y * mouse_sensitivity * std::f32::consts::PI;

            // Calculate the forward vector using the updated yaw and pitch
            let direction = vek::Vec3::new(
                yaw.cos() * pitch.cos(),
                pitch.sin(),
                yaw.sin() * pitch.cos(),
            )
            .normalized();

            // Update the camera's look_at based on the new direction
            self.camera_look_at = self.camera_pos + direction;
        }
        true
    }

    fn mouse_wheel(&mut self, delta: (isize, isize), _ctx: &mut TheContext) -> bool {
        #[allow(clippy::single_match)]
        match &mut self.content {
            MapPreview(meta) => {
                meta.map.offset += vek::Vec2::new(delta.0 as f32 * 0.2, -delta.1 as f32 * 0.2);
            }
            _ => {}
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
                    // Move forward along the camera's forward direction
                    let forward = (self.camera_look_at - self.camera_pos).normalized();
                    self.camera_pos += forward * 0.1;
                    self.camera_look_at += forward * 0.1;
                }
                's' => {
                    // Move backward along the camera's forward direction
                    let forward = (self.camera_look_at - self.camera_pos).normalized();
                    self.camera_pos -= forward * 0.1;
                    self.camera_look_at -= forward * 0.1;
                }
                'a' => {
                    // Rotate camera left (yaw)
                    let direction = self.camera_look_at - self.camera_pos;
                    let rotation = vek::Mat4::rotation_y(-0.1); // Rotate by -0.1 radians
                    let rotated_direction =
                        rotation * vek::Vec4::new(direction.x, direction.y, direction.z, 1.0);
                    self.camera_look_at = self.camera_pos
                        + vek::Vec3::new(
                            rotated_direction.x,
                            rotated_direction.y,
                            rotated_direction.z,
                        );
                }
                'd' => {
                    // Rotate camera right (yaw)
                    let direction = self.camera_look_at - self.camera_pos;
                    let rotation = vek::Mat4::rotation_y(0.1); // Rotate by 0.1 radians
                    let rotated_direction =
                        rotation * vek::Vec4::new(direction.x, direction.y, direction.z, 1.0);
                    self.camera_look_at = self.camera_pos
                        + vek::Vec3::new(
                            rotated_direction.x,
                            rotated_direction.y,
                            rotated_direction.z,
                        );
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
