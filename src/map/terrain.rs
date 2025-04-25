use crate::{Assets, Batch, Pixel, PixelSource, Texture};
use theframework::prelude::*;
use vek::Vec2;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Terrain {
    pub scale: Vec2<f32>, // world units per cell
    pub heights: FxHashMap<(i32, i32), f32>,
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
    pub fn sample_height(&self, world_pos: Vec2<f32>) -> f32 {
        let x = (world_pos.x / self.scale.x).floor() as i32;
        let y = (world_pos.y / self.scale.y).floor() as i32;
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
    pub fn sample_source(&self, world_pos: Vec2<f32>, assets: &Assets) -> Pixel {
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
                            return texture.sample_nearest(uv.x, uv.y);
                        }
                    }
                }
                PixelSource::MaterialId(id) => {
                    if let Some(material) = assets.materials.get(id) {
                        if let Some(texture) = material.textures.first() {
                            return texture.sample_nearest(uv.x, uv.y);
                        }
                    }
                }
                _ => {}
            }
        }

        // Checkerboard fallback based on tile position
        let checker = ((x & 1) ^ (y & 1)) == 0;
        if checker {
            [160, 160, 160, 255] // light gray
        } else {
            [80, 80, 80, 255] // dark gray
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
