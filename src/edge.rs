use vek::Vec2;

/// Represents a pre-computed edge of a 2D triangle. Only used internally.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    a: f32,
    b: f32,
    c: f32,
    pub visible: bool,
}

impl Edge {
    /// Create an edge from two vertices
    pub fn new(v0: Vec2<f32>, v1: Vec2<f32>, visible: bool) -> Self {
        let a = v1.y - v0.y;
        let b = v0.x - v1.x;
        let c = v1.x * v0.y - v1.y * v0.x;
        Edge { a, b, c, visible }
    }

    /// Evaluate the edge function for a point p
    pub fn evaluate(&self, p: Vec2<f32>) -> f32 {
        self.a * p.x + self.b * p.y + self.c
    }
}
