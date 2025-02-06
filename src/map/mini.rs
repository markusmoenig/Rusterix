use crate::{Linedef, Vertex};
use vek::Vec2;

/// A miniature version of the Map used for client side lighting calculations during the rasterization process and server side collision detection etc.
#[derive(Clone)]
pub struct MapMini {
    pub offset: Vec2<f32>,
    pub grid_size: f32,

    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
}

impl Default for MapMini {
    fn default() -> Self {
        Self::empty()
    }
}

impl MapMini {
    pub fn empty() -> Self {
        Self {
            offset: Vec2::zero(),
            grid_size: 0.0,
            vertices: vec![],
            linedefs: vec![],
        }
    }

    pub fn new(
        offset: Vec2<f32>,
        grid_size: f32,
        vertices: Vec<Vertex>,
        linedefs: Vec<Linedef>,
    ) -> Self {
        Self {
            offset,
            grid_size,
            vertices,
            linedefs,
        }
    }

    /// Finds a reference to a vertex by its ID
    pub fn find_vertex(&self, id: u32) -> Option<&Vertex> {
        self.vertices.iter().find(|vertex| vertex.id == id)
    }

    /// Returns true if the two segments intersect.
    pub fn segments_intersect(
        &self,
        a1: Vec2<f32>,
        a2: Vec2<f32>,
        b1: Vec2<f32>,
        b2: Vec2<f32>,
    ) -> bool {
        let d = (a2.x - a1.x) * (b2.y - b1.y) - (a2.y - a1.y) * (b2.x - b1.x);

        if d == 0.0 {
            return false; // Parallel lines
        }

        let u = ((b1.x - a1.x) * (b2.y - b1.y) - (b1.y - a1.y) * (b2.x - b1.x)) / d;
        let v = ((b1.x - a1.x) * (a2.y - a1.y) - (b1.y - a1.y) * (a2.x - a1.x)) / d;

        (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)
    }

    /// Test if "to" is visible from "from".
    pub fn is_visible(&self, from: Vec2<f32>, to: Vec2<f32>) -> bool {
        for linedef in &self.linedefs {
            if let Some(start) = self.find_vertex(linedef.start_vertex) {
                if let Some(end) = self.find_vertex(linedef.end_vertex) {
                    if self.segments_intersect(from, to, start.as_vec2(), end.as_vec2()) {
                        return false; // Line is blocked by a linedef
                    }
                }
            }
        }
        true
    }

    /// Test if "to" is visible from "from" and if it is lit.
    pub fn is_visible_and_lit(&self, from: Vec2<f32>, to: Vec2<f32>) -> bool {
        fn compute_normal(start: &Vec2<f32>, end: &Vec2<f32>) -> Vec2<f32> {
            let direction = (end - start).normalized();
            Vec2::new(-direction.y, direction.x)
        }
        for linedef in &self.linedefs {
            if let Some(start) = self.find_vertex(linedef.start_vertex) {
                if let Some(end) = self.find_vertex(linedef.end_vertex) {
                    let start_pos = start.as_vec2();
                    let end_pos = end.as_vec2();
                    if self.segments_intersect(from, to, start_pos, end_pos) {
                        let normal = compute_normal(&start_pos, &end_pos);
                        let light_dir = (from - to).normalized();
                        let dot_product = normal.dot(light_dir);

                        if dot_product < 0.0 {
                            return true; // Lit (hit inside)
                        } else {
                            return false; // Not lit (hit outside)
                        }
                    }
                }
            }
        }
        true // No intersection, so fully visible and lit
    }

    /// Checks how far an entity can move in the given direction before colliding with a linedef.
    /// Returns the new allowed position and a boolean indicating if movement was blocked.
    pub fn move_distance(
        &self,
        start_pos: Vec2<f32>,
        move_vector: Vec2<f32>,
        radius: f32,
    ) -> (Vec2<f32>, bool) {
        let end_pos = start_pos + move_vector;
        let mut max_distance = move_vector.magnitude();
        let mut blocked = false;

        for linedef in &self.linedefs {
            if let (Some(start_vertex), Some(end_vertex)) = (
                self.find_vertex(linedef.start_vertex),
                self.find_vertex(linedef.end_vertex),
            ) {
                let line_start = Vec2::new(start_vertex.x, start_vertex.y);
                let line_end = Vec2::new(end_vertex.x, end_vertex.y);

                // Check if the linedef should actually block (TODO: Implement actual logic)
                let is_blocking = true;

                if is_blocking {
                    // Check if movement intersects the linedef (considering entity radius)
                    if let Some(distance) =
                        self.check_intersection(start_pos, end_pos, line_start, line_end, radius)
                    {
                        if distance - radius > 0.0 {
                            max_distance = max_distance.min(distance - radius); // Stop at the radius buffer
                        } else {
                            max_distance = 0.0; // Completely blocked
                        }
                        blocked = true;
                    }
                }
            }
        }

        let final_pos = start_pos + move_vector.normalized() * max_distance;
        (final_pos, blocked)
    }

    /// Checks if movement from `start_pos` to `end_pos` would collide with a given linedef.
    /// Returns the distance at which the collision occurs, considering entity radius.
    fn check_intersection(
        &self,
        start_pos: Vec2<f32>,
        end_pos: Vec2<f32>,
        line_start: Vec2<f32>,
        line_end: Vec2<f32>,
        radius: f32,
    ) -> Option<f32> {
        let movement_dir = (end_pos - start_pos).normalized();
        let closest_point = self.closest_point_on_segment(line_start, line_end, start_pos);
        let distance_to_line = (closest_point - start_pos).magnitude();

        if distance_to_line <= radius {
            let projected_distance = movement_dir.dot(closest_point - start_pos);

            // If moving away from the wall, no collision
            if projected_distance < 0.0 {
                return None;
            }

            let safe_distance = projected_distance - radius;

            if safe_distance >= 0.0 && safe_distance <= (end_pos - start_pos).magnitude() {
                return Some(safe_distance);
            } else {
                // Movement would start inside the wall, block completely
                return Some(0.0);
            }
        }

        // Check if the movement path intersects the linedef
        let line_vec = line_end - line_start;
        let segment_to_start = start_pos - line_start;
        let segment_to_end = end_pos - line_start;

        let cross_start = segment_to_start.x * line_vec.y - segment_to_start.y * line_vec.x;
        let cross_end = segment_to_end.x * line_vec.y - segment_to_end.y * line_vec.x;

        // Check if the movement crosses the linedef
        if cross_start * cross_end < 0.0 {
            let movement_vec = end_pos - start_pos;
            let denominator = movement_vec.x * line_vec.y - movement_vec.y * line_vec.x;

            if denominator.abs() < f32::EPSILON {
                return None; // Parallel movement
            }

            let t =
                (segment_to_start.x * line_vec.y - segment_to_start.y * line_vec.x) / denominator;
            let u = (segment_to_start.x * movement_vec.y - segment_to_start.y * movement_vec.x)
                / denominator;

            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                let intersection_point = start_pos + movement_vec * t;
                let distance = (intersection_point - start_pos).magnitude();
                let adjusted_distance = distance - radius;

                if adjusted_distance >= 0.0 {
                    return Some(adjusted_distance);
                } else {
                    return Some(0.0);
                }
            }
        }

        None
    }

    /// Returns the closest point on a segment (p1, p2) to a given point p.
    fn closest_point_on_segment(&self, p1: Vec2<f32>, p2: Vec2<f32>, p: Vec2<f32>) -> Vec2<f32> {
        let line_vec = p2 - p1;
        let point_vec = p - p1;
        let line_length_sq = line_vec.magnitude_squared();

        if line_length_sq == 0.0 {
            return p1; // p1 and p2 are the same point
        }

        let t = point_vec.dot(line_vec) / line_length_sq;
        let t_clamped = t.clamp(0.0, 1.0); // Clamp to segment

        p1 + line_vec * t_clamped
    }
}
