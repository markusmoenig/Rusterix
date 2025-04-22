use crate::SampleMode;
use crate::{
    Batch, CompiledLight, D3Camera, HitInfo, Pixel, Ray, Scene, pixel_to_vec4, vec4_to_pixel,
};
use SampleMode::*;
use bvh::aabb::Aabb;
use bvh::aabb::Bounded;
use bvh::ray::Ray as BvhRay;
use rand::Rng;
use rayon::prelude::*;
use vek::{Vec2, Vec4};

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
    pub fn trace(
        &mut self,
        camera: &dyn D3Camera,
        scene: &mut Scene,
        pixels: &mut [u8],
        width: usize,
        height: usize,
        tile_size: usize,
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
                        let mut pixel: Vec4<f32> = Vec4::new(0.0, 0.0, 0.0, 0.0);
                        let uv = Vec2::new(
                            (tile.x + tx) as f32 / screen_size.x,
                            1.0 - (tile.y + ty) as f32 / screen_size.y,
                        );

                        let samples = 4;
                        for _ in 0..samples {
                            let jitter = Vec2::new(rng.random::<f32>(), rng.random::<f32>());

                            let ray = camera.create_ray(uv, screen_size, jitter);
                            let bvh_ray = BvhRay::new(
                                nalgebra::Point3::new(ray.origin.x, ray.origin.y, ray.origin.z),
                                nalgebra::Vector3::new(ray.dir.x, ray.dir.y, ray.dir.z),
                            );

                            let mut hitinfo = HitInfo::default();

                            for (i, batch) in scene.d3_static.iter().enumerate() {
                                if let Some(bbox) = self.static_bboxes.get(i) {
                                    if !bvh_ray.intersects_aabb(bbox) {
                                        continue;
                                    }
                                }

                                if let Some(mut hit) = batch.intersect(&ray, false) {
                                    if hit.t < hitinfo.t {
                                        pixel += self.render(&ray, scene, batch, &mut hit);
                                        hitinfo = hit;
                                    }
                                }
                            }

                            for (_i, batch) in scene.d3_dynamic.iter().enumerate() {
                                // if let Some(bbox) = self.static_bboxes.get(i) {
                                //     if !bvh_ray.intersects_aabb(bbox) {
                                //         continue;
                                //     }
                                // }

                                if let Some(mut hit) = batch.intersect(&ray, false) {
                                    if hit.t < hitinfo.t {
                                        pixel += self.render(&ray, scene, batch, &mut hit);
                                        hitinfo = hit;
                                    }
                                }
                            }
                        }
                        pixel /= samples as f32;

                        let idx = (ty * tile.width + tx) * 4;
                        buffer[idx..idx + 4].copy_from_slice(&vec4_to_pixel(&pixel));
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

    pub fn render(
        &self,
        ray: &Ray,
        scene: &Scene,
        batch: &Batch<[f32; 4]>,
        hit: &mut HitInfo,
    ) -> Vec4<f32> {
        let textile = &scene.textures[batch.texture_index];
        let index = scene.animation_frame % textile.textures.len();

        let mut texel = pixel_to_vec4(&textile.textures[index].sample(
            hit.uv.x,
            hit.uv.y,
            self.sample_mode,
            batch.repeat_mode,
        ));

        if texel[3] == 1.0 {
            let world = ray.origin + hit.t * ray.dir;
            let mut accumulated_light: [f32; 3] = [0.0, 0.0, 0.0];

            for light in &self.compiled_lights {
                if let Some(light_color) = light.radiance_at(world, hit.normal, self.hash_anim) {
                    accumulated_light[0] += light_color[0];
                    accumulated_light[1] += light_color[1];
                    accumulated_light[2] += light_color[2];
                }
            }

            texel[0] *= accumulated_light[0].clamp(0.0, 1.0);
            texel[1] *= accumulated_light[1].clamp(0.0, 1.0);
            texel[2] *= accumulated_light[2].clamp(0.0, 1.0);
        }

        texel
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
