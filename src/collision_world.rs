use rustc_hash::FxHashMap;
use scenevm::GeoId;
use vek::{Vec2, Vec3};

/// Manages collision data across all chunks in the world
pub struct CollisionWorld {
    /// Collision data indexed by chunk coordinates
    chunks: FxHashMap<Vec2<i32>, ChunkCollision>,
    /// Current state of dynamic geometry (doors open/closed, etc.)
    dynamic_states: FxHashMap<GeoId, DynamicState>,
    /// Chunk size (must match rendering chunk size)
    chunk_size: i32,
}

/// Collision data for a single chunk
#[derive(Clone, Debug)]
pub struct ChunkCollision {
    /// Static blocking volumes (walls, extruded surfaces)
    pub static_volumes: Vec<BlockingVolume>,
    /// Dynamic openings (doors, windows) with their GeoIds
    pub dynamic_openings: Vec<DynamicOpening>,
    /// Walkable floor regions
    pub walkable_floors: Vec<WalkableFloor>,
}

/// A static blocking volume (wall, extruded surface, etc.)
#[derive(Clone, Debug)]
pub struct BlockingVolume {
    pub geo_id: GeoId,
    pub min: Vec3<f32>,
    pub max: Vec3<f32>,
}

/// A dynamic opening that can change state (door, window, etc.)
#[derive(Clone, Debug)]
pub struct DynamicOpening {
    /// GeoId for this opening (used to control state)
    pub geo_id: GeoId,
    /// 2D boundary polygon in world space (XZ plane)
    pub boundary_2d: Vec<Vec2<f32>>,
    /// Floor height (Y coordinate)
    pub floor_height: f32,
    /// Ceiling height (Y coordinate)
    pub ceiling_height: f32,
    /// Type of opening
    pub opening_type: OpeningType,
}

/// Type of dynamic opening
#[derive(Clone, Debug, PartialEq)]
pub enum OpeningType {
    Door,    // Can open/close
    Window,  // Always blocking at player height
    Passage, // Always passable
}

/// A walkable floor region
#[derive(Clone, Debug)]
pub struct WalkableFloor {
    pub geo_id: GeoId,
    pub height: f32,
    pub polygon_2d: Vec<Vec2<f32>>,
}

/// State of a dynamic geometry element
#[derive(Clone, Debug)]
pub struct DynamicState {
    /// Whether this geometry is currently passable
    pub is_passable: bool,
    /// Animation progress (0.0 = closed, 1.0 = open)
    pub animation_progress: f32,
}

impl Default for ChunkCollision {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkCollision {
    pub fn new() -> Self {
        Self {
            static_volumes: Vec::new(),
            dynamic_openings: Vec::new(),
            walkable_floors: Vec::new(),
        }
    }
}

impl Default for CollisionWorld {
    fn default() -> Self {
        Self::new(10) // Default chunk size
    }
}

impl CollisionWorld {
    pub fn new(chunk_size: i32) -> Self {
        Self {
            chunks: FxHashMap::default(),
            dynamic_states: FxHashMap::default(),
            chunk_size,
        }
    }

    /// Add/update collision data for a chunk
    pub fn update_chunk(&mut self, chunk_origin: Vec2<i32>, collision: ChunkCollision) {
        self.chunks.insert(chunk_origin, collision);
    }

    /// Remove collision data for a chunk (when unloading)
    pub fn remove_chunk(&mut self, chunk_origin: Vec2<i32>) {
        self.chunks.remove(&chunk_origin);
    }

    /// Check if a position is blocked (for player movement)
    pub fn is_blocked(&self, position: Vec3<f32>, radius: f32) -> bool {
        // Find which chunk(s) the position overlaps
        let chunk_coords = self.world_to_chunk(Vec2::new(position.x, position.z));

        // Check current chunk and neighbors (player might be on edge)
        for dx in -1..=1 {
            for dy in -1..=1 {
                let check_chunk = Vec2::new(chunk_coords.x + dx, chunk_coords.y + dy);
                if let Some(chunk_collision) = self.chunks.get(&check_chunk) {
                    if self.check_chunk_collision(position, radius, chunk_collision) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn check_chunk_collision(
        &self,
        position: Vec3<f32>,
        radius: f32,
        chunk: &ChunkCollision,
    ) -> bool {
        // First check if player is inside a passable opening
        // If so, don't check static volumes (openings cut through walls)
        for opening in &chunk.dynamic_openings {
            let is_passable = match opening.opening_type {
                OpeningType::Passage => true, // Always passable
                OpeningType::Window => false, // Always blocking
                OpeningType::Door => {
                    // Check dynamic state - default to passable for doors (open by default)
                    self.dynamic_states
                        .get(&opening.geo_id)
                        .map(|state| state.is_passable)
                        .unwrap_or(true) // Default to passable (open) if no state set
                }
            };

            if is_passable {
                // Check if player is within this passable opening
                if position.y + radius >= opening.floor_height
                    && position.y - radius <= opening.ceiling_height
                {
                    let in_polygon = self.point_in_polygon_2d(
                        Vec2::new(position.x, position.z),
                        &opening.boundary_2d,
                        radius,
                    );
                    if in_polygon {
                        // Player is in a passable opening - don't check static volumes
                        return false;
                    }
                }
            }
        }

        // Check static volumes
        for volume in &chunk.static_volumes {
            if self.collides_with_aabb(position, radius, volume.min, volume.max) {
                return true;
            }
        }

        // Check dynamic openings that are blocking
        for opening in &chunk.dynamic_openings {
            let is_blocking = match opening.opening_type {
                OpeningType::Passage => false, // Always passable
                OpeningType::Window => true,   // Always blocking
                OpeningType::Door => {
                    // Check dynamic state - default to passable for doors
                    self.dynamic_states
                        .get(&opening.geo_id)
                        .map(|state| !state.is_passable)
                        .unwrap_or(false) // Default to passable (not blocking) if no state set
                }
            };

            if is_blocking {
                // Check if player is in height range
                if position.y + radius >= opening.floor_height
                    && position.y - radius <= opening.ceiling_height
                {
                    // Check if player is within 2D polygon
                    if self.point_in_polygon_2d(
                        Vec2::new(position.x, position.z),
                        &opening.boundary_2d,
                        radius,
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Set the state of a dynamic opening (door open/close)
    pub fn set_opening_state(&mut self, geo_id: GeoId, is_passable: bool) {
        self.dynamic_states
            .entry(geo_id)
            .or_insert(DynamicState {
                is_passable: false,
                animation_progress: 0.0,
            })
            .is_passable = is_passable;
    }

    /// Get the state of a dynamic opening
    pub fn get_opening_state(&self, geo_id: &GeoId) -> Option<&DynamicState> {
        self.dynamic_states.get(geo_id)
    }

    /// Find floor height at position (for gravity/walking)
    pub fn get_floor_height(&self, position: Vec2<f32>) -> Option<f32> {
        let chunk_coords = self.world_to_chunk(position);

        if let Some(chunk_collision) = self.chunks.get(&chunk_coords) {
            for floor in &chunk_collision.walkable_floors {
                if self.point_in_polygon_2d(position, &floor.polygon_2d, 0.0) {
                    return Some(floor.height);
                }
            }
        }

        None
    }

    fn world_to_chunk(&self, world_pos: Vec2<f32>) -> Vec2<i32> {
        Vec2::new(
            (world_pos.x / self.chunk_size as f32).floor() as i32,
            (world_pos.y / self.chunk_size as f32).floor() as i32,
        )
    }

    fn collides_with_aabb(
        &self,
        pos: Vec3<f32>,
        radius: f32,
        min: Vec3<f32>,
        max: Vec3<f32>,
    ) -> bool {
        // For walls, we only check XZ collision (horizontal plane)
        // The Y check would prevent collision if the player is at a different height
        // We assume the player's height is handled elsewhere (they're always on the ground)

        // Expand AABB by radius in XZ plane only
        let expanded_min_x = min.x - radius;
        let expanded_max_x = max.x + radius;
        let expanded_min_z = min.z - radius;
        let expanded_max_z = max.z + radius;

        // Only check XZ collision for movement blocking
        pos.x >= expanded_min_x
            && pos.x <= expanded_max_x
            && pos.z >= expanded_min_z
            && pos.z <= expanded_max_z
    }

    fn point_in_polygon_2d(&self, point: Vec2<f32>, polygon: &[Vec2<f32>], padding: f32) -> bool {
        if polygon.len() < 3 {
            return false;
        }

        // Simple ray casting algorithm for point-in-polygon test
        let mut inside = false;
        let mut j = polygon.len() - 1;

        for i in 0..polygon.len() {
            let vi = polygon[i];
            let vj = polygon[j];

            // Apply padding (expand polygon)
            let test_point = if padding > 0.0 {
                // For now, simple implementation without padding
                // TODO: Properly expand polygon by padding distance
                point
            } else {
                point
            };

            if ((vi.y > test_point.y) != (vj.y > test_point.y))
                && (test_point.x < (vj.x - vi.x) * (test_point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }

            j = i;
        }

        // If padding is set, also check distance to edges
        if padding > 0.0 && !inside {
            // Check if point is within padding distance of any edge
            for i in 0..polygon.len() {
                let p1 = polygon[i];
                let p2 = polygon[(i + 1) % polygon.len()];

                if self.point_to_segment_distance_2d(point, p1, p2) <= padding {
                    return true;
                }
            }
        }

        inside
    }

    fn point_to_segment_distance_2d(&self, point: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> f32 {
        let ab = b - a;
        let ap = point - a;
        let ab_len_sq = ab.magnitude_squared();

        if ab_len_sq < 1e-6 {
            return ap.magnitude();
        }

        let t = (ap.dot(ab) / ab_len_sq).clamp(0.0, 1.0);
        let closest = a + ab * t;
        (point - closest).magnitude()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_collision() {
        let world = CollisionWorld::new(10);
        let pos = Vec3::new(5.0, 1.0, 5.0);
        let min = Vec3::new(4.0, 0.0, 4.0);
        let max = Vec3::new(6.0, 2.0, 6.0);

        assert!(world.collides_with_aabb(pos, 0.5, min, max));
        assert!(!world.collides_with_aabb(Vec3::new(10.0, 1.0, 5.0), 0.5, min, max));
    }

    #[test]
    fn test_point_in_polygon() {
        let world = CollisionWorld::new(10);
        let polygon = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];

        assert!(world.point_in_polygon_2d(Vec2::new(5.0, 5.0), &polygon, 0.0));
        assert!(!world.point_in_polygon_2d(Vec2::new(15.0, 5.0), &polygon, 0.0));
    }

    #[test]
    fn test_door_state() {
        let mut world = CollisionWorld::new(10);
        let door_id = GeoId::Sector(1);

        // Door starts closed (blocking)
        world.set_opening_state(door_id, false);
        assert!(!world.get_opening_state(&door_id).unwrap().is_passable);

        // Open door
        world.set_opening_state(door_id, true);
        assert!(world.get_opening_state(&door_id).unwrap().is_passable);
    }
}
