use crate::{Assets, Pixel, PixelSource, Ray, TerrainChunk, Texture};
use rayon::prelude::*;
use theframework::prelude::*;
use vek::Vec2;

const CHUNKSIZE: i32 = 8;

#[derive(Clone, Debug)]
pub struct TerrainHit {
    pub world_pos: Vec3<f32>,
    pub grid_pos: Vec2<i32>,
    pub height: f32,
}

pub mod chunk;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Terrain {
    pub scale: Vec2<f32>, // world units per tile
    pub chunk_size: i32,  // number of tiles per chunk
    #[serde(with = "vectorize")]
    pub chunks: FxHashMap<(i32, i32), TerrainChunk>, // (chunk_x, chunk_y) -> chunk
    pub baked_texture: Option<Texture>, // final baked texture
    pub bounds: Option<(Vec2<i32>, Vec2<i32>)>, // min/max world coords
}

impl Terrain {
    /// Creates an empty terrain instance
    pub fn empty() -> Self {
        Self {
            scale: Vec2::one(),
            chunk_size: CHUNKSIZE,
            chunks: FxHashMap::default(),
            baked_texture: None,
            bounds: None,
        }
    }

    /// Generate procedural rolling hills terrain for debugging
    pub fn generate(&mut self, size: i32) {
        let half = size / 2;

        for y in -half..=half {
            for x in -half..=half {
                let fx = x as f32 / size as f32;
                let fy = y as f32 / size as f32;

                // Radial hill + sine/cosine ripples
                let distance = (fx * fx + fy * fy).sqrt();
                let height = (1.0 - distance).max(0.0) * 5.0
                    + (fx * std::f32::consts::PI * 3.0).sin()
                    + (fy * std::f32::consts::PI * 2.0).cos();

                self.set_height(x, y, height);
            }
        }

        self.recompute_bounds();
    }

    /// Returns the coordinates for the chunk at the given world pos
    fn get_chunk_coords(&self, x: i32, y: i32) -> (i32, i32) {
        (x.div_euclid(self.chunk_size), y.div_euclid(self.chunk_size))
    }

    /// Gets the chunk at the given coords (or create it)
    fn get_or_create_chunk(&mut self, x: i32, y: i32) -> &mut TerrainChunk {
        let coords = self.get_chunk_coords(x, y);
        self.chunks
            .entry(coords)
            .or_insert_with(|| TerrainChunk::new(Vec2::new(coords.0, coords.1) * self.chunk_size))
    }

    /// Get height at given cell
    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        let chunk_coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_coords) {
            chunk.get_height(x, y)
        } else {
            0.0
        }
    }

    /// Set height at given cell
    pub fn set_height(&mut self, x: i32, y: i32, height: f32) {
        let chunk = self.get_or_create_chunk(x, y);
        chunk.set_height(x, y, height);
    }

    /// Set source material at given cell
    pub fn set_source(&mut self, x: i32, y: i32, source: PixelSource) {
        let chunk = self.get_or_create_chunk(x, y);
        chunk.set_source(x, y, source);
    }

    /// Get source material at given cell
    pub fn get_source(&self, x: i32, y: i32) -> Option<&PixelSource> {
        let chunk_coords = self.get_chunk_coords(x, y);
        self.chunks
            .get(&(chunk_coords.0, chunk_coords.1))
            .and_then(|chunk| chunk.get_source(x, y))
    }

    /// Sample height at a world position (nearest neighbor)
    pub fn sample_height(&self, x: f32, y: f32) -> f32 {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        self.get_height(xi, yi)
    }

    /// Bilinearly interpolate the height at a world position (x, y)
    pub fn sample_height_bilinear(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let h00 = self.get_height(x0, y0);
        let h10 = self.get_height(x1, y0);
        let h01 = self.get_height(x0, y1);
        let h11 = self.get_height(x1, y1);

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;

        h0 * (1.0 - ty) + h1 * ty
    }

    /// Sample the baked texture at the given world position
    pub fn sample_baked(&self, world_pos: Vec2<f32>) -> Pixel {
        let baked = match &self.baked_texture {
            Some(tex) => tex,
            None => return [255, 0, 255, 255], // Magenta if not baked
        };

        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => return [0, 0, 0, 255], // Black if no bounds
        };

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let rel = world_pos - world_min;

        if rel.x < 0.0 || rel.y < 0.0 || rel.x > world_size.x || rel.y > world_size.y {
            return [0, 0, 0, 255]; // Out of bounds
        }

        let uv = rel / world_size;
        let x = (uv.x * baked.width as f32)
            .floor()
            .clamp(0.0, baked.width as f32 - 1.0) as u32;
        let y = (uv.y * baked.height as f32)
            .floor()
            .clamp(0.0, baked.height as f32 - 1.0) as u32;

        baked.get_pixel(x, y)
    }

    /// Grow bounding box
    pub fn update_bounds(&mut self, x: i32, y: i32) {
        let bounds = self
            .bounds
            .get_or_insert((Vec2::new(x, y), Vec2::new(x, y)));
        bounds.0.x = bounds.0.x.min(x);
        bounds.0.y = bounds.0.y.min(y);
        bounds.1.x = bounds.1.x.max(x);
        bounds.1.y = bounds.1.y.max(y);
    }

    /// Computes the bounding box of the heightmap as (min, max) inclusive and stores internally
    pub fn recompute_bounds(&mut self) {
        let mut min = Vec2::new(i32::MAX, i32::MAX);
        let mut max = Vec2::new(i32::MIN, i32::MIN);

        for chunk in self.chunks.values() {
            let origin = chunk.origin;

            min.x = min.x.min(origin.x);
            min.y = min.y.min(origin.y);
            max.x = max.x.max(origin.x + self.chunk_size - 1);
            max.y = max.y.max(origin.y + self.chunk_size - 1);
        }

        if min.x <= max.x && min.y <= max.y {
            self.bounds = Some((min, max));
        } else {
            self.bounds = None;
        }
    }

    /// Sample the pixel source at the given world position
    pub fn sample_source(&self, world_pos: Vec2<f32>, assets: &Assets) -> (Pixel, bool) {
        // Map world position to tile grid position
        let x = (world_pos.x / self.scale.x).floor() as i32;
        let y = (world_pos.y / self.scale.y).floor() as i32;

        // Local UV inside the tile (0..1)
        let local_x = (world_pos.x / self.scale.x).fract();
        let local_y = (world_pos.y / self.scale.y).fract();
        let uv = Vec2::new(
            if local_x < 0.0 {
                local_x + 1.0
            } else {
                local_x
            },
            if local_y < 0.0 {
                local_y + 1.0
            } else {
                local_y
            },
        );

        if let Some(source) = self.get_source(x, y) {
            match source {
                PixelSource::TileId(id) => {
                    if let Some(tile) = assets.tiles.get(id) {
                        if let Some(texture) = tile.textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                PixelSource::MaterialId(id) => {
                    if let Some(material) = assets.materials.get(id) {
                        if let Some(texture) = material.textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                _ => {}
            }
        }

        // Checkerboard fallback based on tile position
        let checker = ((x & 1) ^ (y & 1)) == 0;
        if checker {
            ([160, 160, 160, 255], false) // light gray
        } else {
            ([80, 80, 80, 255], false) // dark gray
        }
    }

    /// Sample the pixel source at the given world position, with bilinear blending.
    pub fn sample_source_blended(&self, world_pos: Vec2<f32>, assets: &Assets) -> Pixel {
        let offsets = [
            Vec2::new(0.0, 0.0),
            Vec2::new(-self.scale.x, 0.0),
            Vec2::new(self.scale.x, 0.0),
            Vec2::new(0.0, -self.scale.y),
            Vec2::new(0.0, self.scale.y),
            Vec2::new(-self.scale.x, -self.scale.y),
            Vec2::new(self.scale.x, -self.scale.y),
            Vec2::new(-self.scale.x, self.scale.y),
            Vec2::new(self.scale.x, self.scale.y),
        ];

        let mut sum = Vec4::zero();
        let mut count = 0.0;

        for offset in &offsets {
            let pos = world_pos + *offset;
            let (pixel, valid) = self.sample_source(pos, assets);
            if valid {
                sum += Vec4::new(
                    pixel[0] as f32,
                    pixel[1] as f32,
                    pixel[2] as f32,
                    pixel[3] as f32,
                );
                count += 1.0;
            }
        }

        if count > 0.0 {
            let avg = sum / count;
            [
                avg.x.round() as u8,
                avg.y.round() as u8,
                avg.z.round() as u8,
                avg.w.round() as u8,
            ]
        } else {
            // If no real source was found, fallback exactly like sample_source would
            let x = (world_pos.x / self.scale.x).floor() as i32;
            let y = (world_pos.y / self.scale.y).floor() as i32;

            let checker = ((x & 1) ^ (y & 1)) == 0;
            if checker {
                [160, 160, 160, 255]
            } else {
                [80, 80, 80, 255]
            }
        }
    }

    /// Bake all individual chunks
    pub fn bake_chunks(&mut self, assets: &Assets, pixels_per_tile: i32) {
        if self.bounds.is_none() {
            return;
        }

        let bounds_min = self.bounds.unwrap().0;

        let baked_chunks: Vec<_> = self
            .chunks
            .par_iter()
            .map(|(coords, _)| {
                let c = Vec2::new(coords.0, coords.1);
                let baked_texture = self.bake_chunk(&c, bounds_min, assets, pixels_per_tile);
                (*coords, baked_texture)
            })
            .collect();

        for (coords, texture) in baked_chunks {
            if let Some(chunk) = self.chunks.get_mut(&coords) {
                chunk.baked_texture = Some(texture);
                chunk.dirty = false;
            }
        }
    }

    /// Bake an individual chunk
    pub fn bake_chunk(
        &self,
        chunk_coords: &Vec2<i32>,
        _bounds_min: Vec2<i32>, // <-- NOT USED INSIDE bake_chunk!
        assets: &Assets,
        pixels_per_tile: i32,
    ) -> Texture {
        let chunk_min_tile = *chunk_coords * self.chunk_size;

        let chunk_tex_width = self.chunk_size * pixels_per_tile;
        let chunk_tex_height = self.chunk_size * pixels_per_tile;

        let mut pixels = vec![0u8; (chunk_tex_width * chunk_tex_height * 4) as usize];

        pixels
            .par_chunks_exact_mut((chunk_tex_width * 4) as usize)
            .enumerate()
            .for_each(|(y, line)| {
                for (x, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let world_x = (chunk_min_tile.x as f32 + x as f32 / pixels_per_tile as f32)
                        * self.scale.x;
                    let world_y = (chunk_min_tile.y as f32 + y as f32 / pixels_per_tile as f32)
                        * self.scale.y;

                    let world_pos = Vec2::new(world_x, world_y);
                    let color = self.sample_source_blended(world_pos, assets);

                    pixel.copy_from_slice(&color);
                }
            });

        Texture::new(pixels, chunk_tex_width as usize, chunk_tex_height as usize)
    }

    /// Stitch all baked chunks back together
    pub fn stitch_baked_chunks(&mut self, pixels_per_tile: i32) {
        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => {
                println!("[stitch] No bounds found!");
                return;
            }
        };

        let tile_width = max.x - min.x + 1;
        let tile_height = max.y - min.y + 1;
        let tex_width = tile_width * pixels_per_tile;
        let tex_height = tile_height * pixels_per_tile;

        let mut global_pixels = vec![0u8; (tex_width * tex_height * 4) as usize];

        for chunk in self.chunks.values() {
            if let Some(texture) = &chunk.baked_texture {
                // Compute the chunk's tile position relative to the full terrain min
                let local_tile_pos = chunk.origin - min;

                // Pixel position inside the global texture
                let chunk_pixel_min_x = local_tile_pos.x * pixels_per_tile;
                let chunk_pixel_min_y = local_tile_pos.y * pixels_per_tile;

                let chunk_tex_width = self.chunk_size * pixels_per_tile;
                let chunk_tex_height = self.chunk_size * pixels_per_tile;

                for y in 0..chunk_tex_height {
                    for x in 0..chunk_tex_width {
                        let src_idx = (y as usize * chunk_tex_width as usize + x as usize) * 4;

                        let global_x = chunk_pixel_min_x + x;
                        let global_y = chunk_pixel_min_y + y;

                        if global_x >= 0
                            && global_y >= 0
                            && global_x < tex_width
                            && global_y < tex_height
                        {
                            let dst_idx =
                                (global_y as usize * tex_width as usize + global_x as usize) * 4;
                            global_pixels[dst_idx..dst_idx + 4]
                                .copy_from_slice(&texture.data[src_idx..src_idx + 4]);
                        }
                    }
                }
            }
        }

        self.baked_texture = Some(Texture::new(
            global_pixels,
            tex_width as usize,
            tex_height as usize,
        ));
    }

    /// Bake the full world texture by first computing all individual batch textures and than stitching them together
    pub fn bake_texture(&mut self, assets: &Assets, pixels_per_tile: i32) {
        self.bake_chunks(assets, pixels_per_tile);
        self.stitch_baked_chunks(pixels_per_tile);
    }

    pub fn bake_texture_old_but_working(&mut self, assets: &Assets, pixels_per_tile: i32) {
        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => return,
        };

        let tile_width = max.x - min.x + 1;
        let tile_height = max.y - min.y + 1;

        let tex_size = Vec2::new(tile_width * pixels_per_tile, tile_height * pixels_per_tile);

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let mut pixels = vec![0u8; (tex_size.x * tex_size.y * 4) as usize];

        pixels
            .par_chunks_exact_mut(4)
            .enumerate()
            .for_each(|(i, pixel)| {
                let x = (i % tex_size.x as usize) as f32;
                let y = (i / tex_size.x as usize) as f32;

                let uv = Vec2::new(x / tex_size.x as f32, y / tex_size.y as f32);
                let world_pos = world_min + uv * world_size;
                let color = self.sample_source_blended(world_pos, assets);

                pixel.copy_from_slice(&color);
            });

        self.baked_texture = Some(Texture::new(
            pixels,
            tex_size.x as usize,
            tex_size.y as usize,
        ));
    }

    /// Iterate over all chunks and rebuild if dirty
    pub fn build_all_chunks(&mut self) {
        let mut dirty_coords = Vec::new();

        // for chunk in self.chunks.values_mut() {
        //     if chunk.dirty {
        //         chunk.rebuild_batch(self);
        //         chunk.clear_dirty();
        //     }
        // }

        for ((cx, cy), chunk) in &self.chunks {
            if chunk.dirty {
                dirty_coords.push(Vec2::new(*cx, *cy));
            }
        }

        for coords in dirty_coords {
            let chunk_ptr =
                self.chunks.get_mut(&(coords.x, coords.y)).unwrap() as *mut TerrainChunk;

            // Unsafe cannot be avoided here
            unsafe {
                let chunk = &mut *chunk_ptr;
                chunk.rebuild_batch(self);
                chunk.clear_dirty();
            }
        }
    }

    /// Ray / terrain hit used for editing
    pub fn ray_terrain_hit(&self, ray: &Ray, max_distance: f32) -> Option<TerrainHit> {
        let mut t = 0.0;
        for _ in 0..150 {
            let point = ray.origin + ray.dir * t;
            let world_pos = Vec2::new(point.x, point.z);
            let terrain_height = self.sample_height(world_pos.x, world_pos.y);

            let d = point.y - terrain_height;

            if d.abs() < 0.0001 {
                let grid_x = (point.x / self.scale.x).floor() as i32;
                let grid_y = (point.z / self.scale.y).floor() as i32;
                return Some(TerrainHit {
                    world_pos: Vec3::new(point.x, terrain_height, point.z),
                    grid_pos: Vec2::new(grid_x, grid_y),
                    height: terrain_height,
                });
            }

            t += d * 0.5;
            if t > max_distance {
                break;
            }
        }

        None
    }

    /// Returns true if a height value exists at (x, y)
    pub fn exists(&self, x: i32, y: i32) -> bool {
        let chunk_coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_coords) {
            chunk.exists(x, y)
        } else {
            false
        }
    }

    /// Convert world coordinates to tex coordinates.
    pub fn world_to_texcoord(&self, world_pos: Vec2<f32>) -> Option<Vec2<i32>> {
        let (min, max) = self.bounds?;
        let baked = self.baked_texture.as_ref()?;

        let tex_size = Vec2::new(baked.width as i32, baked.height as i32);

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let rel = world_pos - world_min;
        if rel.x < 0.0 || rel.y < 0.0 || rel.x > world_size.x || rel.y > world_size.y {
            return None;
        }

        let uv = rel / world_size;
        let x = (uv.x * tex_size.x as f32)
            .floor()
            .clamp(0.0, tex_size.x as f32 - 1.0) as i32;
        let y = (uv.y * tex_size.y as f32)
            .floor()
            .clamp(0.0, tex_size.y as f32 - 1.0) as i32;

        Some(Vec2::new(x, y))
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::empty()
    }
}

/*
#[derive(Clone, Debug)]
pub struct TerrainHit {
    pub world_pos: Vec3<f32>,
    pub grid_pos: Vec2<i32>,
    pub height: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Terrain {
    pub scale: Vec2<f32>, // world units per cell
    #[serde(with = "vectorize")]
    pub heights: FxHashMap<(i32, i32), f32>,
    #[serde(with = "vectorize")]
    pub sources: FxHashMap<(i32, i32), PixelSource>,
    pub baked_texture: Option<Texture>,
    pub bounds: Option<(Vec2<i32>, Vec2<i32>)>, // (min, max)
}

impl Terrain {
    /// Create a new, empty heightmap with given scale
    pub fn new(scale: Vec2<f32>) -> Self {
        Self {
            scale,
            heights: FxHashMap::default(),
            sources: FxHashMap::default(),
            baked_texture: None,
            bounds: None,
        }
    }

    /// Flat 1x1 map by default
    pub fn empty() -> Self {
        Self::new(Vec2::one())
    }

    /// Create a procedural terrain for testing: rolling hills using sine/cosine waves
    pub fn generate(size: i32, scale: Vec2<f32>) -> Self {
        let mut map = Terrain {
            scale,
            heights: FxHashMap::default(),
            sources: FxHashMap::default(),
            baked_texture: None,
            bounds: None,
        };

        let half = size / 2;

        for y in -half..=half {
            for x in -half..=half {
                let fx = x as f32 / size as f32;
                let fy = y as f32 / size as f32;

                // Simple radial hill + sine waves
                let distance = (fx * fx + fy * fy).sqrt();
                let height = (1.0 - distance).max(0.0) * 5.0
                    + (fx * std::f32::consts::PI * 3.0).sin()
                    + (fy * std::f32::consts::PI * 2.0).cos();

                map.set_height(x, y, height);
            }
        }

        map
    }

    /// Get height at given cell
    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        self.heights.get(&(x, y)).copied().unwrap_or(0.0)
    }

    /// Set height at given cell
    pub fn set_height(&mut self, x: i32, y: i32, value: f32) {
        self.heights.insert((x, y), value);

        let bounds = self
            .bounds
            .get_or_insert((Vec2::new(x, y), Vec2::new(x, y)));
        if x < bounds.0.x {
            bounds.0.x = x;
        }
        if y < bounds.0.y {
            bounds.0.y = y;
        }
        if x > bounds.1.x {
            bounds.1.x = x;
        }
        if y > bounds.1.y {
            bounds.1.y = y;
        }
    }

    /// Get source material at given cell
    pub fn get_source(&self, x: i32, y: i32) -> Option<&PixelSource> {
        self.sources.get(&(x, y))
    }

    /// Set source material at given cell
    pub fn set_source(&mut self, x: i32, y: i32, source: PixelSource) {
        self.sources.insert((x, y), source);
    }

    /// Sample height at a world position (nearest neighbor)
    pub fn sample_height(&self, x: f32, y: f32) -> f32 {
        let x = (x / self.scale.x).floor() as i32;
        let y = (y / self.scale.y).floor() as i32;
        self.get_height(x, y)
    }

    /// Bilinearly interpolates the height at fractional grid coordinates (x, y)
    pub fn sample_height_bilinear(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let h00 = self.get_height(x0, y0);
        let h10 = self.get_height(x1, y0);
        let h01 = self.get_height(x0, y1);
        let h11 = self.get_height(x1, y1);

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;

        h0 * (1.0 - ty) + h1 * ty
    }

    /// Sample the pixel source at the given world position
    pub fn sample_source(&self, world_pos: Vec2<f32>, assets: &Assets) -> (Pixel, bool) {
        // Map world position to tile grid position
        let x = (world_pos.x / self.scale.x).floor() as i32;
        let y = (world_pos.y / self.scale.y).floor() as i32;

        // Local UV inside the tile (0..1)
        let local_x = (world_pos.x / self.scale.x).fract();
        let local_y = (world_pos.y / self.scale.y).fract();
        let uv = Vec2::new(
            if local_x < 0.0 {
                local_x + 1.0
            } else {
                local_x
            },
            if local_y < 0.0 {
                local_y + 1.0
            } else {
                local_y
            },
        );

        if let Some(source) = self.get_source(x, y) {
            match source {
                PixelSource::TileId(id) => {
                    if let Some(tile) = assets.tiles.get(id) {
                        if let Some(texture) = tile.textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                PixelSource::MaterialId(id) => {
                    if let Some(material) = assets.materials.get(id) {
                        if let Some(texture) = material.textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                _ => {}
            }
        }

        // Checkerboard fallback based on tile position
        let checker = ((x & 1) ^ (y & 1)) == 0;
        if checker {
            ([160, 160, 160, 255], false) // light gray
        } else {
            ([80, 80, 80, 255], false) // dark gray
        }
    }

    /// Sample the pixel source at the given world position, with bilinear blending.
    pub fn sample_source_blended(&self, world_pos: Vec2<f32>, assets: &Assets) -> Pixel {
        let offsets = [
            Vec2::new(0.0, 0.0),
            Vec2::new(-self.scale.x, 0.0),
            Vec2::new(self.scale.x, 0.0),
            Vec2::new(0.0, -self.scale.y),
            Vec2::new(0.0, self.scale.y),
            Vec2::new(-self.scale.x, -self.scale.y),
            Vec2::new(self.scale.x, -self.scale.y),
            Vec2::new(-self.scale.x, self.scale.y),
            Vec2::new(self.scale.x, self.scale.y),
        ];

        let mut sum = Vec4::zero();
        let mut count = 0.0;

        for offset in &offsets {
            let pos = world_pos + *offset;
            let (pixel, valid) = self.sample_source(pos, assets);
            if valid {
                sum += Vec4::new(
                    pixel[0] as f32,
                    pixel[1] as f32,
                    pixel[2] as f32,
                    pixel[3] as f32,
                );
                count += 1.0;
            }
        }

        if count > 0.0 {
            let avg = sum / count;
            [
                avg.x.round() as u8,
                avg.y.round() as u8,
                avg.z.round() as u8,
                avg.w.round() as u8,
            ]
        } else {
            // If no real source was found, fallback exactly like sample_source would
            let x = (world_pos.x / self.scale.x).floor() as i32;
            let y = (world_pos.y / self.scale.y).floor() as i32;

            let checker = ((x & 1) ^ (y & 1)) == 0;
            if checker {
                [160, 160, 160, 255]
            } else {
                [80, 80, 80, 255]
            }
        }
    }

    /// Sample the baked terrain texture at the given world position.
    /// Returns the baked pixel, or a fallback if out of bounds or not baked.
    pub fn sample_baked(&self, world_pos: Vec2<f32>) -> Pixel {
        let (x, y) = match self.world_to_texcoord(world_pos) {
            Some(coord) => coord,
            None => return [0, 0, 0, 255], // black fallback if outside bounds
        };

        self.baked_texture
            .as_ref()
            .map(|t| t.get_pixel(x as u32, y as u32))
            .unwrap_or([255, 0, 255, 255]) // magenta fallback if not baked
    }

    /// Samples the steepness of the terrain at the given position.
    pub fn sample_steepness(&self, world_pos: Vec2<f32>) -> f32 {
        let eps = 0.5 * self.scale.x.min(self.scale.y);

        let x = world_pos.x;
        let y = world_pos.y;

        let h_center = self.sample_height(x, y);
        let h_x = self.sample_height(x + eps, y);
        let h_y = self.sample_height(x, y + eps);

        let dx = (h_x - h_center) / eps;
        let dy = (h_y - h_center) / eps;

        let gradient = Vec2::new(dx, dy);

        gradient.magnitude()
    }

    /// Approximate the normal at a world position by sampling neighboring heights
    pub fn sample_normal(&self, world_pos: Vec2<f32>) -> Vec3<f32> {
        const EPSILON: f32 = 0.5; // Fixed sampling distance

        let h_l = self.sample_height(world_pos.x - EPSILON, world_pos.y);
        let h_r = self.sample_height(world_pos.x + EPSILON, world_pos.y);
        let h_d = self.sample_height(world_pos.x, world_pos.y - EPSILON);
        let h_u = self.sample_height(world_pos.x, world_pos.y + EPSILON);

        Vec3::new(
            (h_l - h_r) * 0.5 / EPSILON,
            1.0,
            (h_d - h_u) * 0.5 / EPSILON,
        )
        .normalized()
    }

    /// Generate a batch for all filled cells (2 triangles per 1x1 quad)
    pub fn to_batch(&self) -> Batch<[f32; 4]> {
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Map from cell pos to index into vertices
        let mut vertex_map = FxHashMap::default();

        for &(x, y) in self.heights.keys() {
            // Add vertex for each corner
            for (dx, dy) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                let px = x + dx;
                let py = y + dy;

                if vertex_map.contains_key(&(px, py)) {
                    continue;
                }

                let wx = px as f32 * self.scale.x;
                let wy = py as f32 * self.scale.y;
                let hz = self.get_height(px, py);

                let index = vertices.len();
                vertex_map.insert((px, py), index);

                vertices.push([wx, hz, wy, 1.0]);
                uvs.push([0.0, 0.0]); // Placeholder — could be improved
            }

            // Get corner indices
            let i0 = vertex_map[&(x, y)];
            let i1 = vertex_map[&(x + 1, y)];
            let i2 = vertex_map[&(x, y + 1)];
            let i3 = vertex_map[&(x + 1, y + 1)];

            // Triangle 1
            indices.push((i0, i2, i1));
            // Triangle 2
            indices.push((i1, i2, i3));
        }

        let mut batch = Batch::new_3d(vertices, indices, uvs);
        batch.compute_vertex_normals();
        batch
    }

    /*
    /// Convert to triangle batch
    pub fn to_batch(&self) -> Batch<[f32; 4]> {
        let mut final_vertices = Vec::new();
        let mut final_uvs = Vec::new();
        let mut final_indices = Vec::new();
        let mut final_vertex_map = FxHashMap::default();

        let results: Vec<_> = self
            .heights
            .par_iter()
            .map(|(&(x, y), &_height)| {
                let mut local_vertices = Vec::new();
                let mut local_uvs = Vec::new();
                let mut local_indices = Vec::new();
                let mut local_vertex_map = FxHashMap::default();

                for (dx, dy) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let px = x + dx;
                    let py = y + dy;

                    if local_vertex_map.contains_key(&(px, py)) {
                        continue;
                    }

                    let wx = px as f32 * self.scale.x;
                    let wy = py as f32 * self.scale.y;
                    let hz = self.get_height(px, py);

                    let index = local_vertices.len();
                    local_vertex_map.insert((px, py), index);

                    local_vertices.push([wx, hz, wy, 1.0]);
                    local_uvs.push([0.0, 0.0]);
                }

                let i0 = local_vertex_map[&(x, y)];
                let i1 = local_vertex_map[&(x + 1, y)];
                let i2 = local_vertex_map[&(x, y + 1)];
                let i3 = local_vertex_map[&(x + 1, y + 1)];

                local_indices.push((i0, i2, i1));
                local_indices.push((i1, i2, i3));

                (local_vertices, local_uvs, local_indices, local_vertex_map)
            })
            .collect();

        // Merge results
        for (vertices, uvs, indices, vertex_map) in results {
            let base_index = final_vertices.len();

            final_vertices.extend(vertices);
            final_uvs.extend(uvs);

            for (a, b, c) in indices {
                final_indices.push((a + base_index, b + base_index, c + base_index));
            }

            for ((px, py), idx) in vertex_map {
                final_vertex_map.insert((px, py), idx + base_index);
            }
        }

        let mut batch = Batch::new_3d(final_vertices, final_indices, final_uvs);
        batch.compute_vertex_normals();
        batch
    }*/

    /// Convert world coordinates to tex coordinates.
    pub fn world_to_texcoord(&self, world_pos: Vec2<f32>) -> Option<(i32, i32)> {
        let (min, max) = self.bounds?;
        let baked = self.baked_texture.as_ref()?;

        let tex_size = Vec2::new(baked.width as i32, baked.height as i32);

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let rel = world_pos - world_min;
        if rel.x < 0.0 || rel.y < 0.0 || rel.x > world_size.x || rel.y > world_size.y {
            return None;
        }

        let uv = rel / world_size;
        let x = (uv.x * tex_size.x as f32)
            .floor()
            .clamp(0.0, tex_size.x as f32 - 1.0) as i32;
        let y = (uv.y * tex_size.y as f32)
            .floor()
            .clamp(0.0, tex_size.y as f32 - 1.0) as i32;

        Some((x, y))
    }

    /// Generate a smoothly interpolated mesh with `subdiv` subdivisions per cell
    pub fn to_batch_bilinear(&mut self, subdiv: u32) -> Batch<[f32; 4]> {
        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => return Batch::emptyd3(),
        };

        let width = (max.x - min.x) as u32;
        let height = (max.y - min.y) as u32;

        let vx = width * subdiv;
        let vy = height * subdiv;

        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Generate vertex grid
        for y in 0..=vy {
            for x in 0..=vx {
                let gx = x as f32 / subdiv as f32 + min.x as f32;
                let gy = y as f32 / subdiv as f32 + min.y as f32;

                let world_x = gx * self.scale.x;
                let world_y = gy * self.scale.y;
                let h = self.sample_height_bilinear(gx, gy);

                vertices.push([world_x, h, world_y, 1.0]);
                uvs.push([
                    (gx - min.x as f32) / width as f32,
                    (gy - min.y as f32) / height as f32,
                ]);
            }
        }

        let columns = vx + 1;

        // Generate indices
        for y in 0..vy {
            for x in 0..vx {
                let i0 = (y * columns + x) as usize;
                let i1 = i0 + 1;
                let i2 = i0 + columns as usize;
                let i3 = i2 + 1;

                indices.push((i0, i2, i1));
                indices.push((i1, i2, i3));
            }
        }

        let mut batch = Batch::new_3d(vertices, indices, uvs);
        batch.compute_vertex_normals();
        batch
    }

    /*
    /// Bake the texture
    pub fn bake_texture(&mut self, assets: &Assets, pixels_per_tile: i32) {
        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => return,
        };

        let tile_width = max.x - min.x + 1;
        let tile_height = max.y - min.y + 1;

        let tex_size = Vec2::new(tile_width * pixels_per_tile, tile_height * pixels_per_tile);

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let mut pixels = vec![0u8; (tex_size.x * tex_size.y * 4) as usize];

        for y in 0..tex_size.y {
            for x in 0..tex_size.x {
                let uv = Vec2::new(x as f32 / tex_size.x as f32, y as f32 / tex_size.y as f32);

                let world_pos = world_min + uv * world_size;
                let pixel = self.sample_source(world_pos, assets);

                let index = ((y * tex_size.x + x) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&pixel);
            }
        }

        self.baked_texture = Some(Texture::new(
            pixels,
            tex_size.x as usize,
            tex_size.y as usize,
        ));
    }*/

    pub fn bake_texture(&mut self, assets: &Assets, pixels_per_tile: i32) {
        let (min, max) = match self.bounds {
            Some(bounds) => bounds,
            None => return,
        };

        let tile_width = max.x - min.x + 1;
        let tile_height = max.y - min.y + 1;

        let tex_size = Vec2::new(tile_width * pixels_per_tile, tile_height * pixels_per_tile);

        let world_min = Vec2::new(min.x as f32 * self.scale.x, min.y as f32 * self.scale.y);
        let world_max = Vec2::new(
            (max.x + 1) as f32 * self.scale.x,
            (max.y + 1) as f32 * self.scale.y,
        );
        let world_size = world_max - world_min;

        let mut pixels = vec![0u8; (tex_size.x * tex_size.y * 4) as usize];

        pixels
            .par_chunks_exact_mut(4)
            .enumerate()
            .for_each(|(i, pixel)| {
                let x = (i % tex_size.x as usize) as f32;
                let y = (i / tex_size.x as usize) as f32;

                let uv = Vec2::new(x / tex_size.x as f32, y / tex_size.y as f32);
                let world_pos = world_min + uv * world_size;
                let color = self.sample_source_blended(world_pos, assets);

                pixel.copy_from_slice(&color);
            });

        self.baked_texture = Some(Texture::new(
            pixels,
            tex_size.x as usize,
            tex_size.y as usize,
        ));
    }

    /// Ray / terrain hit used for editing
    pub fn ray_terrain_hit(&self, ray: &Ray, max_distance: f32) -> Option<TerrainHit> {
        let mut t = 0.0;
        for _ in 0..150 {
            let point = ray.origin + ray.dir * t;
            let world_pos = Vec2::new(point.x, point.z);
            let terrain_height = self.sample_height(world_pos.x, world_pos.y);

            let d = point.y - terrain_height;

            if d.abs() < 0.0001 {
                let grid_x = (point.x / self.scale.x).floor() as i32;
                let grid_y = (point.z / self.scale.y).floor() as i32;
                return Some(TerrainHit {
                    world_pos: Vec3::new(point.x, terrain_height, point.z),
                    grid_pos: Vec2::new(grid_x, grid_y),
                    height: terrain_height,
                });
            }

            t += d * 0.5;
            if t > max_distance {
                break;
            }
        }

        None
    }

    /// Computes the bounding box of the heightmap as (min, max) inclusive and stores internally
    pub fn recompute_bounds(&mut self) {
        self.bounds = if self.heights.is_empty() {
            None
        } else {
            let mut min = Vec2::new(i32::MAX, i32::MAX);
            let mut max = Vec2::new(i32::MIN, i32::MIN);

            for &(x, y) in self.heights.keys() {
                min.x = min.x.min(x);
                min.y = min.y.min(y);
                max.x = max.x.max(x);
                max.y = max.y.max(y);
            }

            Some((min, max))
        };
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::empty()
    }
}
*/
