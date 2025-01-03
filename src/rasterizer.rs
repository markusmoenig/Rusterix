use crate::{Batch, Light, Pixel, PrimitiveMode, Scene, Texture};
use rayon::prelude::*;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};
pub struct Rasterizer {
    pub projection_matrix_2d: Option<Mat3<f32>>,

    pub view_matrix: Mat4<f32>,
    pub projection_matrix: Mat4<f32>,

    pub inverse_view_matrix: Mat4<f32>,
    pub inverse_projection_matrix: Mat4<f32>,
    pub width: f32,
    pub height: f32,
    pub camera_pos: Vec3<f32>,
}

/// Rasterizes batches of 2D and 3D meshes (and lines).
impl Rasterizer {
    pub fn setup(
        projection_matrix_2d: Option<Mat3<f32>>,
        view_matrix: Mat4<f32>,
        projection_matrix: Mat4<f32>,
    ) -> Self {
        let inverse_view_matrix = view_matrix.inverted();
        let camera_pos = Vec3::new(
            inverse_view_matrix.cols[3].x,
            inverse_view_matrix.cols[3].y,
            inverse_view_matrix.cols[3].z,
        );

        Self {
            inverse_view_matrix,
            inverse_projection_matrix: projection_matrix.inverted(),

            projection_matrix_2d,
            view_matrix,
            projection_matrix,

            width: 0.0,
            height: 0.0,

            camera_pos,
        }
    }

    pub fn rasterize(
        &mut self,
        scene: &mut Scene,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        tile_size: usize,
    ) {
        self.width = width as f32;
        self.height = height as f32;

        scene.project(
            self.projection_matrix_2d,
            self.view_matrix,
            self.projection_matrix,
            width,
            height,
        );

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

        let screen_size = Vec2::new(width as f32, height as f32);

        // Parallel process each tile
        let tile_buffers: Vec<Vec<u8>> = tiles
            .par_iter()
            .map(|tile| {
                // Local tile buffer
                let mut buffer = vec![0; tile.width * tile.height * 4];
                let mut z_buffer = vec![1.0_f32; tile.width * tile.height];

                if let Some(shader) = &scene.background {
                    for ty in 0..tile.height {
                        for tx in 0..tile.width {
                            let pixel = shader.shade_pixel(
                                Vec2::new(
                                    (tile.x + tx) as f32 / screen_size.x,
                                    (tile.y + ty) as f32 / screen_size.y,
                                ),
                                screen_size,
                            );
                            let idx = (ty * tile.width + tx) * 4;
                            buffer[idx..idx + 4].copy_from_slice(&pixel);
                        }
                    }
                }

                for batch in scene.d3_static.iter() {
                    self.d3_rasterize(
                        &mut buffer,
                        &mut z_buffer,
                        tile,
                        batch,
                        &scene.textures,
                        &scene.lights,
                    );
                }

                for batch in scene.d3_dynamic.iter() {
                    self.d3_rasterize(
                        &mut buffer,
                        &mut z_buffer,
                        tile,
                        batch,
                        &scene.textures,
                        &scene.lights,
                    );
                }

                // Render 2D geometry on top of the 3D geometry (UI)
                for batch in scene.d2.iter() {
                    self.d2_rasterize(&mut buffer, tile, batch, &scene.textures);
                }

                buffer
            })
            .collect();

        // Combine tile buffers into the main framebuffer
        for (i, tile) in tiles.iter().enumerate() {
            let tile_buffer = &tile_buffers[i];
            let px_start = tile.x;
            let py_start = tile.y;

            let tile_row_bytes = tile.width * 4; // Number of bytes in a tile row
            let framebuffer_row_bytes = width * 4; // Number of bytes in a framebuffer row

            let mut src_offset = 0;
            let mut dst_offset = (py_start * width + px_start) * 4;

            for _ in 0..tile.height {
                pixels[dst_offset..dst_offset + tile_row_bytes]
                    .copy_from_slice(&tile_buffer[src_offset..src_offset + tile_row_bytes]);

                // Increment offsets
                src_offset += tile_row_bytes;
                dst_offset += framebuffer_row_bytes;
            }
        }
    }

    /// Rasterizes a 2D batch.
    #[inline(always)]
    fn d2_rasterize(
        &self,
        buffer: &mut [u8],
        tile: &TileRect,
        batch: &Batch<[f32; 3]>,
        textures: &[Texture],
    ) {
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
                            let min_x = [v0[0], v1[0], v2[0]]
                                .iter()
                                .cloned()
                                .fold(f32::INFINITY, f32::min)
                                .floor()
                                .max(tile.x as f32)
                                as usize;
                            let max_x = [v0[0], v1[0], v2[0]]
                                .iter()
                                .cloned()
                                .fold(f32::NEG_INFINITY, f32::max)
                                .ceil()
                                .min((tile.x + tile.width) as f32)
                                as usize;
                            let min_y = [v0[1], v1[1], v2[1]]
                                .iter()
                                .cloned()
                                .fold(f32::INFINITY, f32::min)
                                .floor()
                                .max(tile.y as f32)
                                as usize;
                            let max_y = [v0[1], v1[1], v2[1]]
                                .iter()
                                .cloned()
                                .fold(f32::NEG_INFINITY, f32::max)
                                .ceil()
                                .min((tile.y + tile.height) as f32)
                                as usize;

                            // Rasterize the triangle within its bounding box
                            for ty in min_y..max_y {
                                for tx in min_x..max_x {
                                    let p = [tx as f32 + 0.5, ty as f32 + 0.5];

                                    // Evaluate the edges
                                    if edges.visible && edges.evaluate(p) {
                                        // Interpolate barycentric coordinates
                                        let w = self.barycentric_weights_2d(&v0, &v1, &v2, &p);

                                        // Interpolate UV coordinates
                                        let u = uv0[0] * w[0] + uv1[0] * w[1] + uv2[0] * w[2];
                                        let v = uv0[1] * w[0] + uv1[1] * w[1] + uv2[1] * w[2];

                                        // Sample the texture
                                        let texel = textures[batch.texture_index].sample(
                                            u,
                                            v,
                                            batch.sample_mode,
                                            batch.repeat_mode,
                                        );
                                        // let texel = [(u * 255.0) as u8, (v * 255.0) as u8, 0, 255];

                                        // Write to framebuffer
                                        let idx = ((ty - tile.y) * tile.width + (tx - tile.x)) * 4;
                                        buffer[idx..idx + 4].copy_from_slice(&texel);
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
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
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
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
                                &mut buffer[..],
                                tile,
                                &batch.color,
                            );
                        }
                    }

                    PrimitiveMode::LineLoop => {
                        for i in 0..batch.projected_vertices.len() {
                            let p0 = batch.projected_vertices[i];
                            let p1 =
                                batch.projected_vertices[(i + 1) % batch.projected_vertices.len()];

                            self.rasterize_line_bresenham(
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
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

    /// Rasterizes a 3D batch.
    #[inline(always)]
    fn d3_rasterize(
        &self,
        buffer: &mut [u8],
        z_buffer: &mut [f32],
        tile: &TileRect,
        batch: &Batch<[f32; 4]>,
        textures: &[Texture],
        lights: &[Light],
    ) {
        // Bounding box check for the tile with the batch bbox
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
                            if !edges.visible {
                                continue;
                            }

                            let (i0, i1, i2) = batch.clipped_indices[triangle_index];
                            let v0 = batch.projected_vertices[i0];
                            let v1 = batch.projected_vertices[i1];
                            let v2 = batch.projected_vertices[i2];
                            let uv0 = batch.clipped_uvs[i0];
                            let uv1 = batch.clipped_uvs[i1];
                            let uv2 = batch.clipped_uvs[i2];

                            // Compute bounding box of the triangle
                            let min_x = [v0[0], v1[0], v2[0]]
                                .iter()
                                .cloned()
                                .fold(f32::INFINITY, f32::min)
                                .floor()
                                .max(tile.x as f32)
                                as usize;
                            let max_x = [v0[0], v1[0], v2[0]]
                                .iter()
                                .cloned()
                                .fold(f32::NEG_INFINITY, f32::max)
                                .ceil()
                                .min((tile.x + tile.width) as f32)
                                as usize;
                            let min_y = [v0[1], v1[1], v2[1]]
                                .iter()
                                .cloned()
                                .fold(f32::INFINITY, f32::min)
                                .floor()
                                .max(tile.y as f32)
                                as usize;
                            let max_y = [v0[1], v1[1], v2[1]]
                                .iter()
                                .cloned()
                                .fold(f32::NEG_INFINITY, f32::max)
                                .ceil()
                                .min((tile.y + tile.height) as f32)
                                as usize;

                            // Rasterize the triangle within its bounding box
                            for ty in min_y..max_y {
                                for tx in min_x..max_x {
                                    let p = [tx as f32 + 0.5, ty as f32 + 0.5];

                                    // Evaluate the edges
                                    if edges.evaluate(p) {
                                        // Interpolate barycentric coordinates
                                        let [alpha, beta, gamma] =
                                            self.barycentric_weights_3d(&v0, &v1, &v2, &p);

                                        let one_over_z = 1.0 / v0[2] * alpha
                                            + 1.0 / v1[2] * beta
                                            + 1.0 / v2[2] * gamma;
                                        let z = 1.0 / one_over_z;

                                        let zidx = (ty - tile.y) * tile.width + (tx - tile.x);

                                        if z < z_buffer[zidx] {
                                            z_buffer[zidx] = z;

                                            // Perform the interpolation of all U/w and V/w values using barycentric weights and a factor of 1/w
                                            let mut interpolated_u = (uv0[0] / v0[3]) * alpha
                                                + (uv1[0] / v1[3]) * beta
                                                + (uv2[0] / v2[3]) * gamma;
                                            let mut interpolated_v = (uv0[1] / v0[3]) * alpha
                                                + (uv1[1] / v1[3]) * beta
                                                + (uv2[1] / v2[3]) * gamma;

                                            // Interpolate reciprocal depth
                                            let interpolated_reciprocal_w = (1.0 / v0[3]) * alpha
                                                + (1.0 / v1[3]) * beta
                                                + (1.0 / v2[3]) * gamma;

                                            // Now we can divide back both interpolated values by 1/w
                                            interpolated_u /= interpolated_reciprocal_w;
                                            interpolated_v /= interpolated_reciprocal_w;

                                            // Get the screen coordinates of the hitpoint
                                            let world = self.screen_to_world(p[0], p[1], z);

                                            if batch.receives_light {
                                                // Distance based UV jittering to hide Moire patterns
                                                // Distance to camera
                                                let distance =
                                                    (world - self.camera_pos).magnitude();

                                                // TODO: Make this configurable
                                                let start_distance = 5.0;
                                                let ramp_distance = 3.0;
                                                let jitter_scale = 0.028;

                                                let mut t = ((distance - start_distance)
                                                    / ramp_distance)
                                                    .clamp(0.0, 1.0);
                                                t = t * t * (3.0 - 2.0 * t);

                                                let jitter_x =
                                                    self.hash_uv(tx, ty) * t * jitter_scale;
                                                let jitter_y =
                                                    self.hash_uv(ty, tx) * t * jitter_scale;

                                                interpolated_u += jitter_x;
                                                interpolated_v += jitter_y;
                                            }

                                            // Sample the texture
                                            let mut texel = textures[batch.texture_index].sample(
                                                interpolated_u,
                                                interpolated_v,
                                                batch.sample_mode,
                                                batch.repeat_mode,
                                            );

                                            if batch.receives_light {
                                                // Calc Lights
                                                for light in lights {
                                                    let light_color = light.color_at(world, 0.0);
                                                    texel[0] = ((texel[0] as f32 / 255.0)
                                                        * light_color[0]
                                                        * 255.0)
                                                        .clamp(0.0, 255.0)
                                                        as u8;
                                                    texel[1] = ((texel[1] as f32 / 255.0)
                                                        * light_color[1]
                                                        * 255.0)
                                                        .clamp(0.0, 255.0)
                                                        as u8;
                                                    texel[2] = ((texel[2] as f32 / 255.0)
                                                        * light_color[2]
                                                        * 255.0)
                                                        .clamp(0.0, 255.0)
                                                        as u8;
                                                }
                                            }

                                            if texel[3] == 255 {
                                                /*
                                                let fog_color = WHITE;
                                                let fog_intensity = 0.4;
                                                t *= fog_intensity;

                                                texel[0] = (texel[0] as f32 * (1.0 - t)
                                                    + fog_color[0] as f32 * t)
                                                    as u8;
                                                texel[1] = (texel[1] as f32 * (1.0 - t)
                                                    + fog_color[1] as f32 * t)
                                                    as u8;
                                                texel[2] = (texel[2] as f32 * (1.0 - t)
                                                    + fog_color[2] as f32 * t)
                                                    as u8;*/

                                                let idx = ((ty - tile.y) * tile.width
                                                    + (tx - tile.x))
                                                    * 4;
                                                buffer[idx..idx + 4].copy_from_slice(&texel);
                                                z_buffer[zidx] = z;
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
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
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
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
                                &mut buffer[..],
                                tile,
                                &batch.color,
                            );
                        }
                    }

                    PrimitiveMode::LineLoop => {
                        for i in 0..batch.projected_vertices.len() {
                            let p0 = batch.projected_vertices[i];
                            let p1 =
                                batch.projected_vertices[(i + 1) % batch.projected_vertices.len()];

                            self.rasterize_line_bresenham(
                                &[p0[0], p0[1]],
                                &[p1[0], p1[1]],
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

    /// A simple hash for creating noise to hide Moire patterns while moving
    #[inline(always)]
    fn hash_uv(&self, x: usize, y: usize) -> f32 {
        // Simple hash function for UV jittering, returning a value between -0.5 and 0.5
        let seed = 0x9E3779B9; // Arbitrary prime constant
        let mut hash = x as u32;
        hash = hash.wrapping_mul(seed).wrapping_add(y as u32);
        hash = hash ^ (hash >> 16);
        hash = hash.wrapping_mul(seed);
        ((hash & 0xFF) as f32 / 255.0) - 0.5
    }

    /// Convert screen coordinate to world coordinate
    #[inline(always)]
    fn screen_to_world(
        &self,
        x: f32,     // Screen-space x
        y: f32,     // Screen-space y
        z_ndc: f32, // Z-value
    ) -> Vec3<f32> {
        // Step 1: Convert screen space to NDC
        let x_ndc = 2.0 * (x / self.width) - 1.0;
        let y_ndc = 1.0 - 2.0 * (y / self.height); // Flip Y-axis
        let ndc = Vec4::new(x_ndc, y_ndc, z_ndc, 1.0);

        // Step 2: Transform from NDC to View Space
        let view_space: Vec4<f32> = self.inverse_projection_matrix * ndc; // Ensure ndc is Vec4
        let view_space = view_space / view_space.w; // Perspective division

        // Step 3: Transform from View Space to World Space
        let world_space: Vec4<f32> = self.inverse_view_matrix * view_space;

        // Return the world-space coordinates (drop the W component)
        Vec3::new(world_space.x, world_space.y, world_space.z)
    }

    /// Compute the barycentric weights for a Vec2
    fn barycentric_weights_2d(
        &self,
        a: &[f32; 3],
        b: &[f32; 3],
        c: &[f32; 3],
        p: &[f32; 2],
    ) -> [f32; 3] {
        let ac = [c[0] - a[0], c[1] - a[1]];
        let ab = [b[0] - a[0], b[1] - a[1]];
        let ap = [p[0] - a[0], p[1] - a[1]];
        let pc = [c[0] - p[0], c[1] - p[1]];
        let pb = [b[0] - p[0], b[1] - p[1]];

        let area = ac[0] * ab[1] - ac[1] * ab[0];
        let alpha = (pc[0] * pb[1] - pc[1] * pb[0]) / area;
        let beta = (ac[0] * ap[1] - ac[1] * ap[0]) / area;
        let gamma = 1.0 - alpha - beta;

        [alpha, beta, gamma]
    }

    /// Compute the barycentric weights for a Vec2
    fn barycentric_weights_3d(
        &self,
        a: &[f32; 4],
        b: &[f32; 4],
        c: &[f32; 4],
        p: &[f32; 2],
    ) -> [f32; 3] {
        let ac = [c[0] - a[0], c[1] - a[1]];
        let ab = [b[0] - a[0], b[1] - a[1]];
        let ap = [p[0] - a[0], p[1] - a[1]];
        let pc = [c[0] - p[0], c[1] - p[1]];
        let pb = [b[0] - p[0], b[1] - p[1]];

        let area = ac[0] * ab[1] - ac[1] * ab[0];
        let alpha = (pc[0] * pb[1] - pc[1] * pb[0]) / area;
        let beta = (ac[0] * ap[1] - ac[1] * ap[0]) / area;
        let gamma = 1.0 - alpha - beta;

        [alpha, beta, gamma]
    }

    /// Rasterize a line via Bresenham.
    #[allow(clippy::too_many_arguments)]
    fn rasterize_line_bresenham(
        &self,
        p0: &[f32; 2],
        p1: &[f32; 2],
        buffer: &mut [u8],
        tile: &TileRect,
        color: &Pixel,
    ) {
        let x0 = p0[0] as isize;
        let y0 = p0[1] as isize;
        let x1 = p1[0] as isize;
        let y1 = p1[1] as isize;

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
}

/// A rectangle struct which represents a Tile
#[derive(Clone, Copy)]
struct TileRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}
