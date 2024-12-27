use vek::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Edges {
    a: Vec3<f32>,
    b: Vec3<f32>,
    c: Vec3<f32>,
    pub visible: bool,
}

/// Represents pre-computed edges of a 2D triangle.
impl Edges {
    /// Create edges from three pairs of vertices
    pub fn new(v0: [[f32; 2]; 3], v1: [[f32; 2]; 3], visible: bool) -> Self {
        let a = Vec3::new(
            v1[0][1] - v0[0][1],
            v1[1][1] - v0[1][1],
            v1[2][1] - v0[2][1],
        );
        let b = Vec3::new(
            v0[0][0] - v1[0][0],
            v0[1][0] - v1[1][0],
            v0[2][0] - v1[2][0],
        );
        let c = Vec3::new(
            v1[0][0] * v0[0][1] - v1[0][1] * v0[0][0],
            v1[1][0] * v0[1][1] - v1[1][1] * v0[1][0],
            v1[2][0] * v0[2][1] - v1[2][1] * v0[2][0],
        );
        Edges { a, b, c, visible }
    }

    /// Evaluate all edges for a point and return true if the point is inside the triangle.
    pub fn evaluate(&self, p: [f32; 2]) -> bool {
        let results = self.a * p[0] + self.b * p[1] + self.c;
        results.map(|v| v >= 0.0).reduce(|a, b| a && b)
    }
}
