use crate::Terrain;
use crate::{Batch, PixelSource, Texture};
use theframework::prelude::*;
use vek::Vec2;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum TerrainBlendMode {
    None,
    Blend,                  // Normal blend (centered)
    BlendOffset(Vec2<f32>), // Blend with a fixed offset
    Custom(u8, Vec2<f32>),  // Custom ID and offset
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TerrainChunk {
    pub origin: Vec2<i32>,
    #[serde(with = "vectorize")]
    pub heights: FxHashMap<(i32, i32), f32>,
    #[serde(with = "vectorize")]
    pub sources: FxHashMap<(i32, i32), PixelSource>,
    #[serde(with = "vectorize")]
    pub blend_modes: FxHashMap<(i32, i32), TerrainBlendMode>,
    #[serde(skip, default)]
    pub batch: Option<Batch<[f32; 4]>>,
    #[serde(skip, default)]
    pub batch_d2: Option<Batch<[f32; 2]>>,
    #[serde(skip, default)]
    pub baked_texture: Option<Texture>,
    pub dirty: bool,
}

impl TerrainChunk {
    pub fn new(origin: Vec2<i32>) -> Self {
        Self {
            origin,
            heights: FxHashMap::default(),
            sources: FxHashMap::default(),
            blend_modes: FxHashMap::default(),
            batch: None,
            batch_d2: None,
            baked_texture: None,
            dirty: true,
        }
    }

    pub fn world_to_local(&self, world: Vec2<i32>) -> Vec2<i32> {
        world - self.origin
    }

    pub fn local_to_world(&self, local: Vec2<i32>) -> Vec2<i32> {
        local + self.origin
    }

    pub fn set_height(&mut self, x: i32, y: i32, value: f32) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights.insert((local.x, local.y), value);
        self.mark_dirty();
    }

    pub fn set_blend_mode(&mut self, x: i32, y: i32, mode: TerrainBlendMode) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.blend_modes.insert((local.x, local.y), mode);
        self.mark_dirty();
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights
            .get(&(local.x, local.y))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn set_source(&mut self, x: i32, y: i32, source: PixelSource) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.sources.insert((local.x, local.y), source);
        self.mark_dirty();
    }

    pub fn get_source(&self, x: i32, y: i32) -> Option<&PixelSource> {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.sources.get(&(local.x, local.y))
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn unload(&mut self) {
        self.batch = None;
        self.baked_texture = None;
    }

    pub fn bounds(&self) -> Option<(Vec2<i32>, Vec2<i32>)> {
        if self.heights.is_empty() {
            return None;
        }

        let mut min = Vec2::new(i32::MAX, i32::MAX);
        let mut max = Vec2::new(i32::MIN, i32::MIN);

        for &(x, y) in self.heights.keys() {
            let world = self.local_to_world(Vec2::new(x, y));
            min.x = min.x.min(world.x);
            min.y = min.y.min(world.y);
            max.x = max.x.max(world.x);
            max.y = max.y.max(world.y);
        }

        Some((min, max))
    }

    /// Returns true if the height exists at (x, y) in this chunk
    pub fn exists(&self, x: i32, y: i32) -> bool {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights.contains_key(&(local.x, local.y))
    }

    /// Rebuilds the renderable mesh batch for this chunk
    pub fn rebuild_batch(&mut self, terrain: &Terrain) {
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map = FxHashMap::default();

        for (&(lx, ly), &_) in &self.heights {
            let world_pos = self.local_to_world(Vec2::new(lx, ly));

            for (dx, dy) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                let px = world_pos.x + dx;
                let py = world_pos.y + dy;

                if vertex_map.contains_key(&(px, py)) {
                    continue;
                }

                let index = vertices.len();
                vertex_map.insert((px, py), index);

                vertices.push([
                    px as f32 * terrain.scale.x,
                    terrain.get_height(px, py),
                    py as f32 * terrain.scale.y,
                    1.0,
                ]);
                uvs.push([0.0, 0.0]);
            }

            let i0 = vertex_map[&(world_pos.x, world_pos.y)];
            let i1 = vertex_map[&(world_pos.x + 1, world_pos.y)];
            let i2 = vertex_map[&(world_pos.x, world_pos.y + 1)];
            let i3 = vertex_map[&(world_pos.x + 1, world_pos.y + 1)];

            indices.push((i0, i2, i1));
            indices.push((i1, i2, i3));
        }

        self.batch = Some(Batch::new_3d(vertices, indices, uvs));
        if let Some(batch) = &mut self.batch {
            batch.compute_vertex_normals();
        }
        self.dirty = false;
    }

    /// Rebuilds a simple 2D rectangle batch for this chunk
    pub fn rebuild_batch_d2(&mut self, terrain: &Terrain) {
        let min = self.origin;
        let max = self.origin + Vec2::new(terrain.chunk_size, terrain.chunk_size) - Vec2::new(1, 1);

        let min_pos = Vec2::new(
            min.x as f32 * terrain.scale.x,
            min.y as f32 * terrain.scale.y,
        );
        let max_pos = Vec2::new(
            (max.x + 1) as f32 * terrain.scale.x,
            (max.y + 1) as f32 * terrain.scale.y,
        );

        let width = max_pos.x - min_pos.x;
        let height = max_pos.y - min_pos.y;

        self.batch_d2 = Some(Batch::from_rectangle(min_pos.x, min_pos.y, width, height));
    }
}
