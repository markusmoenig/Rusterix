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

use crate::{Assets, BBox, Chunk, Map, PixelSource};
use rustc_hash::FxHashMap;
use uuid::Uuid;
use vek::{Vec2, Vec3};

/// Terrain generation settings
pub struct TerrainConfig {
    /// Subdivision level: 1 = one quad per world tile, 2 = 4 quads per tile, etc.
    pub subdivisions: u32,
    /// Power parameter for Inverse Distance Weighting (typically 2.0)
    pub idw_power: f32,
    /// Maximum distance for vertex influence (beyond this, influence is zero)
    pub max_influence_distance: f32,
    /// Smoothness factor: lower values = sharper peaks, higher values = smoother transitions
    pub smoothness: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            subdivisions: 1,              // 1 quad per world tile
            idw_power: 3.0,               // Increased from 2.0 for steeper, more cone-like falloff
            max_influence_distance: 50.0, // Large enough to avoid hard edges between chunks
            smoothness: 1.0,              // Default smoothness
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
    /// Returns meshes grouped by tile_id: Vec<(tile_id, vertices, indices, UVs)>
    pub fn generate(
        &self,
        map: &Map,
        chunk: &Chunk,
        assets: &Assets,
        default_tile_id: Uuid,
        tile_overrides: Option<&FxHashMap<(i32, i32), PixelSource>>,
    ) -> Option<Vec<(Uuid, Vec<Vec3<f32>>, Vec<u32>, Vec<[f32; 2]>)>> {
        // 1. Collect ALL control points from entire map (global, not per-chunk)
        // This ensures consistent terrain interpolation across chunk boundaries
        let control_points = self.collect_control_points(map);

        // 2. Identify sectors marked for terrain exclusion
        let excluded_sectors = self.collect_excluded_sectors(map, &chunk.bbox);

        // 3. Generate grid mesh
        let grid = self.generate_grid(&chunk.bbox);

        // 4. Get map bounding box for edge falloff
        // Note: In 2D editor, -Y is up, so we need to flip Y coordinates
        let map_bbox = if let Some(bounds) = map.bounding_box() {
            BBox {
                min: Vec2::new(bounds.x, -bounds.w), // Flip Y: max becomes min
                max: Vec2::new(bounds.z, -bounds.y), // Flip Y: min becomes max
            }
        } else {
            // Fallback: use a large default map if no geometry exists
            BBox {
                min: Vec2::new(-100.0, -100.0),
                max: Vec2::new(100.0, 100.0),
            }
        };

        // 5. Interpolate heights at grid points with map edge falloff
        let heights = self.interpolate_heights(&grid, &control_points, &map_bbox);

        // 5. Cut holes for excluded sectors
        let (vertices, indices, uvs) =
            self.apply_exclusions(&grid, &heights, &excluded_sectors, map);

        // Only return None if there are no vertices
        if vertices.is_empty() {
            return None;
        }

        // 6. Partition triangles by tile using tile overrides (similar to surface builder)
        let meshes_by_tile = self.partition_by_tiles(
            &vertices,
            &indices,
            &uvs,
            assets,
            default_tile_id,
            tile_overrides,
        );

        if meshes_by_tile.is_empty() {
            return None;
        }

        Some(meshes_by_tile)
    }

    /// Collect height control points from vertices (position, height, smoothness)
    /// Now collects ALL control points from the entire map to ensure consistent
    /// terrain interpolation across chunk boundaries
    fn collect_control_points(&self, map: &Map) -> Vec<(Vec2<f32>, f32, f32)> {
        let mut points = Vec::new();

        for vertex in &map.vertices {
            // Only include vertices marked as terrain control points
            let is_terrain_control = vertex.properties.get_bool_default("terrain_control", false);
            if !is_terrain_control {
                continue;
            }

            let pos = vertex.as_vec2();
            // Use vertex Z coordinate as height (in world space, this becomes Y)
            let height = vertex.z;
            // Get smoothness from vertex properties, default to global smoothness
            let smoothness = vertex
                .properties
                .get_float_default("smoothness", self.config.smoothness);
            points.push((pos, height, smoothness));
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

    /// Interpolate heights at grid points using edge-based falloff and IDW
    fn interpolate_heights(
        &self,
        grid: &[Vec2<f32>],
        control_points: &[(Vec2<f32>, f32, f32)],
        bbox: &BBox,
    ) -> Vec<f32> {
        grid.iter()
            .map(|&grid_point| self.interpolate_height_at(grid_point, control_points, bbox))
            .collect()
    }

    /// Interpolate height at a single point using IDW (Inverse Distance Weighting)
    /// Creates smooth hills that blend naturally based on control point influence
    /// Heights fade to 0 at the map boundaries
    fn interpolate_height_at(
        &self,
        point: Vec2<f32>,
        control_points: &[(Vec2<f32>, f32, f32)],
        map_bbox: &BBox,
    ) -> f32 {
        if control_points.is_empty() {
            return 0.0;
        }

        // Check for exact match first (avoid division by zero)
        for &(cp_pos, cp_height, _) in control_points {
            if (point - cp_pos).magnitude() < 1e-6 {
                // Apply map edge falloff even at exact control point
                let edge_factor = self.calculate_map_edge_falloff(point, map_bbox);
                return cp_height * edge_factor;
            }
        }

        // Use distance-based falloff with smoothness control
        let mut max_height = 0.0;

        for &(cp_pos, cp_height, smoothness) in control_points {
            let distance = (point - cp_pos).magnitude();

            // Use smoothness to control the radius of influence (like a brush)
            // smoothness directly represents the radius in tiles
            // Lower smoothness (e.g., 1.0) = 1 tile radius, small steep rounded hill
            // Higher smoothness (e.g., 20.0) = 20 tile radius, large gentle hill
            //
            let smoothness = smoothness * 2.0;
            let effective_radius = smoothness;

            // Circle SDF-based falloff for smooth, round hills (like painting with a brush)
            // Scale smoothing with radius for consistent appearance at all sizes
            let smoothing = effective_radius; // 100% of radius for smooth falloff

            // SDF distance from edge of circle (negative inside, positive outside)
            let sdf_dist = distance - effective_radius;

            // Smooth falloff using smoothstep on the SDF
            let falloff = if sdf_dist < -smoothing {
                1.0 // Full height inside the circle
            } else if sdf_dist > smoothing {
                0.0 // Zero outside the circle
            } else {
                // Smooth transition at the edge
                let t = (smoothing - sdf_dist) / (2.0 * smoothing);
                t * t * (3.0 - 2.0 * t) // Smoothstep
            };

            let height_contribution = cp_height * falloff;

            // Take the maximum height contribution from all control points
            if height_contribution > max_height {
                max_height = height_contribution;
            }
        }

        let base_height = max_height;

        // Apply map edge falloff: height transitions to 0 at map boundaries
        let edge_factor = self.calculate_map_edge_falloff(point, map_bbox);
        base_height * edge_factor
    }

    /// Calculate falloff factor based on distance from map edge
    /// Returns 0.0 at map edge, 1.0 far from edges
    fn calculate_map_edge_falloff(&self, point: Vec2<f32>, map_bbox: &BBox) -> f32 {
        // Calculate distance from each map edge
        let dist_from_left = point.x - map_bbox.min.x;
        let dist_from_right = map_bbox.max.x - point.x;
        let dist_from_bottom = point.y - map_bbox.min.y;
        let dist_from_top = map_bbox.max.y - point.y;

        // Find minimum distance to any edge
        let min_edge_dist = dist_from_left
            .min(dist_from_right)
            .min(dist_from_bottom)
            .min(dist_from_top);

        // Define falloff distance (distance from map edge where falloff starts)
        let falloff_distance = 10.0; // Adjust this to control how far from edge the falloff extends

        if min_edge_dist <= 0.0 {
            0.0 // At or beyond map edge
        } else if min_edge_dist >= falloff_distance {
            1.0 // Far from edge
        } else {
            // Smooth transition using smoothstep
            let t = min_edge_dist / falloff_distance;
            t * t * (3.0 - 2.0 * t) // Smoothstep interpolation
        }
    }

    /// Apply exclusions by clipping triangles against sector boundaries
    /// Returns (vertices, indices, UVs)
    fn apply_exclusions(
        &self,
        grid: &[Vec2<f32>],
        heights: &[f32],
        excluded_sectors: &[u32],
        map: &Map,
    ) -> (Vec<Vec3<f32>>, Vec<u32>, Vec<[f32; 2]>) {
        // First generate all grid vertices and triangles
        let mut all_vertices = Vec::new();
        let mut vertex_map = vec![None; grid.len()];

        for (i, (&grid_point, &height)) in grid.iter().zip(heights.iter()).enumerate() {
            vertex_map[i] = Some(all_vertices.len());
            all_vertices.push((grid_point, height));
        }

        // Generate all triangles (without exclusions yet)
        let all_indices = self.triangulate(grid, &vertex_map);

        if excluded_sectors.is_empty() {
            // No exclusions - just convert to output format
            let vertices: Vec<Vec3<f32>> = all_vertices
                .iter()
                .map(|(pos, h)| Vec3::new(pos.x, *h, pos.y))
                .collect();
            let uvs = self.generate_uvs(&vertices);
            return (vertices, all_indices, uvs);
        }

        // Clip triangles against excluded sectors
        let mut final_vertices = Vec::new();
        let mut final_indices = Vec::new();

        // Convert flat indices to triangle tuples
        let triangles: Vec<(usize, usize, usize)> = all_indices
            .chunks_exact(3)
            .map(|chunk| (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize))
            .collect();

        for (i0, i1, i2) in triangles {
            let p0 = all_vertices[i0].0;
            let p1 = all_vertices[i1].0;
            let p2 = all_vertices[i2].0;
            let h0 = all_vertices[i0].1;
            let h1 = all_vertices[i1].1;
            let h2 = all_vertices[i2].1;

            // Check if triangle is entirely inside any excluded sector
            let mut should_exclude = false;

            for &sector_id in excluded_sectors {
                if let Some(sector) = map.find_sector(sector_id) {
                    // Simple check: if all 3 vertices are inside the sector, exclude the triangle
                    if self.point_in_sector(p0, sector, map)
                        && self.point_in_sector(p1, sector, map)
                        && self.point_in_sector(p2, sector, map)
                    {
                        should_exclude = true;
                        break;
                    }
                }
            }

            if !should_exclude {
                // Keep the triangle as-is
                let base_idx = final_vertices.len();
                final_vertices.push(Vec3::new(p0.x, h0, p0.y));
                final_vertices.push(Vec3::new(p1.x, h1, p1.y));
                final_vertices.push(Vec3::new(p2.x, h2, p2.y));

                final_indices.push(base_idx as u32);
                final_indices.push((base_idx + 1) as u32);
                final_indices.push((base_idx + 2) as u32);
            }
        }

        let uvs = self.generate_uvs(&final_vertices);
        (final_vertices, final_indices, uvs)
    }

    /// Triangulate the grid
    /// Returns flat list of triangle indices
    fn triangulate(&self, grid: &[Vec2<f32>], vertex_map: &[Option<usize>]) -> Vec<u32> {
        let mut indices = Vec::new();

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
                // Two triangles per quad with counter-clockwise winding (for Y+ normals)
                // Triangle 1: bottom-left, top-left, bottom-right
                indices.push(v0 as u32);
                indices.push(v2 as u32);
                indices.push(v1 as u32);

                // Triangle 2: bottom-right, top-left, top-right
                indices.push(v1 as u32);
                indices.push(v2 as u32);
                indices.push(v3 as u32);
            }
        }

        indices
    }

    /// Generate UV coordinates for vertices
    fn generate_uvs(&self, vertices: &[Vec3<f32>]) -> Vec<[f32; 2]> {
        // UV mapping: 1:1 with world tiles (1.0 world unit = 1.0 UV unit = 1 tile)
        vertices
            .iter()
            .map(|v| [v.x, v.z]) // Direct mapping: world XZ → UV
            .collect()
    }

    /// Check if a point is inside a sector using ray casting algorithm
    fn point_in_sector(&self, point: Vec2<f32>, sector: &crate::Sector, map: &crate::Map) -> bool {
        // Get sector boundary vertices
        let mut sector_verts = Vec::new();
        for &linedef_id in &sector.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    sector_verts.push(Vec2::new(start_vertex.x, start_vertex.y));
                }
            }
        }

        if sector_verts.len() < 3 {
            return false;
        }

        // Ray casting algorithm: count intersections with edges
        let mut inside = false;
        let mut j = sector_verts.len() - 1;

        for i in 0..sector_verts.len() {
            let vi = sector_verts[i];
            let vj = sector_verts[j];

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }

            j = i;
        }

        inside
    }

    /// Partition triangles by tile using 1x1 UV cells (same as surface builder)
    /// Returns Vec<(tile_id, vertices, indices, UVs)>
    fn partition_by_tiles(
        &self,
        vertices: &[Vec3<f32>],
        indices: &[u32],
        uvs: &[[f32; 2]],
        assets: &Assets,
        default_tile_id: Uuid,
        tile_overrides: Option<&FxHashMap<(i32, i32), PixelSource>>,
    ) -> Vec<(Uuid, Vec<Vec3<f32>>, Vec<u32>, Vec<[f32; 2]>)> {
        let mut per_tile: FxHashMap<Uuid, Vec<u32>> = FxHashMap::default();

        // Process triangles in groups of 3 indices
        for tri_indices in indices.chunks_exact(3) {
            let i0 = tri_indices[0] as usize;
            let i1 = tri_indices[1] as usize;
            let i2 = tri_indices[2] as usize;

            // Get UVs for the triangle vertices
            let uv0 = uvs[i0];
            let uv1 = uvs[i1];
            let uv2 = uvs[i2];

            // Determine which 1x1 tile cell this triangle belongs to
            // Use the tile containing the triangle's center
            let center_u = (uv0[0] + uv1[0] + uv2[0]) / 3.0;
            let center_v = (uv0[1] + uv1[1] + uv2[1]) / 3.0;
            let tile_cell = (center_u.floor() as i32, center_v.floor() as i32);

            // Look up tile override for this cell
            let tile_id = if let Some(overrides) = tile_overrides {
                if let Some(pixel_source) = overrides.get(&tile_cell) {
                    if let Some(tile) = pixel_source.tile_from_tile_list(assets) {
                        tile.id
                    } else {
                        default_tile_id
                    }
                } else {
                    default_tile_id
                }
            } else {
                default_tile_id
            };

            // Add triangle indices to this tile's batch
            per_tile
                .entry(tile_id)
                .or_insert_with(Vec::new)
                .extend_from_slice(tri_indices);
        }

        // Build separate meshes for each tile
        let mut result = Vec::new();

        for (tile_id, tile_indices) in per_tile {
            // For each tile, we need to create a new vertex list with only used vertices
            // and remap the indices accordingly
            let mut vertex_remap: FxHashMap<u32, u32> = FxHashMap::default();
            let mut tile_vertices = Vec::new();
            let mut tile_uvs = Vec::new();
            let mut remapped_indices = Vec::new();

            for &old_idx in &tile_indices {
                let new_idx = if let Some(&existing_idx) = vertex_remap.get(&old_idx) {
                    existing_idx
                } else {
                    let new_idx = tile_vertices.len() as u32;
                    vertex_remap.insert(old_idx, new_idx);
                    tile_vertices.push(vertices[old_idx as usize]);
                    tile_uvs.push(uvs[old_idx as usize]);
                    new_idx
                };
                remapped_indices.push(new_idx);
            }

            // Keep UVs in world space (don't convert to local tile space)
            // The renderer expects world-space UVs for tiling
            result.push((tile_id, tile_vertices, remapped_indices, tile_uvs));
        }

        result
    }
}
