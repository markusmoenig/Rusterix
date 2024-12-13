use rusterix::{Batch, Rasterizer, Texture};
use theframework::*;
use vek::{Mat4, Vec3};

fn main() {
    let cube = Cube::new();
    let mut app = TheApp::new();

    () = app.run(Box::new(cube));
}

pub struct Cube {
    i: i32,
}

impl TheTrait for Cube {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self { i: 0 }
    }

    /// Draw a circle in the middle of the window
    fn draw(&mut self, pixels: &mut [u8], ctx: &mut TheContext) {
        let texture = Texture::checkerboard(100, 10);

        pixels.fill(0);
        let _start = get_time();

        let mut batches_2d = vec![Batch::from_rectangle(100.0, 100.0, 200.0, 200.0)];
        let mut batches_3d = vec![Batch::from_box(-0.5, -0.5, -0.5, 1.0, 1.0, 1.0)];
        let projection_matrix_2d = None; //Mat3::translation_2d(Vec2::new(500.0, 50.0));
                                         //let projection_matrix_3d = Mat4::identity(); //translation_2d(Vec2::new(500.0, 50.0));

        let projection_matrix_3d =
            Mat4::perspective_fov_lh_zo(1.3, ctx.width as f32, ctx.height as f32, 0.01, 100.0)
                * Mat4::translation_3d(Vec3::new(0.0, 0.0, -2.0))
                * Mat4::rotation_x((self.i as f32 * 0.0002).sin() * 8.0)
                * Mat4::rotation_y((self.i as f32 * 0.0004).cos() * 4.0)
                * Mat4::rotation_z((self.i as f32 * 0.0008).sin() * 2.0);
        // * Mat4::scaling_3d(Vec3::new(1.0, 1.0, -1.0));

        self.i += 10;

        let rasterizer = Rasterizer {};
        // Rasterize the batch
        rasterizer.rasterize(
            &mut batches_2d,
            &mut batches_3d,
            pixels,
            ctx.width,
            ctx.height,
            128,
            projection_matrix_2d,
            projection_matrix_3d,
            &texture,
        );

        let _stop = get_time();
        println!("Shader execution time: {:?} ms.", _stop - _start);
    }

    /// Touch down event
    fn touch_down(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Touch up event
    fn touch_up(&mut self, _x: f32, _y: f32, _ctx: &mut TheContext) -> bool {
        false
    }

    /// Query if the widget needs a redraw
    fn update(&mut self, _ctx: &mut TheContext) -> bool {
        true
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

/*
fn rasterize_lines(
    lines: &[((f32, f32), (f32, f32), [u8; 4])], // Array of lines and their colors
    framebuffer: &mut [u8],
    screen_width: usize,
    screen_height: usize,
    tile_size: usize,
    line_thickness: f32,
) {
    // Step 1: Divide screen into tiles
    let mut tiles = Vec::new();
    for y in (0..screen_height).step_by(tile_size) {
        for x in (0..screen_width).step_by(tile_size) {
            tiles.push(Rect {
                x,
                y,
                width: tile_size.min(screen_width - x),
                height: tile_size.min(screen_height - y),
            });
        }
    }

    // Step 2: Parallel process each tile
    let tile_buffers: Vec<Vec<u8>> = tiles
        .par_iter()
        .map(|tile| {
            let mut buffer = vec![0; tile.width * tile.height * 4]; // RGBA format

            // Process every line for this tile
            for ((x0, y0), (x1, y1), color) in lines {
                // Compute bounding box of the line segment
                let min_x = (*x0 as usize)
                    .min(*x1 as usize)
                    .clamp(tile.x, tile.x + tile.width);
                let max_x = (*x0 as usize)
                    .max(*x1 as usize)
                    .clamp(tile.x, tile.x + tile.width);
                let min_y = (*y0 as usize)
                    .min(*y1 as usize)
                    .clamp(tile.y, tile.y + tile.height);
                let max_y = (*y0 as usize)
                    .max(*y1 as usize)
                    .clamp(tile.y, tile.y + tile.height);

                // Rasterize the line within its bounding box
                for ty in min_y..max_y {
                    for tx in min_x..max_x {
                        let px = tx as f32 + 0.5;
                        let py = ty as f32 + 0.5;

                        // Calculate distance from the pixel to the line segment
                        let distance = point_to_line_distance((*x0, *y0), (*x1, *y1), (px, py));

                        if distance <= line_thickness {
                            let idx = ((ty - tile.y) * tile.width + (tx - tile.x)) * 4;
                            buffer[idx..idx + 4].copy_from_slice(color);
                        }
                    }
                }
            }

            buffer
        })
        .collect();

    // Step 3: Combine tile buffers into the main framebuffer
    for (i, tile) in tiles.iter().enumerate() {
        let tile_buffer = &tile_buffers[i];
        for ty in 0..tile.height {
            for tx in 0..tile.width {
                let px = tile.x + tx;
                let py = tile.y + ty;

                let src_idx = (ty * tile.width + tx) * 4;
                let dst_idx = (py * screen_width + px) * 4;

                framebuffer[dst_idx..dst_idx + 4]
                    .copy_from_slice(&tile_buffer[src_idx..src_idx + 4]);
            }
        }
    }
}

fn point_to_line_distance(v0: (f32, f32), v1: (f32, f32), p: (f32, f32)) -> f32 {
    let (x0, y0) = v0;
    let (x1, y1) = v1;
    let (px, py) = p;

    let line_length_squared = (x1 - x0).powi(2) + (y1 - y0).powi(2);
    if line_length_squared == 0.0 {
        // The line segment is a point
        return ((px - x0).powi(2) + (py - y0).powi(2)).sqrt();
    }

    let t = ((px - x0) * (x1 - x0) + (py - y0) * (y1 - y0)) / line_length_squared;
    let t_clamped = t.clamp(0.0, 1.0);

    let closest_x = x0 + t_clamped * (x1 - x0);
    let closest_y = y0 + t_clamped * (y1 - y0);

    ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt()
}
*/
