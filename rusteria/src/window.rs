use rusterix::prelude::*;
use std::path::Path;
use theframework::*;
use vek::Vec2;

use crate::{Cmd::*, TX};

pub struct Cube {
    camera: D3OrbitCamera,
    scene: Scene,
}

impl TheTrait for Cube {
    fn new() -> Self
    where
        Self: Sized,
    {
        let scene = Scene::from_static(
            vec![Batch::from_rectangle(0.0, 0.0, 200.0, 200.0)],
            vec![Batch::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0)
                .sample_mode(SampleMode::Nearest)
                .cull_mode(CullMode::Back)],
        )
        .background(Box::new(VGrayGradientShader::new()))
        .textures(vec![Texture::from_image(Path::new("images/logo.png"))]);

        Self {
            camera: D3OrbitCamera::new(),
            scene,
        }
    }

    /// Draw a cube and a rectangle
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        // let _start = get_time();

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
        );

        //let _stop = get_time();
        // println!("Execution time: {:?} ms.", _stop - _start);
    }

    // Hover event
    fn touch_down(&mut self, x: f32, y: f32, ctx: &mut TheContext) -> bool {
        if let Some(tx) = TX.get() {
            tx.send(MouseDown(Vec2::new(x, y))).unwrap();
        }

        self.camera.set_parameter_vec2(
            "from_normalized",
            Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
        );
        true
    }

    // Hover event
    // fn hover(&mut self, x: f32, y: f32, _ctx: &mut TheContext) -> bool {
    // if let Some(tx) = TX.get() {
    //     tx.send(MouseDown(Vec2::zero())).unwrap();
    // }

    // self.camera.set_parameter_vec2(
    //     "from_normalized",
    //     Vec2::new(x / ctx.width as f32, y / ctx.height as f32),
    // );
    // true
    // false
    // }

    // Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        false
    }

    fn window_title(&self) -> String {
        "Rusteria Editor".to_string()
    }

    fn closing(&self) -> bool {
        if let Some(tx) = TX.get() {
            tx.send(ClosingWindow).unwrap();
        }
        false
    }
}
