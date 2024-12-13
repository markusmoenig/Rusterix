use crate::{Batch, Pixel, PrimitiveMode, Texture};
use rayon::prelude::*;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};
pub struct Rasterizer;

impl Rasterizer {
    #[allow(clippy::too_many_arguments)]
    pub fn rasterize(
        &self,
        batches_2d: &mut [Batch<Vec3<f32>>],
        batches_3d: &mut [Batch<Vec4<f32>>],
        pixels: &mut [u8],
        width: usize,
        height: usize,
        tile_size: usize,
        projection_matrix_2d: Option<Mat3<f32>>,
        projection_matrix_3d: Mat4<f32>,
        atlas: &Texture,
    ) {
        batches_2d.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        batches_3d.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_3d, width as f32, height as f32);
        });

        // Divide the screen into tiles
        let mut tiles = Vec::new();
        for y in (0..height).step_by(tile_size) {
            for x in (0..width).step_by(tile_size) {
                tiles.push(TileRect {
                    x,
                    y,
                    width: tile_size.min(width - x),
                    height: tile_size.min(height - y),
                });
            }
        }

        // Parallel process each tile
        let tile_buffers: Vec<Vec<u8>> = tiles
            .par_iter()
            .map(|tile| {
                // Local tile buffer
                let mut buffer = vec![0; tile.width * tile.height * 4];
                let mut z_buffer = vec![1.0_f32; tile.width * tile.height];

                for batch in batches_3d.iter() {
                    // Bounding box check for the tile with the batch bbox
                    if let Some(bbox) = batch.bounding_box {
                        if bbox.x < (tile.x + tile.width) as f32
                            && (bbox.x + bbox.width) > tile.x as f32
                            && bbox.y < (tile.y + tile.height) as f32
                            && (bbox.y + bbox.height) > tile.y as f32
                        {
                            // Process each triangle in the batch
                            for (triangle_index, edges) in batch.edges.iter().enumerate() {
                                let (i0, i1, i2) = batch.indices[triangle_index];
                                let v0 = batch.projected_vertices[i0];
                                let v1 = batch.projected_vertices[i1];
                                let v2 = batch.projected_vertices[i2];
                                let uv0 = batch.uvs[i0];
                                let uv1 = batch.uvs[i1];
                                let uv2 = batch.uvs[i2];

                                // Check if all three vertices are behind the near clipping plane
                                if v0.z < 0.0 && v1.z < 0.0 && v2.z < 0.0 {
                                    continue;
                                }

                                // Compute bounding box of the triangle
                                let min_x = [v0.x, v1.x, v2.x]
                                    .iter()
                                    .cloned()
                                    .fold(f32::INFINITY, f32::min)
                                    .floor()
                                    .max(tile.x as f32)
                                    as usize;
                                let max_x = [v0.x, v1.x, v2.x]
                                    .iter()
                                    .cloned()
                                    .fold(f32::NEG_INFINITY, f32::max)
                                    .ceil()
                                    .min((tile.x + tile.width) as f32)
                                    as usize;
                                let min_y = [v0.y, v1.y, v2.y]
                                    .iter()
                                    .cloned()
                                    .fold(f32::INFINITY, f32::min)
                                    .floor()
                                    .max(tile.y as f32)
                                    as usize;
                                let max_y = [v0.y, v1.y, v2.y]
                                    .iter()
                                    .cloned()
                                    .fold(f32::NEG_INFINITY, f32::max)
                                    .ceil()
                                    .min((tile.y + tile.height) as f32)
                                    as usize;

                                // Rasterize the triangle within its bounding box
                                for ty in min_y..max_y {
                                    for tx in min_x..max_x {
                                        let p = Vec2::new(tx as f32 + 0.5, ty as f32 + 0.5);

                                        // Evaluate the edges
                                        let edge0 = edges[0].evaluate(p);
                                        if edge0 >= 0.0 {
                                            let edge1 = edges[1].evaluate(p);
                                            if edge1 >= 0.0 {
                                                let edge2 = edges[2].evaluate(p);
                                                if edge2 >= 0.0 {
                                                    // Interpolate barycentric coordinates
                                                    let w =
                                                        self.barycentric_weights_3d(v0, v1, v2, p);

                                                    // Compute reciprocal depths (1 / z) for each vertex
                                                    let z0 = 1.0 / v0.z;
                                                    let z1 = 1.0 / v1.z;
                                                    let z2 = 1.0 / v2.z;

                                                    // Interpolate reciprocal depth
                                                    let one_over_z = z0 * w.x + z1 * w.y + z2 * w.z;
                                                    // let one_over_z =
                                                    // (w.x / v0.z) + (w.y / v1.z) + (w.z / v2.z);

                                                    let z = 1.0 - (1.0 / one_over_z);

                                                    // println!("z {}", z);
                                                    let zidx =
                                                        (ty - tile.y) * tile.width + (tx - tile.x);

                                                    if z < z_buffer[zidx] {
                                                        z_buffer[zidx] = z;

                                                        // Perspective-correct interpolation of UVs
                                                        let u_over_z = uv0.x * z0 * w.x
                                                            + uv1.x * z1 * w.y
                                                            + uv2.x * z2 * w.z;
                                                        let v_over_z = uv0.y * z0 * w.x
                                                            + uv1.y * z1 * w.y
                                                            + uv2.y * z2 * w.z;

                                                        let u = u_over_z / one_over_z;
                                                        let v = v_over_z / one_over_z;

                                                        // Interpolate UV coordinates
                                                        // let u = uv0.x * w.x + uv1.x * w.y + uv2.x * w.z;
                                                        // let v =
                                                        //     1.0 - (uv0.y * w.x + uv1.y * w.y + uv2.y * w.z);
                                                        // u = u.clamp(0.0, 1.0);
                                                        // v = v.clamp(0.0, 1.0);

                                                        // Sample the texture
                                                        let texel = atlas.sample_linear(u, v);
                                                        // let texel = [
                                                        //     (u * 255.0) as u8,
                                                        //     (v * 255.0) as u8,
                                                        //     0,
                                                        //     255,
                                                        // ];

                                                        // Write to framebuffer
                                                        let idx = ((ty - tile.y) * tile.width
                                                            + (tx - tile.x))
                                                            * 4;
                                                        buffer[idx..idx + 4]
                                                            .copy_from_slice(&texel);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Render 2D geometry on top of the 3D geometry (UI)
                for batch in batches_2d.iter() {
                    if let Some(bbox) = batch.bounding_box {
                        if bbox.x < (tile.x + tile.width) as f32
                            && (bbox.x + bbox.width) > tile.x as f32
                            && bbox.y < (tile.y + tile.height) as f32
                            && (bbox.y + bbox.height) > tile.y as f32
                        {
                            match batch.mode {
                                PrimitiveMode::Triangles => {
                                    // Process each triangle in the batch
                                    for (triangle_index, edges) in batch.edges.iter().enumerate() {
                                        let (i0, i1, i2) = batch.indices[triangle_index];
                                        let v0 = batch.projected_vertices[i0];
                                        let v1 = batch.projected_vertices[i1];
                                        let v2 = batch.projected_vertices[i2];
                                        let uv0 = batch.uvs[i0];
                                        let uv1 = batch.uvs[i1];
                                        let uv2 = batch.uvs[i2];

                                        // Compute bounding box of the triangle
                                        let min_x = [v0.x, v1.x, v2.x]
                                            .iter()
                                            .cloned()
                                            .fold(f32::INFINITY, f32::min)
                                            .floor()
                                            .max(tile.x as f32)
                                            as usize;
                                        let max_x = [v0.x, v1.x, v2.x]
                                            .iter()
                                            .cloned()
                                            .fold(f32::NEG_INFINITY, f32::max)
                                            .ceil()
                                            .min((tile.x + tile.width) as f32)
                                            as usize;
                                        let min_y = [v0.y, v1.y, v2.y]
                                            .iter()
                                            .cloned()
                                            .fold(f32::INFINITY, f32::min)
                                            .floor()
                                            .max(tile.y as f32)
                                            as usize;
                                        let max_y = [v0.y, v1.y, v2.y]
                                            .iter()
                                            .cloned()
                                            .fold(f32::NEG_INFINITY, f32::max)
                                            .ceil()
                                            .min((tile.y + tile.height) as f32)
                                            as usize;

                                        // Rasterize the triangle within its bounding box
                                        for ty in min_y..max_y {
                                            for tx in min_x..max_x {
                                                let p = Vec2::new(tx as f32 + 0.5, ty as f32 + 0.5);

                                                // Edge function tests for triangle rasterization
                                                let edge0 = edges[0].evaluate(p);
                                                if edge0 >= 0.0 {
                                                    let edge1 = edges[1].evaluate(p);
                                                    if edge1 >= 0.0 {
                                                        let edge2 = edges[2].evaluate(p);
                                                        if edge2 >= 0.0 {
                                                            // Interpolate barycentric coordinates
                                                            let w = self.barycentric_weights_2d(
                                                                v0, v1, v2, p,
                                                            );

                                                            // Interpolate UV coordinates
                                                            let u = uv0.x * w.x
                                                                + uv1.x * w.y
                                                                + uv2.x * w.z;
                                                            let v = 1.0
                                                                - (uv0.y * w.x
                                                                    + uv1.y * w.y
                                                                    + uv2.y * w.z);
                                                            // u = u.clamp(0.0, 1.0);
                                                            // v = v.clamp(0.0, 1.0);

                                                            // Sample the texture
                                                            let texel = atlas.sample(
                                                                u,
                                                                v,
                                                                batch.sample_mode,
                                                            );
                                                            // let texel = [(u * 255.0) as u8, (v * 255.0) as u8, 0, 255];

                                                            // Write to framebuffer
                                                            let idx = ((ty - tile.y) * tile.width
                                                                + (tx - tile.x))
                                                                * 4;
                                                            buffer[idx..idx + 4]
                                                                .copy_from_slice(&texel);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                PrimitiveMode::Lines => {
                                    for &(i0, i1, _) in batch.indices.iter() {
                                        let p0 = batch.projected_vertices[i0];
                                        let p1 = batch.projected_vertices[i1];

                                        self.rasterize_line_bresenham(
                                            Vec2::new(p0.x, p0.y),
                                            Vec2::new(p1.x, p1.y),
                                            &mut buffer[..],
                                            tile,
                                            &batch.color,
                                        );
                                    }
                                }
                                PrimitiveMode::LineStrip => {
                                    for i in 0..(batch.projected_vertices.len() - 1) {
                                        let p0 = batch.projected_vertices[i];
                                        let p1 = batch.projected_vertices[i + 1];

                                        self.rasterize_line_bresenham(
                                            Vec2::new(p0.x, p0.y),
                                            Vec2::new(p1.x, p1.y),
                                            &mut buffer[..],
                                            tile,
                                            &batch.color,
                                        );
                                    }
                                }

                                PrimitiveMode::LineLoop => {
                                    for i in 0..batch.projected_vertices.len() {
                                        let p0 = batch.projected_vertices[i];
                                        let p1 = batch.projected_vertices
                                            [(i + 1) % batch.projected_vertices.len()];

                                        self.rasterize_line_bresenham(
                                            Vec2::new(p0.x, p0.y),
                                            Vec2::new(p1.x, p1.y),
                                            &mut buffer[..],
                                            tile,
                                            &batch.color,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                buffer
            })
            .collect();

        // Combine tile buffers into the main framebuffer
        for (i, tile) in tiles.iter().enumerate() {
            let tile_buffer = &tile_buffers[i];
            for ty in 0..tile.height {
                for tx in 0..tile.width {
                    let px = tile.x + tx;
                    let py = tile.y + ty;

                    let src_idx = (ty * tile.width + tx) * 4;
                    let dst_idx = (py * width + px) * 4;

                    pixels[dst_idx..dst_idx + 4]
                        .copy_from_slice(&tile_buffer[src_idx..src_idx + 4]);
                }
            }
        }
    }

    /// Compute the barycentric weights for a Vec2
    fn barycentric_weights_2d(
        &self,
        a: Vec3<f32>,
        b: Vec3<f32>,
        c: Vec3<f32>,
        p: Vec2<f32>,
    ) -> Vec3<f32> {
        let ac = c - a;
        let ab = b - a;
        let ap = p - a;
        let pc = c - p;
        let pb = b - p;

        let area = ac.x * ab.y - ac.y * ab.x;
        let alpha = (pc.x * pb.y - pc.y * pb.x) / area;
        let beta = (ac.x * ap.y - ac.y * ap.x) / area;
        let gamma = 1.0 - alpha - beta;

        Vec3::new(alpha, beta, gamma)
    }

    /// Compute the barycentric weights for a Vec2
    fn barycentric_weights_3d(
        &self,
        a: Vec4<f32>,
        b: Vec4<f32>,
        c: Vec4<f32>,
        p: Vec2<f32>,
    ) -> Vec3<f32> {
        let ac = c - a;
        let ab = b - a;
        let ap = p - a;
        let pc = c - p;
        let pb = b - p;

        let area = ac.x * ab.y - ac.y * ab.x;
        let alpha = (pc.x * pb.y - pc.y * pb.x) / area;
        let beta = (ac.x * ap.y - ac.y * ap.x) / area;
        let gamma = 1.0 - alpha - beta;

        Vec3::new(alpha, beta, gamma)
    }

    /// Rasterize a line via Bresenham.
    #[allow(clippy::too_many_arguments)]
    fn rasterize_line_bresenham(
        &self,
        p0: Vec2<f32>,
        p1: Vec2<f32>,
        buffer: &mut [u8],
        tile: &TileRect,
        color: &Pixel,
    ) {
        let x0 = p0.x as isize;
        let y0 = p0.y as isize;
        let x1 = p1.x as isize;
        let y1 = p1.y as isize;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        while x != x1 || y != y1 {
            // Map (x, y) to tile coordinates
            let tx = (x - tile.x as isize) as usize;
            let ty = (y - tile.y as isize) as usize;

            if tx < tile.width && ty < tile.height {
                // Write to framebuffer
                let idx = (ty * tile.width + tx) * 4;
                buffer[idx..idx + 4].copy_from_slice(color);
            }

            let e2 = err * 2;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn _rasterize_line_with_width(
        &self,
        p0: Vec2<f32>,
        p1: Vec2<f32>,
        buffer: &mut [u8],
        tile: &TileRect,
        color: &Pixel,
        line_width: f32,
    ) {
        fn distance_to_line(p0: Vec2<f32>, p1: Vec2<f32>, p: Vec2<f32>) -> f32 {
            let line = p1 - p0;
            let to_point = p - p0;
            let cross = line.x * to_point.y - line.y * to_point.x;
            cross.abs() / line.magnitude()
        }

        let min_x = p0.x.min(p1.x) - line_width;
        let max_x = p0.x.max(p1.x) + line_width;
        let min_y = p0.y.min(p1.y) - line_width;
        let max_y = p0.y.max(p1.y) + line_width;

        for ty in min_y.floor() as usize..=max_y.ceil() as usize {
            for tx in min_x.floor() as usize..=max_x.ceil() as usize {
                let p = Vec2::new(tx as f32 + 0.5, ty as f32 + 0.5);
                let dist = distance_to_line(p0, p1, p);

                if dist <= line_width / 2.0 {
                    let tile_tx = tx - tile.x;
                    let tile_ty = ty - tile.y;

                    if tile_tx < tile.width && tile_ty < tile.height {
                        let idx = (tile_ty * tile.width + tile_tx) * 4;
                        buffer[idx..idx + 4].copy_from_slice(color);
                    }
                }
            }
        }
    }

    //// Edge function for a triangle for a Vec2
    // fn edge_function_2d(&self, v0: Vec3<f32>, v1: Vec3<f32>, p: Vec2<f32>) -> f32 {
    //     let edge = v1 - v0;
    //     let to_point = p - v0;
    //     edge.x * to_point.y - edge.y * to_point.x
    // }

    //// Edge function for a triangle for a Vec3
    // fn edge_function_3d(&self, v0: Vec4<f32>, v1: Vec4<f32>, p: Vec2<f32>) -> f32 {
    //     let edge = Vec2::new(v1.x - v0.x, v1.y - v0.y);
    //     let to_point = Vec2::new(p.x - v0.x, p.y - v0.y);
    //     edge.x * to_point.y - edge.y * to_point.x
    // }

    // fn edge_function_3d(&self, v0: Vec3<f32>, v1: Vec3<f32>, p: Vec2<f32>) -> f32 {
    //     (p.x - v0.x) * (v1.y - v0.y) - (p.y - v0.y) * (v1.x - v0.x)
    // }
}

/// A rectangle struct which represents a Tile
#[derive(Clone, Copy)]
struct TileRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
