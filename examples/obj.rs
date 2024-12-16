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
    textures: Vec<Texture>,
    camera: Box<dyn D3Camera>,
    scene: Scene,
}

impl TheTrait for ObjDemo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut scene = Scene::from_static(
            vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![Batch::from_obj(Path::new("examples/teapot.obj"))
                .sample_mode(SampleMode::Linear)
                .repeat_mode(RepeatMode::RepeatXY)],
        );
        scene.background = Some(Box::new(VGrayGradientShader::new()));

        Self {
            textures: vec![Texture::from_image(Path::new("images/logo.png"))],
            camera: Box::new(D3OrbitCamera::new()),
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
            self.camera.view_projection_matrix(
                75.0,
                ctx.width as f32,
                ctx.height as f32,
                0.1,
                100.0,
            ) * Mat4::scaling_3d(Vec3::new(0.3, 0.3, 0.3)),
            &self.textures,
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

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        false
    }

    fn window_title(&self) -> String {
        "Rusterix OBJ Demo".to_string()
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
