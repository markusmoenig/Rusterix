use rusterix::prelude::*;
use std::path::Path;
use theframework::*;
use vek::{Mat4, Vec3, Vec4};

fn main() {
    let demo = ObjDemo::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(demo));
}

// This example uses raw draw calls into rusterix, bypassing the engine API.

pub struct ObjDemo {
    textures: Vec<Texture>,
    batches_2d: Vec<Batch<Vec3<f32>>>,
    batches_3d: Vec<Batch<Vec4<f32>>>,
    i: i32,
}

impl TheTrait for ObjDemo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let batches_2d = vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)];
        let batches_3d = vec![
            Batch::from_obj(Path::new("examples/teapot.obj")).sample_mode(SampleMode::Nearest)
        ];

        Self {
            textures: vec![Texture::from_image(Path::new("images/logo.png"))],
            batches_2d,
            batches_3d,
            i: 0,
        }
    }

    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        let projection_matrix_2d = None;

        // Rotation code taken from euc
        let projection_matrix_3d =
            Mat4::perspective_fov_lh_zo(1.3, ctx.width as f32, ctx.height as f32, 0.01, 100.0)
                * Mat4::translation_3d(Vec3::new(0.0, 0.0, -2.0))
                * Mat4::rotation_x((self.i as f32 * 0.0002).sin() * 8.0)
                * Mat4::rotation_y((self.i as f32 * 0.0004).cos() * 4.0)
                * Mat4::rotation_z((self.i as f32 * 0.0008).sin() * 2.0)
                * Mat4::scaling_3d(Vec3::new(0.3, 0.3, 0.3));

        self.i += 10;

        // Rasterize the batch
        Rasterizer {}.rasterize(
            &mut self.batches_2d,
            &mut self.batches_3d,
            pixels,
            ctx.width,
            ctx.height,
            80,
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
