use rusterix::prelude::*;
use std::path::Path;
use theframework::*;
use vek::{Mat4, Vec3};

fn main() {
    let cube = Cube::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(cube));
}

// This example uses raw draw calls into rusterix, bypassing the engine API.

pub struct Cube {
    textures: Vec<Texture>,
    scene: Scene,
    i: i32,
}

impl TheTrait for Cube {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut scene = Scene::from_static(
            vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![Batch::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0).sample_mode(SampleMode::Nearest)],
        );
        scene.background = Some(Box::new(VGrayGradientShader::new()));

        Self {
            textures: vec![Texture::from_image(Path::new("images/logo.png"))],
            scene,
            i: 0,
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        let projection_matrix_2d = None;

        // Rotation code taken from euc
        let projection_matrix_3d =
            Mat4::perspective_fov_lh_zo(1.3, ctx.width as f32, ctx.height as f32, 0.01, 100.0)
                * Mat4::translation_3d(Vec3::new(0.0, 0.0, -2.0))
                * Mat4::rotation_x((self.i as f32 * 0.0002).sin() * 8.0)
                * Mat4::rotation_y((self.i as f32 * 0.0004).cos() * 4.0)
                * Mat4::rotation_z((self.i as f32 * 0.0008).sin() * 2.0);

        self.i += 10;

        // Rasterize the batches
        Rasterizer {}.rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            80,         // Tile size
            projection_matrix_2d,
            projection_matrix_3d,
            &self.textures,
        );

        let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Touch down event
    fn touch_down(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    // Touch up event
    fn touch_up(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }

    fn window_title(&self) -> String {
        "Rusterix Cube Demo".to_string()
    }
}

pub fn get_time() -> u128 {
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
