use crate::prelude::*;
use vek::{Mat3, Vec2, Vec3};

use CullMode::*;
use PrimitiveMode::*;
use RepeatMode::*;
use SampleMode::*;

// impl Default for Batch<Vec3<f32>> {
//     fn default() -> Self {
//         Self::empty()
//     }
// }

/// A batch of 3D vertices, indices and their UVs which make up a 2D polygons.
impl Batch<Vec3<f32>> {
    /// Empty constructor (the default)
    pub fn emptyd2() -> Self {
        Batch {
            mode: Triangles,
            vertices: vec![],
            indices: vec![],
            uvs: vec![],
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            color: WHITE,
            sample_mode: Nearest,
            repeat_mode: ClampXY,
            cull_mode: Off,
            texture_index: 0,
        }
    }

    /// Constructor for 2D vertices
    pub fn new_2d(
        vertices: Vec<Vec3<f32>>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<Vec2<f32>>,
    ) -> Self {
        Batch {
            mode: Triangles,
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            color: WHITE,
            sample_mode: Nearest,
            repeat_mode: ClampXY,
            cull_mode: Off,
            texture_index: 0,
        }
    }

    /// Create a Batch for a rectangle in 2D
    pub fn from_rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            Vec3::new(x, y, 1.0),                  // Bottom-left
            Vec3::new(x, y + height, 1.0),         // Top-left
            Vec3::new(x + width, y + height, 1.0), // Top-right
            Vec3::new(x + width, y, 1.0),          // Bottom-right
        ];

        let indices = vec![(0, 1, 2), (0, 2, 3)];

        let uvs = vec![
            Vec2::new(0.0, 1.0), // Top-left
            Vec2::new(0.0, 0.0), // Bottom-left
            Vec2::new(1.0, 0.0), // Bottom-right
            Vec2::new(1.0, 1.0), // Top-right
        ];

        Batch::new_2d(vertices, indices, uvs)
    }

    /// Append a rectangle to the existing batch
    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let base_index = self.vertices.len();

        // Add vertices
        self.vertices.extend(vec![
            Vec3::new(x, y, 1.0),                  // Bottom-left
            Vec3::new(x, y + height, 1.0),         // Top-left
            Vec3::new(x + width, y + height, 1.0), // Top-right
            Vec3::new(x + width, y, 1.0),          // Bottom-right
        ]);

        // Add UVs
        self.uvs.extend(vec![
            Vec2::new(0.0, 1.0), // Top-left
            Vec2::new(0.0, 0.0), // Bottom-left
            Vec2::new(1.0, 0.0), // Bottom-right
            Vec2::new(1.0, 1.0), // Top-right
        ]);

        // Add indices
        self.indices.extend(vec![
            (base_index, base_index + 1, base_index + 2),
            (base_index, base_index + 2, base_index + 3),
        ]);
    }

    /*
    /// Append a rectangle to the existing batch. Has to be rendered in Line mode.
    pub fn add_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let base_index = self.vertices.len();

        // Add vertices
        self.vertices
            .extend(vec![Vec3::new(x0, y0, 1.0), Vec3::new(x1, y1, 1.0)]);

        // Add indices
        self.indices
            .extend(vec![(base_index, base_index + 1, base_index)]);
    }*/

    /// Append a line to the existing batch
    pub fn add_line(&mut self, start: Vec2<f32>, end: Vec2<f32>, thickness: f32) {
        let direction = (end - start).normalized();
        let normal = Vec2::new(-direction.y, direction.x) * thickness / 2.0;

        let base_index = self.vertices.len();

        // Calculate vertices for a thick line
        let vertices = vec![
            Vec3::new(start.x - normal.x, start.y - normal.y, 1.0),
            Vec3::new(start.x + normal.x, start.y + normal.y, 1.0),
            Vec3::new(end.x + normal.x, end.y + normal.y, 1.0),
            Vec3::new(end.x - normal.x, end.y - normal.y, 1.0),
        ];

        let uvs = vec![
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
        ];

        self.vertices.extend(vertices);
        self.uvs.extend(uvs);

        // Add indices for two triangles
        self.indices.extend(vec![
            (base_index, base_index + 1, base_index + 2),
            (base_index, base_index + 2, base_index + 3),
        ]);
    }

    /// Sets the drawing mode for the batch using the builder pattern.
    pub fn mode(mut self, mode: PrimitiveMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the sample mode for the batch using the builder pattern.
    pub fn sample_mode(mut self, sample_mode: SampleMode) -> Self {
        self.sample_mode = sample_mode;
        self
    }

    /// Sets the repeat mode for the batch using the builder pattern.
    pub fn repeat_mode(mut self, repeat_mode: RepeatMode) -> Self {
        self.repeat_mode = repeat_mode;
        self
    }

    /// Sets the cull mode for the batch using the builder pattern.
    pub fn cull_mode(mut self, cull_mode: CullMode) -> Self {
        self.cull_mode = cull_mode;
        self
    }

    /// Set the color for the batch using the builder pattern. Colors are only used for line drawing.
    pub fn color(mut self, color: Pixel) -> Self {
        self.color = color;
        self
    }

    /// Set the texture index into the texture array for the batch using the builder pattern.
    pub fn texture_index(mut self, texture_index: usize) -> Self {
        self.texture_index = texture_index;
        self
    }

    /// Project 2D vertices using a optional Mat3 transformation matrix
    pub fn project(&mut self, matrix: Option<Mat3<f32>>) {
        if let Some(matrix) = matrix {
            self.projected_vertices = self.vertices.iter().map(|&v| matrix * v).collect();
        } else {
            self.projected_vertices = self.vertices.clone();
        }

        // Precompute batch bounding box
        self.bounding_box = Some(self.calculate_bounding_box());

        // Precompute edges for each triangle
        self.edges = self
            .indices
            .iter()
            .map(|&(i0, i1, i2)| {
                let v0 = self.projected_vertices[i0];
                let v1 = self.projected_vertices[i1];
                let v2 = self.projected_vertices[i2];
                [
                    Edge::new(Vec2::new(v0.x, v0.y), Vec2::new(v1.x, v1.y), true),
                    Edge::new(Vec2::new(v1.x, v1.y), Vec2::new(v2.x, v2.y), true),
                    Edge::new(Vec2::new(v2.x, v2.y), Vec2::new(v0.x, v0.y), true),
                ]
            })
            .collect();
    }

    /// Calculate the bounding box for the projected vertices
    fn calculate_bounding_box(&self) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for v in &self.projected_vertices {
            min_x = min_x.min(v.x);
            max_x = max_x.max(v.x);
            min_y = min_y.min(v.y);
            max_y = max_y.max(v.y);
        }

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}
