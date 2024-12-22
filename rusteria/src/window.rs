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
enum PreviewMode {
    D2,
    D3,
}

use Content::*;
use PreviewMode::*;

pub struct Editor {
    camera: D3IsoCamera,
    content: Content,
    preview_mode: PreviewMode,
}

impl TheTrait for Editor {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            camera: D3IsoCamera::new(),
            content: Off,
            preview_mode: D3,
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

                    let look_at = vek::Vec3::new(2.0, 0.0, -2.0);

                    let position =
                        vek::Vec3::new(look_at.x - 10.0, look_at.y + 10.0, look_at.z + 10.0);

                    self.camera.set_parameter_vec3("position", position);
                    self.camera.set_parameter_vec3("look_at", look_at);

                    let matrix = self.camera.view_projection_matrix(
                        75.0,
                        ctx.width as f32,
                        ctx.height as f32,
                        0.1,
                        100.0,
                    );

                    Rasterizer {}
                        .rasterize(&mut scene, pixels, ctx.width, ctx.height, 100, None, matrix);
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
