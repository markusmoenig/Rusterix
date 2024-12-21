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
    pub fn new(v0: &[f32; 2], v1: &[f32; 2], visible: bool) -> Self {
        let a = v1[1] - v0[1];
        let b = v0[0] - v1[0];
        let c = v1[0] * v0[1] - v1[1] * v0[0];
        Edge { a, b, c, visible }
    }

    /// Evaluate the edge function for a point p
    #[inline(always)]
    pub fn evaluate(&self, p: [f32; 2]) -> f32 {
        self.a * p[0] + self.b * p[1] + self.c
    }
}
