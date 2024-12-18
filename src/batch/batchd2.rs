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
impl Batch<[f32; 3]> {
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
        vertices: Vec<[f32; 3]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
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

    /// Create a Batch for a rectangle in 2D.
    pub fn from_rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            [x, y, 1.0],                  // Bottom-left
            [x, y + height, 1.0],         // Top-left
            [x + width, y + height, 1.0], // Top-right
            [x + width, y, 1.0],          // Bottom-right
        ];

        let indices = vec![(0, 1, 2), (0, 2, 3)];

        let uvs = vec![
            [0.0, 0.0], // Top-left
            [0.0, 1.0], // Bottom-left
            [1.0, 1.0], // Bottom-right
            [1.0, 0.0], // Top-right
        ];

        Batch::new_2d(vertices, indices, uvs)
    }

    /// Append a rectangle to the existing batch
    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let base_index = self.vertices.len();

        // Add vertices
        self.vertices.extend(vec![
            [x, y, 1.0],                  // Bottom-left
            [x, y + height, 1.0],         // Top-left
            [x + width, y + height, 1.0], // Top-right
            [x + width, y, 1.0],          // Bottom-right
        ]);

        // Add UVs
        self.uvs.extend(vec![
            [0.0, 0.0], // Top-left
            [0.0, 1.0], // Bottom-left
            [1.0, 1.0], // Bottom-right
            [1.0, 0.0], // Top-right
        ]);

        // Add indices
        self.indices.extend(vec![
            (base_index, base_index + 1, base_index + 2),
            (base_index, base_index + 2, base_index + 3),
        ]);
    }

    /// Add a set of geometry to the batch.
    pub fn add(
        &mut self,
        vertices: Vec<[f32; 3]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
    ) {
        let base_index = self.vertices.len();

        self.vertices.extend(vertices);
        self.uvs.extend(uvs);

        for i in &indices {
            self.indices
                .push((i.0 + base_index, i.1 + base_index, i.2 + base_index));
        }
    }

    /// Append a line to the existing batch
    pub fn add_line(&mut self, start: Vec2<f32>, end: Vec2<f32>, thickness: f32) {
        let start = [start.x, start.y];
        let end = [end.x, end.y];

        let direction = [end[0] - start[0], end[1] - start[1]];
        let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        let normalized = [direction[0] / length, direction[1] / length];
        let normal = [
            -normalized[1] * thickness / 2.0,
            normalized[0] * thickness / 2.0,
        ];

        let base_index = self.vertices.len();

        let vertices = vec![
            [start[0] - normal[0], start[1] - normal[1], 1.0],
            [start[0] + normal[0], start[1] + normal[1], 1.0],
            [end[0] + normal[0], end[1] + normal[1], 1.0],
            [end[0] - normal[0], end[1] - normal[1], 1.0],
        ];

        let uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

        self.vertices.extend(vertices);
        self.uvs.extend(uvs);

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
            self.projected_vertices = self
                .vertices
                .iter()
                .map(|&v| {
                    let result = matrix * Vec3::new(v[0], v[1], v[2]);
                    [result.x, result.y, result.z]
                })
                .collect();
        } else {
            self.projected_vertices = self.vertices.clone();
        }

        // Precompute the bounding box
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
                    Edge::new([v0[0], v0[1]], [v1[0], v1[1]], true),
                    Edge::new([v1[0], v1[1]], [v2[0], v2[1]], true),
                    Edge::new([v2[0], v2[1]], [v0[0], v0[1]], true),
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
            min_x = min_x.min(v[0]); // `x` coordinate
            max_x = max_x.max(v[0]);
            min_y = min_y.min(v[1]); // `y` coordinate
            max_y = max_y.max(v[1]);
        }

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}
