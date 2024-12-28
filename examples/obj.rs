use rusterix::prelude::*;
use std::path::Path;
use theframework::*;
use vek::{Mat4, Vec2, Vec3};

fn main() {
    let demo = ObjDemo::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(demo));
}

// This example uses raw draw calls into rusterix, bypassing the engine API.

pub struct ObjDemo {
    camera: D3OrbitCamera,
    scene: Scene,
}

impl TheTrait for ObjDemo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let scene = Scene::from_static(
            vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![Batch::from_obj(Path::new("examples/teapot.obj"))
                .sample_mode(SampleMode::Linear)
                .repeat_mode(RepeatMode::RepeatXY)],
        )
        .background(Box::new(VGrayGradientShader::new()))
        .textures(vec![Texture::from_image(Path::new("images/logo.png"))]);

        Self {
            camera: D3OrbitCamera::new(),
            scene,
        }
    }

    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let _start = get_time();

        let projection_matrix_2d = None;

        // Rasterize the batches
        Rasterizer {}.rasterize(
            &mut self.scene,
            pixels,     // Destination buffer
            ctx.width,  // Destination buffer width
            ctx.height, // Destination buffer height
            80,         // Tile size
            projection_matrix_2d,
            self.camera.view_matrix() * Mat4::scaling_3d(Vec3::new(0.35, -0.35, 0.35)),
            self.camera
                .projection_matrix(ctx.width as f32, ctx.height as f32),
        );

        let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Hover event
    fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        self.camera.set_parameter_vec2(
            "from_normalized",
            Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        );
        true
    }

    // Query if the widget needs a redraw, we redraw at max speed (which is not necessary)
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
    }

    fn window_title(&self) -> String {
        "Rusterix OBJ Demo".to_string()
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
