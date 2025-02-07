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

    /*
    /// Returns collision distance and surface normal if collision occurs
    pub fn move_distance(
        &self,
        start_pos: Vec2<f32>,
        move_vector: Vec2<f32>,
        radius: f32,
    ) -> (Vec2<f32>, bool) {
        let mut current_pos = start_pos;
        let remaining_vector = move_vector;
        let mut blocked = false;
        let epsilon = 0.001; // Small offset to prevent wall sticking

        // Single iteration with proper sliding
        let mut closest_hit = None;

        // First pass: Find earliest collision
        for linedef in &self.linedefs {
            if let (Some(start), Some(end)) = (
                self.find_vertex(linedef.start_vertex),
                self.find_vertex(linedef.end_vertex),
            ) {
                let line_start = start.as_vec2();
                let line_end = end.as_vec2();

                let radius = radius + linedef.properties.get_float_default("wall_width", 0.0) / 2.0;

                if let Some((distance, normal)) = self.check_intersection(
                    current_pos,
                    current_pos + remaining_vector,
                    line_start,
                    line_end,
                    radius,
                ) {
                    if closest_hit.map(|(d, _)| distance < d).unwrap_or(true) {
                        closest_hit = Some((distance, normal));
                    }
                }
            }
        }

        if let Some((distance, normal)) = closest_hit {
            blocked = true;

            // Move up to collision point with epsilon buffer
            let move_dir = remaining_vector.normalized();
            let allowed_move = move_dir * (distance - epsilon);
            current_pos += allowed_move;

            // Calculate sliding direction using remaining movement
            let remaining = remaining_vector - allowed_move;
            let tangent = Vec2::new(-normal.y, normal.x);
            let slide_amount = remaining.dot(tangent);

            // Apply sliding movement with wall-end detection
            current_pos += tangent * slide_amount;

            // Final position correction to prevent wall penetration
            current_pos += normal * epsilon;
        } else {
            // No collision, full movement
            current_pos += remaining_vector;
        }

        (current_pos, blocked)
    }

    /// Improved intersection check with endpoint handling
    fn check_intersection(
        &self,
        start_pos: Vec2<f32>,
        end_pos: Vec2<f32>,
        line_start: Vec2<f32>,
        line_end: Vec2<f32>,
        radius: f32,
    ) -> Option<(f32, Vec2<f32>)> {
        let line_vec = line_end - line_start;
        let line_length = line_vec.magnitude();
        if line_length < f32::EPSILON {
            return None; // Zero-length line
        }

        let line_dir = line_vec / line_length;
        let normal = Vec2::new(-line_dir.y, line_dir.x);

        // Calculate distance from line using expanded polygon
        let start_dist = (start_pos - line_start).dot(normal);
        let end_dist = (end_pos - line_start).dot(normal);

        // Check if movement crosses the expanded line
        if start_dist > radius && end_dist > radius {
            return None;
        }
        if start_dist < -radius && end_dist < -radius {
            return None;
        }

        // Find intersection point
        let t = (radius - start_dist) / (end_dist - start_dist);
        if !(0.0..=1.0).contains(&t) {
            return None;
        }

        let intersection = start_pos + (end_pos - start_pos) * t;

        // Verify intersection lies within line segment
        let line_proj = (intersection - line_start).dot(line_dir);
        if line_proj < -radius || line_proj > line_length + radius {
            return None;
        }

        let distance = (intersection - start_pos).magnitude();
        Some((distance, normal))
    }*/

    /// Returns collision distance if collision occurs
    pub fn move_distance(
        &self,
        start_pos: Vec2<f32>,
        move_vector: Vec2<f32>,
        radius: f32,
    ) -> (Vec2<f32>, bool) {
        const MAX_ITERATIONS: usize = 3;
        const EPSILON: f32 = 0.001;

        let mut current_pos = start_pos;
        let mut remaining = move_vector;
        let mut blocked = false;
        let mut iterations = 0;

        while remaining.magnitude_squared() > EPSILON * EPSILON && iterations < MAX_ITERATIONS {
            iterations += 1;

            // Find earliest collision in remaining path
            let mut closest_collision = None;
            for linedef in &self.linedefs {
                if let (Some(start_v), Some(end_v)) = (
                    self.find_vertex(linedef.start_vertex),
                    self.find_vertex(linedef.end_vertex),
                ) {
                    let line_start = start_v.as_vec2();
                    let line_end = end_v.as_vec2();

                    // Add any 'wall_width' to the player's collision radius
                    let coll_radius =
                        radius + linedef.properties.get_float_default("wall_width", 0.0) / 2.0;

                    if let Some((distance, normal)) = self.check_intersection(
                        current_pos,
                        current_pos + remaining,
                        line_start,
                        line_end,
                        coll_radius,
                    ) {
                        // Keep the closest collision only
                        if closest_collision.map_or(true, |(d, _)| distance < d) {
                            closest_collision = Some((distance, normal));
                        }
                    }
                }
            }

            match closest_collision {
                Some((distance, normal)) => {
                    blocked = true;

                    // Move up to (just before) collision point
                    let move_dir = remaining.normalized();
                    let allowed_move = move_dir * (distance - EPSILON);
                    current_pos += allowed_move;

                    // Project leftover movement onto the wall's tangent
                    let leftover = remaining.magnitude() - distance;
                    if leftover > EPSILON {
                        // Remove the component along the normal, leaving only tangent
                        let normal_component = normal.dot(remaining) * normal;
                        let slide_vec = remaining - normal_component;
                        let slide_len = slide_vec.magnitude();

                        // Reassign 'remaining' to be the tangent movement scaled to leftover length
                        if slide_len > EPSILON {
                            remaining = slide_vec.normalized() * leftover;
                        } else {
                            remaining = Vec2::zero();
                        }
                    } else {
                        remaining = Vec2::zero();
                    }

                    // Nudge outward from wall to avoid corner clipping
                    current_pos += normal * EPSILON;
                }
                None => {
                    // No collision => just move the full vector
                    current_pos += remaining;
                    remaining = Vec2::zero();
                }
            }
        }

        // Final position correction: ensure we're not still penetrating any walls
        /*
        for linedef in &self.linedefs {
            if let (Some(start_v), Some(end_v)) = (
                self.find_vertex(linedef.start_vertex),
                self.find_vertex(linedef.end_vertex),
            ) {
                let line_start = start_v.as_vec2();
                let line_end = end_v.as_vec2();

                let coll_radius =
                    radius + linedef.properties.get_float_default("wall_width", 0.0) / 2.0;

                if let Some((distance, normal)) = self.check_intersection(
                    current_pos,
                    current_pos,
                    line_start,
                    line_end,
                    coll_radius,
                ) {
                    // If we're inside the wall, push out
                    if distance < coll_radius {
                        current_pos += normal * (coll_radius + EPSILON - distance);
                    }
                }
            }
        }*/

        for linedef in &self.linedefs {
            if let (Some(start_v), Some(end_v)) = (
                self.find_vertex(linedef.start_vertex),
                self.find_vertex(linedef.end_vertex),
            ) {
                let line_start = start_v.as_vec2();
                let line_end = end_v.as_vec2();
                let coll_radius =
                    radius + linedef.properties.get_float_default("wall_width", 0.0) / 2.0;

                if let Some((dist, normal)) =
                    self.check_point_against_segment(current_pos, line_start, line_end, coll_radius)
                {
                    // We are inside the wall if dist < coll_radius
                    let penetration = coll_radius - dist;
                    if penetration > 0.0 {
                        // Push out by the overlap
                        current_pos += normal * (penetration + EPSILON);
                    }
                }
            }
        }

        (current_pos, blocked)
    }

    /// Precise collision detection with corner handling
    fn check_intersection(
        &self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        line_start: Vec2<f32>,
        line_end: Vec2<f32>,
        radius: f32,
    ) -> Option<(f32, Vec2<f32>)> {
        let line_vec = line_end - line_start;
        let line_len = line_vec.magnitude();
        if line_len < f32::EPSILON {
            return None;
        }

        // Unit direction of the line
        let line_dir = line_vec / line_len;

        // A "default" normal (perpendicular)
        let normal = Vec2::new(-line_dir.y, line_dir.x);

        // Distance from line_start to start/end in the normal direction
        let start_dist = (start - line_start).dot(normal);
        let end_dist = (end - line_start).dot(normal);

        // If both start and end are entirely outside radius on the same side, no collision
        if start_dist > radius && end_dist > radius {
            return None;
        }
        if start_dist < -radius && end_dist < -radius {
            return None;
        }

        // We'll solve for the parameter t in [0..1] where we cross the "radius boundary"
        // That is, we want the moment we go from 'inside' to 'outside' or vice versa.
        let dist_diff = end_dist - start_dist;

        // If motion in normal direction is extremely small, check if already "within" the line corridor
        let t = if dist_diff.abs() < f32::EPSILON {
            // If start_dist is within ±radius, then t=0 => collision at start
            if start_dist.abs() <= radius {
                0.0
            } else {
                return None;
            }
        } else {
            // Decide which side of the line we are crossing: if start < 0 then we cross -radius, else +radius
            let desired_dist = if start_dist < 0.0 { -radius } else { radius };
            (desired_dist - start_dist) / dist_diff
        };

        // If intersection falls outside [0..1], it means we never collide on that segment
        if !(0.0..=1.0).contains(&t) {
            return None;
        }

        // Intersection point along start->end
        let intersection = start + (end - start) * t;

        // Project intersection onto the line to see if it's within segment bounds
        let line_proj = (intersection - line_start).dot(line_dir);

        // If intersection is "before" line_start or "after" line_end, we treat it as a corner check
        if line_proj < 0.0 || line_proj > line_len {
            // Check corner collisions
            if line_proj < 0.0 {
                // Collide vs. line_start as a corner
                return self.check_point_collision(intersection, line_start, radius, start);
            } else {
                // Collide vs. line_end as a corner
                return self.check_point_collision(intersection, line_end, radius, start);
            }
        }

        // Collision distance from 'start' to intersection
        let collision_dist = (intersection - start).magnitude();

        // Figure out the correct normal direction: we want a normal that pushes *out* from the line
        // (If start_dist is positive, normal points one way; if negative, we flip it)
        let final_normal = if start_dist < 0.0 { -normal } else { normal };

        Some((collision_dist, final_normal))
    }

    /// Special case for corner points
    fn check_point_collision(
        &self,
        collision_point: Vec2<f32>, // The intersection point along the player's path
        corner: Vec2<f32>,
        radius: f32,
        start: Vec2<f32>, // We also need to know the player's start
    ) -> Option<(f32, Vec2<f32>)> {
        let to_corner = collision_point - corner;
        let dist_sq = to_corner.magnitude_squared();

        // If the collision_point is more than `radius` away from the corner, no collision
        if dist_sq > radius * radius {
            return None;
        }

        // Distance from corner to the intersection
        let dist_corner = dist_sq.sqrt();

        // Normal is outward from the corner
        let normal = if dist_corner > f32::EPSILON {
            to_corner / dist_corner
        } else {
            Vec2::unit_x() // Arbitrary fallback if corner and collision_point coincide
        };

        // ***CRITICAL***: distance from the player's `start` to the collision_point
        // so the main collision code knows how far along the path we collided.
        let collision_dist = (collision_point - start).magnitude();

        Some((collision_dist, normal))
    }

    fn check_point_against_segment(
        &self,
        point: Vec2<f32>,
        seg_start: Vec2<f32>,
        seg_end: Vec2<f32>,
        radius: f32,
    ) -> Option<(f32, Vec2<f32>)> {
        let seg_vec = seg_end - seg_start;
        let seg_len = seg_vec.magnitude();
        if seg_len < f32::EPSILON {
            // Degenerate line => just check corner distance
            let d_sq = (point - seg_start).magnitude_squared();
            if d_sq > radius * radius {
                return None;
            }
            let d = d_sq.sqrt();
            let normal = if d > f32::EPSILON {
                (point - seg_start) / d
            } else {
                // Arbitrary fallback
                Vec2::new(1.0, 0.0)
            };
            return Some((d, normal));
        }

        let seg_dir = seg_vec / seg_len;
        let diff = point - seg_start;
        // Param of point's projection onto seg_start..seg_end
        let t = diff.dot(seg_dir).clamp(0.0, seg_len);
        // Closest point on the segment
        let closest_point = seg_start + seg_dir * t;

        let delta = point - closest_point;
        let dist_sq = delta.magnitude_squared();
        if dist_sq > radius * radius {
            return None; // Not penetrating
        }

        let dist = dist_sq.sqrt();
        let normal = if dist > f32::EPSILON {
            delta / dist
        } else {
            // Arbitrary fallback if exactly on the segment
            Vec2::new(1.0, 0.0)
        };

        Some((dist, normal))
    }
}
