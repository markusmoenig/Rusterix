use crate::{
    Assets, Batch2D, Batch3D, Chunk, LightType, MapMini, Pixel, PixelSource, PrimitiveMode, Ray,
    RenderMode, Scene, pixel_to_vec4, vec4_to_pixel,
};
use crate::{SampleMode, ShapeFXGraph};
use rayon::prelude::*;
use rusteria::Execution;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};

use SampleMode::*;

#[derive(Clone, PartialEq)]
pub struct BrushPreview {
    pub position: Vec3<f32>,
    pub radius: f32,
    pub falloff: f32,
}

#[inline(always)]
fn srgb_to_linear_fast(x: f32) -> f32 {
    // Approximate powf(x, 2.2)
    // Polynomial fit, max abs error ≈ 0.008
    let x2 = x * x;
    (0.697_5 * x2 + 0.302_5) * x
}

#[inline(always)]
fn linear_to_srgb_fast(x: f32) -> f32 {
    // Approximate powf(x, 1.0/2.2)
    // Polynomial fit, max abs error ≈ 0.006
    let sqrt_x = x.sqrt();
    1.055 * sqrt_x - 0.055 * sqrt_x * sqrt_x
}

pub struct Rasterizer {
    pub render_mode: RenderMode,

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

    /// Hash for animation
    pub hash_anim: u32,

    /// Background color (Sky etc.)
    pub background_color: Option<[u8; 4]>,

    /// Ambient color (Sky etc.)
    pub ambient_color: Option<Vec4<f32>>,

    /// Optional brush preview
    pub brush_preview: Option<BrushPreview>,

    /// 2D Translation / Scaling
    translationd2: Vec2<f32>,
    scaled2: f32,

    /// Useful when the resulting framebuffer is used as an blended overlay
    pub preserve_transparency: bool,

    /// The rendergraph
    pub render_graph: ShapeFXGraph,
    render_hit: Vec<u16>,
    render_miss: Vec<u16>,

    /// The hour of the day.Used for procedural sky and ambient.
    pub hour: f32,

    /// Timer, used for shading animations
    pub time: f32,

    /// Optional sun direction provided by the Sky node
    pub sun_dir: Option<Vec3<f32>>,
    pub day_factor: f32,
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
        let mut scaled2 = 1.0;
        if let Some(projection_matrix_2d) = projection_matrix_2d {
            translationd2.x = projection_matrix_2d[(0, 2)];
            translationd2.y = projection_matrix_2d[(1, 2)];
            scaled2 = projection_matrix_2d[(0, 0)];
        }

        Self {
            render_mode: RenderMode::render_all(),

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

            hash_anim: 0,

            background_color: None,
            ambient_color: None,

            brush_preview: None,

            translationd2,
            scaled2,

            preserve_transparency: false,

            render_graph: ShapeFXGraph::default(),
            render_hit: vec![],
            render_miss: vec![],

            hour: 12.0,
            time: 0.0,

            sun_dir: None,
            day_factor: 0.0,
        }
    }

    /// Sets the render mode using the builder pattern.
    pub fn render_mode(mut self, render_mode: RenderMode) -> Self {
        self.render_mode = render_mode;
        self
    }

    /// Sets the sample mode using the builder pattern.
    pub fn sample_mode(mut self, sample_mode: SampleMode) -> Self {
        self.sample_mode = sample_mode;
        self
    }

    /// Sets the background color using the builder pattern.
    pub fn background(mut self, background: Pixel) -> Self {
        self.background_color = Some(background);
        self
    }

    /// Sets the ambient color using the builder pattern.
    pub fn ambient(mut self, ambient: Vec4<f32>) -> Self {
        self.ambient_color = Some(ambient);
        self
    }

    /// Sets the time
    pub fn time(mut self, time: f32) -> Self {
        self.time = time;
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
        assets: &Assets,
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
            self.width,
            self.height,
        );

        // We append the in-scope chunk lights to the dynamic lights
        for chunk in scene.chunks.values() {
            for light in &chunk.lights {
                scene.dynamic_lights.push(light.clone());
            }
        }

        // We collect the nodes for dynamic hit and miss post processing
        // from the terminals of the render node.
        self.render_hit = self.render_graph.collect_nodes_from(0, 0);
        self.render_miss = self.render_graph.collect_nodes_from(0, 1);

        // Precompute hit node values
        for node in &mut self.render_hit {
            self.render_graph.nodes[*node as usize].render_setup(self.hour);
        }

        // Precompute missed node values
        for node in &mut self.render_miss {
            if let Some((sun_dir, day_factor)) =
                self.render_graph.nodes[*node as usize].render_setup(self.hour)
            {
                self.sun_dir = Some(sun_dir);
                self.day_factor = day_factor;
            }
        }

        // Render a node based ambient color (procedural Sky) or if not
        // available use the ambient color (if any)
        for node in &mut self.render_miss {
            if let Some(ambient) =
                self.render_graph.nodes[*node as usize].render_ambient_color(self.hour)
            {
                self.ambient_color = Some(ambient);
            }
        }

        // Divide the screen into tiles (pre-reserve to avoid reallocations)
        let tiles_x = (width + tile_size - 1) / tile_size;
        let tiles_y = (height + tile_size - 1) / tile_size;
        let mut tiles = Vec::with_capacity(tiles_x * tiles_y);
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

                if !self.render_mode.ignore_background_shader {
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
                }

                let mut execution = Execution::new(0);

                if self.render_mode.supports3d() {
                    // Chunks
                    for chunk in scene.chunks.values() {
                        for batch3d in &chunk.batches3d {
                            self.d3_rasterize(
                                &mut buffer,
                                &mut z_buffer,
                                tile,
                                batch3d,
                                scene,
                                assets,
                                Some(chunk),
                                &mut execution,
                            );
                        }
                        if let Some(terrain_chunk) = &chunk.terrain_batch3d {
                            self.d3_rasterize(
                                &mut buffer,
                                &mut z_buffer,
                                tile,
                                terrain_chunk,
                                scene,
                                assets,
                                Some(chunk),
                                &mut execution,
                            );
                        }
                    }

                    // Static
                    for batch in scene.d3_static.iter() {
                        self.d3_rasterize(
                            &mut buffer,
                            &mut z_buffer,
                            tile,
                            batch,
                            scene,
                            assets,
                            None,
                            &mut execution,
                        );
                    }

                    // Dynamic
                    for batch in scene.d3_dynamic.iter() {
                        self.d3_rasterize(
                            &mut buffer,
                            &mut z_buffer,
                            tile,
                            batch,
                            scene,
                            assets,
                            None,
                            &mut execution,
                        );
                    }

                    // Call post-processing for missed geometry hits
                    if !self.render_miss.is_empty() || self.brush_preview.is_some() {
                        for ty in 0..tile.height {
                            for tx in 0..tile.width {
                                let uv = Vec2::new(
                                    (tile.x + tx) as f32 / self.width,
                                    (tile.y + ty) as f32 / self.height,
                                );
                                let z_idx = ty * tile.width + tx;
                                if z_buffer[z_idx] == 1.0 {
                                    let mut color = Vec4::new(0.0, 0.0, 0.0, 1.0);
                                    let ray =
                                        self.screen_ray((tile.x + tx) as f32, (tile.y + ty) as f32);
                                    for node in &self.render_miss {
                                        self.render_graph.nodes[*node as usize].render_miss_d3(
                                            &mut color,
                                            &self.camera_pos,
                                            &ray,
                                            &uv,
                                            self.hour,
                                        );
                                    }

                                    // Brush preview
                                    if let Some(brush_preview) = &self.brush_preview {
                                        if ray.dir.y.abs() > 1e-5 {
                                            // Intersect with y=0 plane
                                            let t = -ray.origin.y / ray.dir.y;
                                            if t > 0.0 {
                                                let world = ray.origin + ray.dir * t;
                                                let dist =
                                                    (world - brush_preview.position).magnitude();
                                                if dist < brush_preview.radius {
                                                    let normalized = dist / brush_preview.radius;
                                                    let falloff =
                                                        brush_preview.falloff.clamp(0.001, 1.0);
                                                    let fade = ((1.0 - normalized) / falloff)
                                                        .clamp(0.0, 1.0);

                                                    let blend = 0.2 + 0.6 * fade;

                                                    for i in 0..3 {
                                                        color[i] = (color[i] * (1.0 - blend)
                                                            + blend)
                                                            .min(1.0);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    let idx = (ty * tile.width + tx) * 4;
                                    buffer[idx..idx + 4].copy_from_slice(&vec4_to_pixel(&color));
                                }
                            }
                        }
                    }
                }

                if self.render_mode.supports2d() {
                    // Chunks
                    for chunk in scene.chunks.values() {
                        for batch2d in &chunk.batches2d {
                            self.d2_rasterize(
                                &mut buffer,
                                tile,
                                batch2d,
                                scene,
                                assets,
                                Some(chunk),
                                &mut execution,
                            );
                        }
                        if let Some(terrain_chunk) = &chunk.terrain_batch2d {
                            self.d2_rasterize(
                                &mut buffer,
                                tile,
                                terrain_chunk,
                                scene,
                                assets,
                                Some(chunk),
                                &mut execution,
                            );
                        }
                    }

                    // Static
                    for batch in scene.d2_static.iter() {
                        self.d2_rasterize(
                            &mut buffer,
                            tile,
                            batch,
                            scene,
                            assets,
                            None,
                            &mut execution,
                        );
                    }

                    // Dynamic
                    for batch in scene.d2_dynamic.iter() {
                        self.d2_rasterize(
                            &mut buffer,
                            tile,
                            batch,
                            scene,
                            assets,
                            None,
                            &mut execution,
                        );
                    }
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
        batch: &Batch2D,
        scene: &Scene,
        assets: &Assets,
        chunk: Option<&Chunk>,
        execution: &mut Execution,
    ) {
        if let Some(bbox) = batch.bounding_box {
            // Without padding horizontal lines may not be insde the BBox.
            let pad = 0.5;
            if bbox.x < (tile.x + tile.width) as f32 + pad
                && (bbox.x + bbox.width) > tile.x as f32 - pad
                && bbox.y < (tile.y + tile.height) as f32 + pad
                && (bbox.y + bbox.height) > tile.y as f32 - pad
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
                            let (min_xf, max_xf) = {
                                let ax = v0[0];
                                let bx = v1[0];
                                let cx = v2[0];
                                let minx = ax.min(bx.min(cx));
                                let maxx = ax.max(bx.max(cx));
                                (minx, maxx)
                            };
                            let (min_yf, max_yf) = {
                                let ay = v0[1];
                                let by = v1[1];
                                let cy = v2[1];
                                let miny = ay.min(by.min(cy));
                                let maxy = ay.max(by.max(cy));
                                (miny, maxy)
                            };
                            let min_x = min_xf.floor().max(tile.x as f32) as usize;
                            let max_x = max_xf.ceil().min((tile.x + tile.width) as f32) as usize;
                            let min_y = min_yf.floor().max(tile.y as f32) as usize;
                            let max_y = max_yf.ceil().min((tile.y + tile.height) as f32) as usize;

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

                                        // Calculate grid and world positions
                                        let grid_space_pos = Vec2::new(tx as f32, ty as f32)
                                            - Vec2::new(self.width, self.height) / 2.0
                                            - Vec2::new(
                                                self.translationd2.x - self.width / 2.0,
                                                self.translationd2.y - self.height / 2.0,
                                            );
                                        let world = grid_space_pos / self.scaled2;

                                        let mut texel = match batch.source {
                                            PixelSource::StaticTileIndex(index) => {
                                                if let Some(textile) =
                                                    &assets.tile_list.get(index as usize)
                                                {
                                                    let index = scene.animation_frame
                                                        % textile.textures.len();
                                                    textile.textures[index].sample(
                                                        u,
                                                        v,
                                                        self.sample_mode,
                                                        batch.repeat_mode,
                                                    )
                                                } else {
                                                    [0, 0, 0, 0]
                                                }
                                            }
                                            PixelSource::DynamicTileIndex(index) => {
                                                if let Some(textile) =
                                                    &scene.dynamic_textures.get(index as usize)
                                                {
                                                    let index = scene.animation_frame
                                                        % textile.textures.len();
                                                    textile.textures[index].sample(
                                                        u,
                                                        v,
                                                        self.sample_mode,
                                                        batch.repeat_mode,
                                                    )
                                                } else {
                                                    [0, 0, 0, 0]
                                                }
                                            }
                                            PixelSource::Pixel(col) => col,
                                            PixelSource::EntityTile(id, index) => {
                                                if let Some(entity_sequences) =
                                                    assets.entity_tiles.get(&id)
                                                {
                                                    if let Some(textile) =
                                                        entity_sequences.get_index(index as usize)
                                                    {
                                                        let index = scene.animation_frame
                                                            % textile.1.textures.len();
                                                        textile.1.textures[index].sample(
                                                            u,
                                                            v,
                                                            self.sample_mode,
                                                            batch.repeat_mode,
                                                        )
                                                    } else {
                                                        [0, 0, 0, 0]
                                                    }
                                                } else {
                                                    [0, 0, 0, 0]
                                                }
                                            }
                                            PixelSource::ItemTile(id, index) => {
                                                if let Some(item_sequences) =
                                                    assets.item_tiles.get(&id)
                                                {
                                                    if let Some(textile) =
                                                        item_sequences.get_index(index as usize)
                                                    {
                                                        let index = scene.animation_frame
                                                            % textile.1.textures.len();
                                                        textile.1.textures[index].sample(
                                                            u,
                                                            v,
                                                            self.sample_mode,
                                                            batch.repeat_mode,
                                                        )
                                                    } else {
                                                        [0, 0, 0, 0]
                                                    }
                                                } else {
                                                    [0, 0, 0, 0]
                                                }
                                            }
                                            PixelSource::Terrain => {
                                                if let Some(chunk) = chunk {
                                                    chunk.sample_terrain_texture(world, Vec2::one())
                                                } else {
                                                    [0, 0, 0, 0]
                                                }
                                            }
                                            _ => [0, 0, 0, 0],
                                        };

                                        // Execute the batch shader (if any)
                                        if let Some(shader_index) = batch.shader {
                                            let program = if let Some(chunk) = chunk {
                                                chunk.shaders.get(shader_index)
                                            } else {
                                                scene.shaders.get(shader_index)
                                            };

                                            if let Some(program) = program {
                                                if let Some(sh) = program.shade_index {
                                                    let mut color: Vec4<f32> =
                                                        pixel_to_vec4(&texel);

                                                    execution.uv.x = u;
                                                    execution.uv.y = v;
                                                    execution.color.x = color.x;
                                                    execution.color.y = color.y;
                                                    execution.color.z = color.z;
                                                    execution.hitpoint.x = world.x;
                                                    execution.hitpoint.y = world.y;
                                                    execution.time.x = self.time;
                                                    execution.time.y = self.time;
                                                    execution.time.z = self.time;

                                                    execution.roughness.x = 0.5;
                                                    execution.metallic.x = 0.0;

                                                    execution.reset(program.globals);
                                                    execution.shade(sh, program);

                                                    color.x = execution.color.x;
                                                    color.y = execution.color.y;
                                                    color.z = execution.color.z;
                                                    color.w = 1.0;
                                                    texel = vec4_to_pixel(&color);
                                                }
                                            }
                                        }

                                        if batch.receives_light
                                            && (!scene.lights.is_empty()
                                                || !scene.dynamic_lights.is_empty())
                                            || self.ambient_color.is_some()
                                        {
                                            let mut accumulated_light = [0.0, 0.0, 0.0];

                                            if let Some(ambient) = &self.ambient_color {
                                                let occlusion = self.mapmini.get_occlusion(world);
                                                accumulated_light[0] += ambient.x * occlusion;
                                                accumulated_light[1] += ambient.y * occlusion;
                                                accumulated_light[2] += ambient.z * occlusion;
                                            }

                                            for light in
                                                scene.lights.iter().chain(&scene.dynamic_lights)
                                            {
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
                                            if !self.preserve_transparency {
                                                buffer[idx + 3] = 255;
                                            } else {
                                                buffer[idx + 3] = texel[3];
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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    /// Rasterizes a 3D batch.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn d3_rasterize(
        &self,
        buffer: &mut [u8],
        z_buffer: &mut [f32],
        tile: &TileRect,
        batch: &Batch3D,
        scene: &Scene,
        assets: &Assets,
        chunk: Option<&Chunk>,
        execution: &mut Execution,
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
                            let (min_xf, max_xf) = {
                                let ax = v0[0];
                                let bx = v1[0];
                                let cx = v2[0];
                                let minx = ax.min(bx.min(cx));
                                let maxx = ax.max(bx.max(cx));
                                (minx, maxx)
                            };
                            let (min_yf, max_yf) = {
                                let ay = v0[1];
                                let by = v1[1];
                                let cy = v2[1];
                                let miny = ay.min(by.min(cy));
                                let maxy = ay.max(by.max(cy));
                                (miny, maxy)
                            };
                            let min_x = min_xf.floor().max(tile.x as f32) as usize;
                            let max_x = max_xf.ceil().min((tile.x + tile.width) as f32) as usize;
                            let min_y = min_yf.floor().max(tile.y as f32) as usize;
                            let max_y = max_yf.ceil().min((tile.y + tile.height) as f32) as usize;

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
                                            let world_2d = Vec2::new(world.x, world.z);

                                            let (mut texel, is_terrain) = match batch.source {
                                                PixelSource::StaticTileIndex(index) => {
                                                    let textile = &assets.tile_list[index as usize];
                                                    let index = scene.animation_frame
                                                        % textile.textures.len();
                                                    (
                                                        textile.textures[index].sample(
                                                            interpolated_u,
                                                            interpolated_v,
                                                            self.sample_mode,
                                                            batch.repeat_mode,
                                                        ),
                                                        false,
                                                    )
                                                }
                                                PixelSource::DynamicTileIndex(index) => {
                                                    let textile =
                                                        &scene.dynamic_textures[index as usize];
                                                    let index = scene.animation_frame
                                                        % textile.textures.len();
                                                    (
                                                        textile.textures[index].sample(
                                                            interpolated_u,
                                                            interpolated_v,
                                                            self.sample_mode,
                                                            batch.repeat_mode,
                                                        ),
                                                        false,
                                                    )
                                                }
                                                PixelSource::Pixel(col) => (col, false),
                                                PixelSource::EntityTile(id, index) => {
                                                    if let Some(entity_sequences) =
                                                        assets.entity_tiles.get(&id)
                                                    {
                                                        if let Some(textile) = entity_sequences
                                                            .get_index(index as usize)
                                                        {
                                                            let index = scene.animation_frame
                                                                % textile.1.textures.len();
                                                            (
                                                                textile.1.textures[index].sample(
                                                                    interpolated_u,
                                                                    interpolated_v,
                                                                    self.sample_mode,
                                                                    batch.repeat_mode,
                                                                ),
                                                                false,
                                                            )
                                                        } else {
                                                            ([0, 0, 0, 0], false)
                                                        }
                                                    } else {
                                                        ([0, 0, 0, 0], false)
                                                    }
                                                }
                                                PixelSource::ItemTile(id, index) => {
                                                    if let Some(item_sequences) =
                                                        assets.item_tiles.get(&id)
                                                    {
                                                        if let Some(textile) =
                                                            item_sequences.get_index(index as usize)
                                                        {
                                                            let index = scene.animation_frame
                                                                % textile.1.textures.len();
                                                            (
                                                                textile.1.textures[index].sample(
                                                                    interpolated_u,
                                                                    interpolated_v,
                                                                    self.sample_mode,
                                                                    batch.repeat_mode,
                                                                ),
                                                                false,
                                                            )
                                                        } else {
                                                            ([0, 0, 0, 0], false)
                                                        }
                                                    } else {
                                                        ([0, 0, 0, 0], false)
                                                    }
                                                }
                                                PixelSource::Terrain => {
                                                    if let Some(chunk) = chunk {
                                                        let mut texel = chunk
                                                            .sample_terrain_texture(
                                                                world_2d,
                                                                Vec2::one(),
                                                            );
                                                        if let Some(brush_preview) =
                                                            &self.brush_preview
                                                        {
                                                            let dist = (world
                                                                - brush_preview.position)
                                                                .magnitude();

                                                            if dist < brush_preview.radius {
                                                                let normalized =
                                                                    dist / brush_preview.radius;
                                                                let falloff = brush_preview
                                                                    .falloff
                                                                    .clamp(0.001, 1.0); // avoid divide-by-zero
                                                                let fade = ((1.0 - normalized)
                                                                    / falloff)
                                                                    .clamp(0.0, 1.0);

                                                                let blend = 0.2 + 0.6 * fade; // blend between 20% and 80% white

                                                                for channel in &mut texel[..3] {
                                                                    *channel = ((*channel as f32)
                                                                        * (1.0 - blend)
                                                                        + 255.0 * blend)
                                                                        .min(255.0)
                                                                        as u8;
                                                                }
                                                            }
                                                        }
                                                        (texel, true)
                                                    } else {
                                                        ([255, 0, 0, 255], false)
                                                    }
                                                }
                                                _ => ([0, 0, 0, 255], false),
                                            };

                                            let normal = if !batch.normals.is_empty() {
                                                let n0 = batch.clipped_normals[i0];
                                                let n1 = batch.clipped_normals[i1];
                                                let n2 = batch.clipped_normals[i2];

                                                let mut normal =
                                                    (n0 * alpha + n1 * beta + n2 * gamma)
                                                        .normalized();

                                                let view_dir =
                                                    (self.camera_pos - world).normalized();
                                                if normal.dot(view_dir) < 0.0 {
                                                    normal = -normal;
                                                }

                                                normal
                                            } else {
                                                Vec3::zero()
                                            };

                                            let mut color: Vec4<f32> = pixel_to_vec4(&texel);
                                            color.x = srgb_to_linear_fast(color.x);
                                            color.y = srgb_to_linear_fast(color.y);
                                            color.z = srgb_to_linear_fast(color.z);

                                            // Execute the batch shader (if any)
                                            if let Some(shader_index) = batch.shader {
                                                let program = if let Some(chunk) = chunk {
                                                    chunk.shaders.get(shader_index)
                                                } else {
                                                    scene.shaders.get(shader_index)
                                                };

                                                if let Some(program) = program {
                                                    if let Some(sh) = program.shade_index {
                                                        execution.normal = normal;
                                                        execution.uv.x = interpolated_u;
                                                        execution.uv.y = interpolated_v;
                                                        execution.color.x = color.x;
                                                        execution.color.y = color.y;
                                                        execution.color.z = color.z;
                                                        execution.hitpoint = world;
                                                        execution.time.x = self.time;
                                                        execution.time.y = self.time;
                                                        execution.time.z = self.time;

                                                        execution.roughness.x = 0.5;
                                                        execution.metallic.x = 0.0;

                                                        execution.reset(program.globals);
                                                        execution.shade(sh, program);
                                                    }
                                                }
                                            }

                                            let mat_base = execution.color;
                                            let mat_roughness =
                                                execution.roughness.x.clamp(0.0, 1.0);
                                            let mat_metallic = execution.metallic.x.clamp(0.0, 1.0);
                                            let mat_emissive = execution.emissive;

                                            let mut lit = Vec3::<f32>::zero();

                                            if is_terrain {
                                                // Terrain path: sky hemisphere + directional sun
                                                if let Some(sky) = &self.ambient_color {
                                                    let hemi = 0.5 * (normal.y + 1.0);
                                                    // ambient only affects diffuse path
                                                    let kd = mat_base
                                                        * (1.0 - mat_metallic)
                                                        * (1.0 - 0.04);
                                                    lit += sky.xyz() * kd * hemi;
                                                }

                                                if let Some(sun_dir) = self.sun_dir {
                                                    if self.day_factor > 0.0 {
                                                        // Sun is directional: light vector is opposite to sun_dir
                                                        let ldir = (-sun_dir).normalized();
                                                        let sun_radiance = Vec3::broadcast(
                                                            self.day_factor.max(0.0),
                                                        );
                                                        lit += self.shade_fast_brdf(
                                                            mat_base,
                                                            mat_roughness,
                                                            mat_metallic,
                                                            Vec3::zero(),
                                                            normal,
                                                            (self.camera_pos - world).normalized(),
                                                            ldir,
                                                            sun_radiance,
                                                        );
                                                    }
                                                }

                                                // (Optional) You can add local lights here too if desired.
                                            } else {
                                                // Non-terrain: batch ambient + all scene lights
                                                let hemi = 0.5 * (normal.y + 1.0);
                                                let kd =
                                                    mat_base * (1.0 - mat_metallic) * (1.0 - 0.04); // cheap F0 reduction
                                                lit += batch.ambient_color * kd * hemi;

                                                // Direct lights
                                                for light in
                                                    scene.lights.iter().chain(&scene.dynamic_lights)
                                                {
                                                    let Some(radiance) = light.radiance_at(
                                                        world,
                                                        None,
                                                        self.hash_anim,
                                                    ) else {
                                                        continue;
                                                    };
                                                    let ldir =
                                                        (light.position - world).normalized();

                                                    lit += self.shade_fast_brdf(
                                                        mat_base,
                                                        mat_roughness,
                                                        mat_metallic,
                                                        Vec3::zero(), // emissive added after loop for stability
                                                        normal,
                                                        (self.camera_pos - world).normalized(),
                                                        ldir,
                                                        radiance,
                                                    );
                                                }
                                            }

                                            // Add emissive unshadowed at the end
                                            lit += mat_emissive;

                                            // color.x = lit.x.powf(1.0 / 2.2);
                                            // color.y = lit.y.powf(1.0 / 2.2);
                                            // color.z = lit.z.powf(1.0 / 2.2);

                                            color.x = linear_to_srgb_fast(lit.x);
                                            color.y = linear_to_srgb_fast(lit.y);
                                            color.z = linear_to_srgb_fast(lit.z);
                                            texel = vec4_to_pixel(&color);

                                            // ---

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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
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
                                &if let PixelSource::Pixel(color) = &batch.source {
                                    *color
                                } else {
                                    crate::WHITE
                                },
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
        a: &[f32; 2],
        b: &[f32; 2],
        c: &[f32; 2],
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

    pub fn _sample_sky_debug(&self, ray_dir: Vec3<f32>) -> [u8; 4] {
        let up = ray_dir.normalized().y.clamp(-1.0, 1.0); // -1 = down, 1 = up

        // Map Y to gradient range
        let t = (up + 1.0) * 0.5;

        // Sky color: gradient from horizon to zenith
        let horizon = Vec3::new(0.8, 0.7, 0.6); // Light gray-orange near horizon
        let sky = Vec3::new(0.1, 0.4, 0.9); // Deep blue overhead

        let color = Vec3::lerp(horizon, sky, t);

        [
            (color.x * 255.0).clamp(0.0, 255.0) as u8,
            (color.y * 255.0).clamp(0.0, 255.0) as u8,
            (color.z * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ]
    }

    /// Computes a world-space ray from a screen-space pixel (x, y)
    pub fn screen_ray(&self, x: f32, y: f32) -> Ray {
        // Convert screen to normalized device coordinates
        let ndc_x = 2.0 * (x / self.width) - 1.0;
        let ndc_y = 1.0 - 2.0 * (y / self.height); // Flip Y

        // Near and far points in NDC space
        let ndc_near = Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let ndc_far = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        // Transform to view space
        let view_near = self.inverse_projection_matrix * ndc_near;
        let view_far = self.inverse_projection_matrix * ndc_far;

        let view_near = view_near / view_near.w;
        let view_far = view_far / view_far.w;

        // Transform to world space
        let world_near = self.inverse_view_matrix * view_near;
        let world_far = self.inverse_view_matrix * view_far;

        // Origin and direction
        let origin = Vec3::new(world_near.x, world_near.y, world_near.z);
        let target = Vec3::new(world_far.x, world_far.y, world_far.z);
        let dir = (target - origin).normalized();

        Ray::new(origin, dir)
    }

    // Fast, approximated PBR shading via blinn / phong

    #[inline(always)]
    fn roughness_to_shininess(&self, r: f32) -> f32 {
        // r in [0,1], perceptual roughness. Map to Blinn exponent (fast parametric fit).
        let a = (r * r).max(1e-4);
        (2.0 / a - 2.0).clamp(1.0, 2048.0)
    }

    #[inline(always)]
    fn schlick_fresnel(&self, f0: Vec3<f32>, cos_theta: f32) -> Vec3<f32> {
        // F = F0 + (1-F0)*(1-cos)^5
        let one_minus = 1.0 - cos_theta.clamp(0.0, 1.0);
        let x = one_minus * one_minus * one_minus * one_minus * one_minus;
        f0 + (Vec3::one() - f0) * x
    }

    #[inline(always)]
    fn max3(&self, v: Vec3<f32>) -> f32 {
        v.x.max(v.y.max(v.z))
    }

    #[inline(always)]
    fn blinn_phong_spec(&self, n: Vec3<f32>, l: Vec3<f32>, v: Vec3<f32>, shininess: f32) -> f32 {
        let h = (l + v).normalized();
        let n_dot_h = n.dot(h).max(0.0);
        // Use your fast pow approx to keep it cheap
        // pow32_fast approximates x^32; generalize with exp/log or use n^k fit:
        // We'll keep a standard powf for clarity; you can plug your approx if desired.
        n_dot_h.powf(shininess)
    }

    #[inline(always)]
    fn shade_fast_brdf(
        &self,
        base_color: Vec3<f32>, // linear
        roughness: f32,
        metallic: f32,
        emissive: Vec3<f32>, // linear, can be base_color * strength
        n: Vec3<f32>,
        v: Vec3<f32>,
        l: Vec3<f32>,
        light_radiance: Vec3<f32>, // linear radiance/intensity
    ) -> Vec3<f32> {
        let n_dot_l = n.dot(l).max(0.0);
        if n_dot_l <= 0.0 {
            return emissive;
        }

        // PBR-friendly params mapped to fast model
        let f0 = Vec3::lerp(Vec3::broadcast(0.04), base_color, metallic); // colored F0 for metals
        let mut kd = base_color * (1.0 - metallic); // no diffuse for metals

        // Cheap “energy-ish” conservation: reduce diffuse by specular strength
        kd *= 1.0 - self.max3(f0);

        // Blinn exponent from roughness
        let shininess = self.roughness_to_shininess(roughness);

        // Spec term
        let spec_b = self.blinn_phong_spec(n, l, v, shininess);

        // Fresnel at the (H·V) angle: approximate with N·V to save ops
        let n_dot_v = n.dot(v).max(0.0);
        let f = self.schlick_fresnel(f0, n_dot_v);

        // Final lobes
        let diffuse = kd * n_dot_l; // Lambert
        let specular = f * spec_b * n_dot_l; // colored specular

        // Lighted color
        (diffuse + specular) * light_radiance + emissive
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
