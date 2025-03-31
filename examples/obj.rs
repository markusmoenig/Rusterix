use rusterix::prelude::*;
use std::path::Path;
use theframework::*;
use vek::{Mat4, Vec2, Vec3};

fn main() {
    let demo = ObjDemo::new();
    let mut app = TheApp::new();
    app.run(Box::new(demo));
}

/// A 3D rendering demo using Rusterix with a rectangle and teapot model.
pub struct ObjDemo {
    camera: D3OrbitCamera,
    scene: Scene,
}

impl TheTrait for ObjDemo {
    fn new() -> Self {
        let scene = create_scene().expect("Failed to create scene");
        Self {
            camera: D3OrbitCamera::new(),
            scene,
        }
    }

    /// Renders the scene into the pixel buffer.
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let start = get_time();

        // Use an identity matrix for 2D rendering; adjust as needed
        let projection_matrix_2d = Some(Mat4::identity());
        Rasterizer::setup(
            projection_matrix_2d,
            self.camera.view_matrix(),
            self.camera.projection_matrix(ctx.width as f32, ctx.height as f32),
        )
        .rasterize(
            &mut self.scene,
            pixels,
            ctx.width,
            ctx.height,
            200, // Tile size
        );

        let stop = get_time();
        println!("Render time: {} ms", stop - start);
    }

    /// Updates camera based on mouse hover position.
    fn hover(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        self.camera.set_parameter_vec2(
            "from_normalized",
            Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        );
        true // Request redraw on hover
    }

    /// Determines if a redraw is needed (currently always true for max speed).
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true // Consider optimizing to redraw only when necessary
    }

    fn window_title(&self) -> String {
        "Rusterix OBJ Demo".to_string()
    }
}

/// Creates a scene with a 2D rectangle and a 3D teapot model.
fn create_scene() -> Result<Scene, Box<dyn std::error::Error>> {
    let rectangle = Batch::from_rectangle(0.0, 0.0, 200.0, 200.0);
    
    // Load teapot with error handling
    let teapot = Batch::from_obj(Path::new("examples/teapot.obj"))
        .sample_mode(SampleMode::Linear)
        .repeat_mode(RepeatMode::RepeatXY)
        .transform(Mat4::scaling_3d(Vec3::new(0.35, -0.35, 0.35)));

    let batches = vec![rectangle, teapot];
    
    // Load texture with error handling
    let textures = vec![Tile::from_texture(Texture::from_image(Path::new(
        "images/logo.png",
    )))];
    
    Ok(Scene::from_static(batches, textures)
        .background(Box::new(VGrayGradientShader::new())))
}

/// Returns the current time in milliseconds, platform-dependent.
fn get_time() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window().unwrap().performance().unwrap().now() as u128
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
    }
}
