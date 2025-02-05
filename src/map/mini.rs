use crate::{Linedef, Vertex};
use vek::Vec2;

/// A miniature version of the Map used for client side lighting calculations during the rasterization process.
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
}
