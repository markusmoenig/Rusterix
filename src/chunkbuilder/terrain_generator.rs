//! Global terrain generation system
//!
//! This module generates continuous terrain meshes across chunks based on
//! geometry control points:
//! - Vertices provide height data (z-coordinate)
//! - Linedefs provide features (roads, rivers, etc.)
//! - Sectors define terrain regions and exclusions (houses, interiors)
//!
//! The system generates a grid-based mesh with:
//! - Height interpolation from vertex control points
//! - Hole cutting for excluded sectors
//! - Deterministic edge matching between chunks
//! - Tile assignment from nearest geometry

use crate::{BBox, Chunk, Map};
use vek::{Vec2, Vec3};

/// Terrain generation settings
pub struct TerrainConfig {
    /// Subdivision level: 1 = one quad per world tile, 2 = 4 quads per tile, etc.
    pub subdivisions: u32,
    /// Power parameter for Inverse Distance Weighting (typically 2.0)
    pub idw_power: f32,
    /// Maximum distance for vertex influence (beyond this, influence is zero)
    pub max_influence_distance: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            subdivisions: 1, // 1 quad per world tile
            idw_power: 2.0,
            max_influence_distance: 50.0,
        }
    }
}

/// Generates terrain mesh for a chunk
pub struct TerrainGenerator {
    config: TerrainConfig,
}

impl TerrainGenerator {
    pub fn new(config: TerrainConfig) -> Self {
        Self { config }
    }

    /// Generate terrain mesh for the given chunk
    ///
    /// Returns a vector of meshes grouped by tile: (vertices, indices, UVs, vertex_id for tile lookup)
    pub fn generate(
        &self,
        map: &Map,
        chunk: &Chunk,
    ) -> Option<Vec<(Vec<Vec3<f32>>, Vec<u32>, Vec<[f32; 2]>, u32)>> {
        // 1. Collect control points from vertices within influence range
        let control_points = self.collect_control_points(map, &chunk.bbox);

        // 2. Identify sectors marked for terrain exclusion
        let excluded_sectors = self.collect_excluded_sectors(map, &chunk.bbox);

        // 3. Generate grid mesh
        let grid = self.generate_grid(&chunk.bbox);

        // 4. Interpolate heights at grid points
        // If no control points, all heights will be 0.0 (flat terrain)
        let heights = self.interpolate_heights(&grid, &control_points);

        // 5. Cut holes for excluded sectors and group by tile
        let meshes_by_tile =
            self.apply_exclusions_and_group(&grid, &heights, &excluded_sectors, map);

        // Only return None if there are no meshes
        if meshes_by_tile.is_empty() {
            return None;
        }

        Some(meshes_by_tile)
    }

    /// Collect height control points from vertices (position, height, vertex_id for tile lookup)
    fn collect_control_points(&self, map: &Map, bbox: &BBox) -> Vec<(Vec2<f32>, f32, u32)> {
        let mut points = Vec::new();

        // Expand bbox to include vertices that might influence this chunk
        let expanded = bbox.expanded(Vec2::broadcast(self.config.max_influence_distance));

        for vertex in &map.vertices {
            // Only include vertices marked as terrain control points
            let is_terrain_control = vertex.properties.get_bool_default("terrain_control", false);
            if !is_terrain_control {
                continue;
            }

            let pos = vertex.as_vec2();

            // Only include vertices within influence range
            if expanded.contains(pos) {
                // Use vertex Z coordinate as height (in world space, this becomes Y)
                let height = vertex.z;
                points.push((pos, height, vertex.id));
            }
        }

        points
    }

    /// Collect sectors marked for terrain exclusion
    fn collect_excluded_sectors(&self, map: &Map, bbox: &BBox) -> Vec<u32> {
        let mut excluded = Vec::new();

        for sector in &map.sectors {
            // Check if sector intersects chunk bbox
            let sector_bbox = sector.bounding_box(map);
            if !sector_bbox.intersects(bbox) {
                continue;
            }

            // Check terrain_mode property
            let terrain_mode = sector
                .properties
                .get_str_default("terrain_mode", "none".to_string());
            if terrain_mode == "exclude" {
                excluded.push(sector.id);
            }
        }

        excluded
    }

    /// Generate grid points within chunk bbox
    fn generate_grid(&self, bbox: &BBox) -> Vec<Vec2<f32>> {
        let mut grid = Vec::new();

        // Cell size based on subdivisions: subdiv=1 → 1.0, subdiv=2 → 0.5, subdiv=4 → 0.25
        let cell_size = 1.0 / self.config.subdivisions as f32;

        // Align to world tile grid
        let min_x = bbox.min.x.floor();
        let min_y = bbox.min.y.floor();
        let max_x = bbox.max.x.ceil();
        let max_y = bbox.max.y.ceil();

        // Generate grid points at subdivision resolution
        let steps_x = ((max_x - min_x) / cell_size).ceil() as i32 + 1;
        let steps_y = ((max_y - min_y) / cell_size).ceil() as i32 + 1;

        for iy in 0..steps_y {
            for ix in 0..steps_x {
                let x = min_x + ix as f32 * cell_size;
                let y = min_y + iy as f32 * cell_size;
                grid.push(Vec2::new(x, y));
            }
        }

        grid
    }

    /// Interpolate heights at grid points using Inverse Distance Weighting
    /// Returns (height, nearest_vertex_id) for tile assignment
    fn interpolate_heights(
        &self,
        grid: &[Vec2<f32>],
        control_points: &[(Vec2<f32>, f32, u32)],
    ) -> Vec<(f32, Option<u32>)> {
        grid.iter()
            .map(|&grid_point| self.interpolate_height_at(grid_point, control_points))
            .collect()
    }

    /// Interpolate height at a single point using IDW
    /// Returns (height, nearest_vertex_id)
    fn interpolate_height_at(
        &self,
        point: Vec2<f32>,
        control_points: &[(Vec2<f32>, f32, u32)],
    ) -> (f32, Option<u32>) {
        if control_points.is_empty() {
            return (0.0, None);
        }

        // Check for exact match first (avoid division by zero)
        for &(cp_pos, cp_height, vertex_id) in control_points {
            if (point - cp_pos).magnitude() < 1e-6 {
                return (cp_height, Some(vertex_id));
            }
        }

        // Inverse Distance Weighting - also track which vertex has most influence
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;
        let mut max_weight = 0.0;
        let mut nearest_vertex_id = None;

        for &(cp_pos, cp_height, vertex_id) in control_points {
            let distance = (point - cp_pos).magnitude();

            // Skip control points beyond max influence distance
            if distance > self.config.max_influence_distance {
                continue;
            }

            let weight = 1.0 / distance.powf(self.config.idw_power);
            weighted_sum += weight * cp_height;
            weight_sum += weight;

            // Track vertex with highest weight (nearest)
            if weight > max_weight {
                max_weight = weight;
                nearest_vertex_id = Some(vertex_id);
            }
        }

        if weight_sum > 0.0 {
            (weighted_sum / weight_sum, nearest_vertex_id)
        } else {
            // No control points in range, use default height
            (0.0, None)
        }
    }

    /// Apply exclusions by cutting holes in the mesh and group triangles by tile
    /// Returns a vector of (vertices, indices, UVs, vertex_id) for each tile
    fn apply_exclusions_and_group(
        &self,
        grid: &[Vec2<f32>],
        heights: &[(f32, Option<u32>)],
        excluded_sectors: &[u32],
        map: &Map,
    ) -> Vec<(Vec<Vec3<f32>>, Vec<u32>, Vec<[f32; 2]>, u32)> {
        use std::collections::HashMap;

        // Build vertices, skipping points inside excluded sectors
        let mut vertices = Vec::new();
        let mut vertex_map = vec![None; grid.len()]; // Maps grid index to final vertex index
        let mut vertex_tile_ids = Vec::new(); // Track tile (vertex_id) for each vertex

        for (i, (&grid_point, &(height, vertex_id))) in grid.iter().zip(heights.iter()).enumerate()
        {
            let mut excluded = false;

            for &sector_id in excluded_sectors {
                if let Some(sector) = map.find_sector(sector_id) {
                    if sector.is_inside(map, grid_point) {
                        excluded = true;
                        break;
                    }
                }
            }

            if !excluded {
                vertex_map[i] = Some(vertices.len());
                // Convert to world space: grid XY becomes world XZ, height becomes Y
                vertices.push(Vec3::new(grid_point.x, height, grid_point.y));
                vertex_tile_ids.push(vertex_id);
            }
        }

        // Generate triangles and group by tile
        let triangles_by_tile =
            self.triangulate_and_group_by_tile(grid, &vertex_map, &vertex_tile_ids);

        // Build separate meshes for each tile
        let mut result = Vec::new();

        for (tile_vertex_id, triangle_indices) in triangles_by_tile {
            // Build unique vertex list for this tile
            let mut tile_vertices = Vec::new();
            let mut tile_vertex_map = HashMap::new();
            let mut tile_indices = Vec::new();

            for &(v0, v1, v2) in &triangle_indices {
                // Remap each vertex index to tile-local index
                for &orig_idx in &[v0, v1, v2] {
                    if !tile_vertex_map.contains_key(&orig_idx) {
                        let new_idx = tile_vertices.len();
                        tile_vertex_map.insert(orig_idx, new_idx);
                        tile_vertices.push(vertices[orig_idx]);
                    }
                }

                // Add triangle with remapped indices
                tile_indices.push(tile_vertex_map[&v0] as u32);
                tile_indices.push(tile_vertex_map[&v1] as u32);
                tile_indices.push(tile_vertex_map[&v2] as u32);
            }

            // Generate UVs for this tile
            let tile_uvs = self.generate_uvs(&tile_vertices);

            result.push((tile_vertices, tile_indices, tile_uvs, tile_vertex_id));
        }

        result
    }

    /// Triangulate the grid and group triangles by tile
    /// Returns HashMap of tile_vertex_id -> Vec of triangle vertex indices
    fn triangulate_and_group_by_tile(
        &self,
        grid: &[Vec2<f32>],
        vertex_map: &[Option<usize>],
        vertex_tile_ids: &[Option<u32>],
    ) -> std::collections::HashMap<u32, Vec<(usize, usize, usize)>> {
        use std::collections::HashMap;

        let mut triangles_by_tile: HashMap<u32, Vec<(usize, usize, usize)>> = HashMap::new();

        // Calculate grid dimensions
        let cell_size = 1.0 / self.config.subdivisions as f32;
        let min_x = grid.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let max_x = grid.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);

        let cols = ((max_x - min_x) / cell_size).round() as usize + 1;

        // Generate triangles for each grid cell
        for (i, _) in grid.iter().enumerate() {
            let col = i % cols;

            // Skip if this is not a bottom-left corner of a cell
            if col >= cols - 1 {
                continue;
            }

            // Get the four corners of this cell
            let i0 = i; // bottom-left
            let i1 = i + 1; // bottom-right
            let i2 = i + cols; // top-left
            let i3 = i + cols + 1; // top-right

            if i2 >= grid.len() || i3 >= grid.len() {
                continue;
            }

            // Check if all four vertices exist (not excluded)
            if let (Some(v0), Some(v1), Some(v2), Some(v3)) = (
                vertex_map[i0],
                vertex_map[i1],
                vertex_map[i2],
                vertex_map[i3],
            ) {
                // Determine which tile this quad belongs to
                // Use tile from the vertex with a valid tile_id (prioritize v0)
                let tile_id = vertex_tile_ids[v0]
                    .or(vertex_tile_ids[v1])
                    .or(vertex_tile_ids[v2])
                    .or(vertex_tile_ids[v3])
                    .unwrap_or(0); // Use 0 as default tile marker

                let triangles = triangles_by_tile.entry(tile_id).or_insert_with(Vec::new);

                // Two triangles per quad
                triangles.push((v0, v1, v2));
                triangles.push((v1, v3, v2));
            }
        }

        triangles_by_tile
    }

    /// Generate UV coordinates for vertices
    fn generate_uvs(&self, vertices: &[Vec3<f32>]) -> Vec<[f32; 2]> {
        // UV mapping: 1:1 with world tiles (1.0 world unit = 1.0 UV unit = 1 tile)
        vertices
            .iter()
            .map(|v| [v.x, v.z]) // Direct mapping: world XZ → UV
            .collect()
    }
}
