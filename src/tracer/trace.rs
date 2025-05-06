use crate::SampleMode;
use crate::{
    Batch, CompiledLight, D3Camera, HitInfo, Pixel, Scene, ShapeFXGraph, pixel_to_vec4,
    vec4_to_pixel,
};
use SampleMode::*;
use bvh::aabb::Aabb;
use bvh::aabb::Bounded;
use bvh::ray::Ray as BvhRay;
use rand::Rng;
use rayon::prelude::*;
use vek::{Vec2, Vec3, Vec4};

pub struct Tracer {
    /// SampleMode, default is Nearest.
    pub sample_mode: SampleMode,

    /// Background color (Sky etc.)
    pub background_color: Option<[u8; 4]>,

    /// Hash for animation
    pub hash_anim: u32,

    /// The compliled lights in the scene.
    pub compiled_lights: Vec<CompiledLight>,

    /// Optional per-batch bounding boxes for fast culling
    pub static_bboxes: Vec<Aabb<f32, 3>>,

    /// The rendergraph
    pub render_graph: ShapeFXGraph,
    render_hit: Vec<u16>,
    render_miss: Vec<u16>,

    pub hour: f32,
}

impl Default for Tracer {
    fn default() -> Self {
        Tracer::new()
    }
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            sample_mode: Nearest,
            background_color: None,
            static_bboxes: vec![],
            compiled_lights: vec![],
            hash_anim: 0,

            render_graph: ShapeFXGraph::default(),
            render_hit: vec![],
            render_miss: vec![],
            hour: 12.0,
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

    /// Precomputes the bounding boxes of all static batches.
    pub fn compute_static_bboxes(&mut self, scene: &Scene) {
        self.static_bboxes.clear();

        for batch in &scene.d3_static {
            self.static_bboxes.push(batch.aabb());
        }
    }

    /// Path trace the scene.
    #[allow(clippy::too_many_arguments)]
    pub fn trace(
        &mut self,
        camera: &dyn D3Camera,
        scene: &mut Scene,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        tile_size: usize,
        accum: i32,
    ) {
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
        self.compiled_lights = scene.compile_lights(self.background_color);

        self.compute_static_bboxes(scene);

        self.render_hit = self.render_graph.collect_nodes_from(0, 0);
        self.render_miss = self.render_graph.collect_nodes_from(0, 1);

        // Precompute hit node values
        for node in &mut self.render_hit {
            self.render_graph.nodes[*node as usize].render_setup(self.hour);
        }

        // Precompute missed node values
        for node in &mut self.render_miss {
            self.render_graph.nodes[*node as usize].render_setup(self.hour);
        }

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

                let mut rng = rand::rng();
                for ty in 0..tile.height {
                    for tx in 0..tile.width {
                        let mut ret: Vec3<f32> = Vec3::zero();
                        let mut throughput: Vec3<f32> = Vec3::one();

                        let screen_uv = Vec2::new(
                            (tile.x + tx) as f32 / screen_size.x,
                            1.0 - (tile.y + ty) as f32 / screen_size.y,
                        );

                        let jitter = Vec2::new(rng.random::<f32>(), rng.random::<f32>());
                        let mut ray = camera.create_ray(screen_uv, screen_size, jitter);
                        let mut bvh_ray = BvhRay::new(
                            nalgebra::Point3::new(ray.origin.x, ray.origin.y, ray.origin.z),
                            nalgebra::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
                        );
                        let camera_pos = ray.origin;

                        let bounces = 8;
                        for _ in 0..bounces {
                            let mut hitinfo = HitInfo::default();

                            // Evaluate hit

                            for (i, batch) in scene.d3_static.iter().enumerate() {
                                if let Some(bbox) = self.static_bboxes.get(i) {
                                    if !bvh_ray.intersects_aabb(bbox) {
                                        continue;
                                    }
                                }

                                if let Some(mut hit) = batch.intersect(&ray, false) {
                                    if hit.t < hitinfo.t
                                        && self.evaluate_hit(scene, batch, &mut hit)
                                    {
                                        hitinfo = hit;
                                    }
                                }
                            }

                            // Bounce

                            if hitinfo.t < f32::MAX {
                                if let Some(normal) = hitinfo.normal {
                                    ray.origin = ray.at(hitinfo.t) + normal * 0.01;
                                    ray.dir =
                                        (normal + self.random_unit_vector(&mut rng)).normalized();
                                    bvh_ray = BvhRay::new(
                                        nalgebra::Point3::new(
                                            ray.origin.x,
                                            ray.origin.y,
                                            ray.origin.z,
                                        ),
                                        nalgebra::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
                                    );
                                    ret += hitinfo.emissive * throughput;
                                    throughput *= hitinfo.albedo;
                                } else {
                                    println!("no normal");
                                    break;
                                }
                            } else if !self.render_miss.is_empty() {
                                // Call post-processing for missed geometry hits
                                let mut color = Vec4::new(0.0, 0.0, 0.0, 1.0);
                                for node in &self.render_miss {
                                    self.render_graph.nodes[*node as usize].render_miss_d3(
                                        &mut color,
                                        &camera_pos,
                                        &ray,
                                        &screen_uv,
                                        self.hour,
                                    );
                                }
                                ret += Vec3::new(color.x, color.y, color.z) * throughput;
                                break;
                            }
                        }

                        // Get the prev pixel
                        let idx = (ty * tile.width + tx) * 4;
                        let global_x = tile.x + tx;
                        let global_y = tile.y + ty;
                        let global_idx = (global_y * width + global_x) * 4;

                        let prev = pixel_to_vec4(&[
                            pixels[global_idx],
                            pixels[global_idx + 1],
                            pixels[global_idx + 2],
                            255,
                        ]);
                        // Accumulation
                        let t = 1.0 / (accum as f32 + 1.0);
                        let blended = prev * (1.0 - t) + Vec4::new(ret.x, ret.y, ret.z, 1.0) * t;
                        //let gamma_corrected = blended.map(|v| v.powf(2.2));
                        buffer[idx..idx + 4].copy_from_slice(&vec4_to_pixel(&blended));
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

    pub fn evaluate_hit(&self, scene: &Scene, batch: &Batch<[f32; 4]>, hit: &mut HitInfo) -> bool {
        let textile = &scene.textures[batch.texture_index];
        let index = scene.animation_frame % textile.textures.len();

        let texel = pixel_to_vec4(&textile.textures[index].sample(
            hit.uv.x,
            hit.uv.y,
            self.sample_mode,
            batch.repeat_mode,
        ));

        if let Some(_material) = &batch.material {
            hit.emissive = Vec3::new(texel.x, texel.y, texel.z) * 5.0;
        }

        if texel[3] == 1.0 {
            hit.albedo = Vec3::new(texel.x, texel.y, texel.z);
            true
        } else {
            false
        }
    }

    pub fn random_unit_vector<R: Rng>(&self, rng: &mut R) -> Vec3<f32> {
        let z = rng.random::<f32>() * 2.0 - 1.0;
        let a = rng.random::<f32>() * std::f32::consts::TAU;
        let r = (1.0 - z * z).sqrt();
        let x = r * a.cos();
        let y = r * a.sin();
        Vec3::new(x, y, z)
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
