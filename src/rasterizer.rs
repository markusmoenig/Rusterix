use crate::SampleMode;
use crate::{
    pixel_to_vec4, vec4_to_pixel, Batch, CompiledLight, LightType, MapMini, Pixel, PrimitiveMode,
    Scene,
};
use rayon::prelude::*;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};

use SampleMode::*;

pub struct Rasterizer {
    pub projection_matrix_2d: Option<Mat3<f32>>,

    pub view_matrix: Mat4<f32>,
    pub projection_matrix: Mat4<f32>,

    pub inverse_view_matrix: Mat4<f32>,
    pub inverse_projection_matrix: Mat4<f32>,
    pub width: f32,
    pub height: f32,
    pub camera_pos: Vec3<f32>,

    /// For D2 grid space conversion when we dont use a translation matrix
    pub mapmini: MapMini,

    /// SampleMode, default is Nearest.
    pub sample_mode: SampleMode,

    /// The compliled lights in the scene.
    pub compiled_lights: Vec<CompiledLight>,

    /// Hash for animation
    pub hash_anim: u32,

    /// Background color (Sky etc.)
    pub background_color: Option<[u8; 4]>,

    /// 2D Translation
    pub translationd2: Vec2<f32>,
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

        let mut translationd2 = Vec2::new(0.0, 0.0);
        if let Some(projection_matrix_2d) = projection_matrix_2d {
            translationd2.x = projection_matrix_2d[(0, 2)];
            translationd2.y = projection_matrix_2d[(1, 2)];
        }

        Self {
            inverse_view_matrix,
            inverse_projection_matrix: projection_matrix.inverted(),

            projection_matrix_2d,
            view_matrix,
            projection_matrix,

            width: 0.0,
            height: 0.0,

            camera_pos,

            mapmini: MapMini::default(),
            sample_mode: Nearest,

            compiled_lights: vec![],
            hash_anim: 0,

            background_color: None,

            translationd2,
        }
    }

    /// Sets the sample mode using the builder pattern.
    pub fn sample_mode(mut self, sample_mode: SampleMode) -> Self {
        self.sample_mode = sample_mode;
        self
    }

    /// Sets the background using the builder pattern.
    pub fn background(mut self, background: Pixel) -> Self {
        self.background_color = Some(background);
        self
    }

    /// Rasterize the scene.
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

        /// Generate a hash value for the given animation frame.
        /// We use it for random light flickering.
        fn hash_u32(seed: u32) -> u32 {
            let mut state = seed;
            state = (state ^ 61) ^ (state >> 16);
            state = state.wrapping_add(state << 3);
            state ^= state >> 4;
            state = state.wrapping_mul(0x27d4eb2d);
            state ^= state >> 15;
            state
        }
        self.hash_anim = hash_u32(scene.animation_frame as u32);

        scene.project(
            self.projection_matrix_2d,
            self.view_matrix,
            self.projection_matrix,
            width,
            height,
        );

        self.compiled_lights = scene.compile_lights(self.background_color);

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
                // Local tile buffer, fill with background color if needed
                let mut buffer = vec![0; tile.width * tile.height * 4];
                if let Some(background_color) = &self.background_color {
                    for chunk in buffer.chunks_exact_mut(4) {
                        chunk.copy_from_slice(background_color);
                    }
                }

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
                    self.d3_rasterize(&mut buffer, &mut z_buffer, tile, batch, scene, false);
                }

                for batch in scene.d3_dynamic.iter() {
                    self.d3_rasterize(&mut buffer, &mut z_buffer, tile, batch, scene, true);
                }

                // Render 2D geometry on top of the 3D geometry (UI)
                for batch in scene.d2_static.iter() {
                    self.d2_rasterize(&mut buffer, tile, batch, scene, false);
                }

                for batch in scene.d2_dynamic.iter() {
                    self.d2_rasterize(&mut buffer, tile, batch, scene, true);
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
        scene: &Scene,
        dynamic: bool,
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
                                    let mut p = [tx as f32 + 0.5, ty as f32 + 0.5];

                                    // Wrap coordinates if they are out of bounds
                                    if p[0] >= (tile.x + tile.width) as f32 {
                                        p[0] -= tile.width as f32;
                                    } else if p[0] < tile.x as f32 {
                                        p[0] += tile.width as f32;
                                    }

                                    if p[1] >= (tile.y + tile.height) as f32 {
                                        p[1] -= tile.height as f32;
                                    } else if p[1] < tile.y as f32 {
                                        p[1] += tile.height as f32;
                                    }

                                    // Evaluate the edges
                                    if edges.visible && edges.evaluate(p) {
                                        // Interpolate barycentric coordinates
                                        let w = self.barycentric_weights_2d(&v0, &v1, &v2, &p);

                                        // Interpolate UV coordinates
                                        let u = uv0[0] * w[0] + uv1[0] * w[1] + uv2[0] * w[2];
                                        let v = uv0[1] * w[0] + uv1[1] * w[1] + uv2[1] * w[2];

                                        // Sample the texture

                                        let textures = if dynamic {
                                            &scene.dynamic_textures
                                        } else {
                                            &scene.textures
                                        };

                                        let t = &textures[batch.texture_index];
                                        let index = scene.animation_frame % t.textures.len();

                                        let mut texel = t.textures[index].sample(
                                            u,
                                            v,
                                            self.sample_mode,
                                            batch.repeat_mode,
                                        );

                                        if batch.receives_light
                                            && (!scene.lights.is_empty()
                                                || !scene.dynamic_lights.is_empty())
                                        {
                                            let mut accumulated_light = [0.0, 0.0, 0.0];

                                            // Calculate grid position
                                            let grid_space_pos = Vec2::new(tx as f32, ty as f32)
                                                - Vec2::new(self.width, self.height) / 2.0
                                                - Vec2::new(
                                                    // self.mapmini.offset.x,
                                                    // -self.mapmini.offset.y,
                                                    self.translationd2.x - self.width / 2.0,
                                                    self.translationd2.y - self.height / 2.0,
                                                );
                                            let world = grid_space_pos / self.mapmini.grid_size;

                                            for light in &self.compiled_lights {
                                                if let Some(mut light_color) = light.color_at(
                                                    Vec3::new(world.x, 0.0, world.y),
                                                    &self.hash_anim,
                                                    true,
                                                ) {
                                                    let mut light_is_visible = true;

                                                    // Sector daylight occlusion
                                                    if light.light_type
                                                        == LightType::AmbientDaylight
                                                    {
                                                        let occlusion =
                                                            self.mapmini.get_occlusion(world);
                                                        light_color[0] *= occlusion;
                                                        light_color[1] *= occlusion;
                                                        light_color[2] *= occlusion;
                                                    }

                                                    if light.light_type != LightType::Ambient
                                                        && light.light_type
                                                            != LightType::AmbientDaylight
                                                        && !self
                                                            .mapmini
                                                            .is_visible(world, light.position_2d())
                                                    {
                                                        light_is_visible = false;
                                                    }

                                                    if light_is_visible {
                                                        accumulated_light[0] += light_color[0];
                                                        accumulated_light[1] += light_color[1];
                                                        accumulated_light[2] += light_color[2];
                                                    }
                                                }
                                            }

                                            accumulated_light[0] =
                                                accumulated_light[0].clamp(0.0, 1.0);
                                            accumulated_light[1] =
                                                accumulated_light[1].clamp(0.0, 1.0);
                                            accumulated_light[2] =
                                                accumulated_light[2].clamp(0.0, 1.0);

                                            for i in 0..3 {
                                                texel[i] = ((texel[i] as f32 / 255.0)
                                                    * accumulated_light[i]
                                                    * 255.0)
                                                    .clamp(0.0, 255.0)
                                                    as u8;
                                            }
                                        }

                                        // Copy or blend to framebuffer
                                        let idx = ((ty - tile.y) * tile.width + (tx - tile.x)) * 4;

                                        if texel[3] == 255 {
                                            buffer[idx..idx + 4].copy_from_slice(&texel);
                                        } else {
                                            let src_alpha = texel[3] as f32 / 255.0;
                                            let dst_alpha = 1.0 - src_alpha;

                                            for i in 0..3 {
                                                buffer[idx + i] = ((texel[i] as f32 * src_alpha)
                                                    + (buffer[idx + i] as f32 * dst_alpha))
                                                    as u8;
                                            }
                                            buffer[idx + 3] = 255;
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

    /// Rasterizes a 3D batch.
    #[inline(always)]
    fn d3_rasterize(
        &self,
        buffer: &mut [u8],
        z_buffer: &mut [f32],
        tile: &TileRect,
        batch: &Batch<[f32; 4]>,
        scene: &Scene,
        dynamic: bool,
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

                                            // Sample the texture
                                            let textures = if dynamic {
                                                &scene.dynamic_textures
                                            } else {
                                                &scene.textures
                                            };

                                            let textile = &textures[batch.texture_index];
                                            let index =
                                                scene.animation_frame % textile.textures.len();

                                            // let distance = (world - self.camera_pos).magnitude();
                                            // let start_distance = 8.0;
                                            // let ramp_distance = 4.0;

                                            // let mut blur = ((distance - start_distance)
                                            //     / ramp_distance)
                                            //     .clamp(0.0, 1.0);
                                            // blur = blur * blur * (3.0 - 2.0 * blur);
                                            // blur = blur.clamp(0.0, 1.0);

                                            let texel = textile.textures[index].sample(
                                                interpolated_u,
                                                interpolated_v,
                                                self.sample_mode,
                                                batch.repeat_mode,
                                            );

                                            let mut color = pixel_to_vec4(&texel);

                                            // -- Pipeline

                                            /*
                                            if batch.receives_light {
                                                // Distance based bayer matrix dithering
                                                // Distance to camera
                                                let distance =
                                                    (world - self.camera_pos).magnitude();

                                                // TODO: Make this configurable
                                                let start_distance = 2.0;
                                                let ramp_distance = 2.0;
                                                let jitter_scale = 0.028;

                                                let mut t = ((distance - start_distance)
                                                    / ramp_distance)
                                                    .clamp(0.0, 1.0);
                                                t = t * t * (3.0 - 2.0 * t);

                                                const BAYER_8X8: [[i32; 8]; 8] = [
                                                    [0, 32, 8, 40, 2, 34, 10, 42],
                                                    [48, 16, 56, 24, 50, 18, 58, 26],
                                                    [12, 44, 4, 36, 14, 46, 6, 38],
                                                    [60, 28, 52, 20, 62, 30, 54, 22],
                                                    [3, 35, 11, 43, 1, 33, 9, 41],
                                                    [51, 19, 59, 27, 49, 17, 57, 25],
                                                    [15, 47, 7, 39, 13, 45, 5, 37],
                                                    [63, 31, 55, 23, 61, 29, 53, 21],
                                                ];

                                                let threshold =
                                                    BAYER_8X8[ty % 8][tx % 8] as f32 / 64.0 - 0.5;

                                                fn hash(x: f32, y: f32) -> f32 {
                                                    // Convert x and y to integers by scaling and truncating
                                                    let ix = x as i32;
                                                    let iy = y as i32;

                                                    // Combine the integer parts using bitwise operations
                                                    let mut n = (ix ^ iy) as u32;
                                                    n ^= n << 13;
                                                    n ^= n >> 17;
                                                    n ^= n << 5;

                                                    // Normalize the result to [0, 1]
                                                    n as f32 / u32::MAX as f32
                                                }

                                                let jitter = (hash(p[0], p[1])) * 0.2 * t; //threshold * 0.5 * t; // * jitter_scale; // * t;

                                                color[0] += jitter;
                                                color[1] += jitter;
                                                color[3] += jitter;
                                            }*/

                                            // Distance based fog
                                            /*
                                            if batch.receives_light {
                                                let distance =
                                                    (world - self.camera_pos).magnitude();

                                                let fog_density = 0.1;
                                                let fog_factor = (-fog_density * distance).exp();
                                                let fog_factor = fog_factor.clamp(0.0, 1.0);

                                                let fog_color = [1.0, 1.0, 1.0];

                                                for i in 0..3 {
                                                    color[i] = color[i] * fog_factor
                                                        + fog_color[i] * (1.0 - fog_factor);
                                                }
                                            }*/

                                            // Sample Lights
                                            if batch.receives_light
                                                && (!self.compiled_lights.is_empty())
                                            {
                                                let mut accumulated_light: [f32; 3] =
                                                    [0.0, 0.0, 0.0];
                                                let world_2d = Vec2::new(world.x, world.z);
                                                let epsilon = 0.01;

                                                for light in &self.compiled_lights {
                                                    if let Some(mut light_color) = light.color_at(
                                                        world,
                                                        &self.hash_anim,
                                                        false,
                                                    ) {
                                                        let direction_to_light =
                                                            (light.position_2d() - world_2d)
                                                                .normalized();
                                                        let offset_world_2d =
                                                            world_2d + direction_to_light * epsilon;

                                                        // Sector daylight occlusion
                                                        if light.light_type
                                                            == LightType::AmbientDaylight
                                                        {
                                                            let occlusion =
                                                                self.mapmini.get_occlusion(
                                                                    Vec2::new(world.x, world.y),
                                                                );
                                                            light_color[0] *= occlusion;
                                                            light_color[1] *= occlusion;
                                                            light_color[2] *= occlusion;
                                                        }

                                                        let mut light_is_visible = true;
                                                        if light.light_type != LightType::Ambient
                                                            && light.light_type
                                                                != LightType::AmbientDaylight
                                                            && !self.mapmini.is_visible(
                                                                offset_world_2d,
                                                                light.position_2d(),
                                                            )
                                                        {
                                                            light_is_visible = false;
                                                        }

                                                        if light_is_visible {
                                                            accumulated_light[0] += light_color[0];
                                                            accumulated_light[1] += light_color[1];
                                                            accumulated_light[2] += light_color[2];
                                                        }
                                                    }
                                                }

                                                color[0] *= accumulated_light[0].clamp(0.0, 1.0);
                                                color[1] *= accumulated_light[1].clamp(0.0, 1.0);
                                                color[2] *= accumulated_light[2].clamp(0.0, 1.0);
                                            }

                                            // ---

                                            let texel = vec4_to_pixel(&color);
                                            if texel[3] == 255 {
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

    // Gamma correction in final output
    #[allow(dead_code)]
    #[inline(always)]
    fn gamma_correct(&self, color: [u8; 4]) -> [u8; 4] {
        let gamma = 2.2;
        [
            (f32::powf(color[0] as f32 / 255.0, 1.0 / gamma) * 255.0) as u8,
            (f32::powf(color[1] as f32 / 255.0, 1.0 / gamma) * 255.0) as u8,
            (f32::powf(color[2] as f32 / 255.0, 1.0 / gamma) * 255.0) as u8,
            color[3],
        ]
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
    #[inline(always)]
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
    #[inline(always)]
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
