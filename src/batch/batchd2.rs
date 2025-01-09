use crate::prelude::*;
use vek::{Mat3, Mat4, Vec2, Vec3};

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
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform_2d: Mat3::identity(),
            transform_3d: Mat4::identity(),
            receives_light: true,
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
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform_2d: Mat3::identity(),
            transform_3d: Mat4::identity(),
            receives_light: true,
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

    /// Add a set of geometry to the batch with wrapping (to create tilable textures).
    pub fn add_wrapped(
        &mut self,
        vertices: Vec<[f32; 3]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
        wrap_size: f32,
    ) {
        // Helper function to wrap a vertex
        let wrap_vertex = |v: [f32; 3], offset: [f32; 2]| -> [f32; 3] {
            [
                v[0] + offset[0] * wrap_size,
                v[1] + offset[1] * wrap_size,
                v[2], // Keep z-coordinate unchanged
            ]
        };

        // Offsets for wrapping in 2D space
        let offsets = [
            [0.0, 0.0],   // Original polygon
            [1.0, 0.0],   // Wrap to the right
            [-1.0, 0.0],  // Wrap to the left
            [0.0, 1.0],   // Wrap to the top
            [0.0, -1.0],  // Wrap to the bottom
            [1.0, 1.0],   // Wrap top-right
            [-1.0, 1.0],  // Wrap top-left
            [1.0, -1.0],  // Wrap bottom-right
            [-1.0, -1.0], // Wrap bottom-left
        ];

        let mut all_wrapped_vertices = vec![];
        let mut all_wrapped_uvs = vec![];
        let mut all_wrapped_indices = vec![];

        for offset in offsets.iter() {
            // Wrap vertices for the current offset
            let wrapped_vertices: Vec<[f32; 3]> =
                vertices.iter().map(|&v| wrap_vertex(v, *offset)).collect();

            // Offset indices for the current set of wrapped vertices
            let base_index = all_wrapped_vertices.len();
            let wrapped_indices: Vec<(usize, usize, usize)> = indices
                .iter()
                .map(|&(i0, i1, i2)| (i0 + base_index, i1 + base_index, i2 + base_index))
                .collect();

            // Collect all wrapped data
            all_wrapped_vertices.extend(wrapped_vertices);
            all_wrapped_uvs.extend(uvs.clone());
            all_wrapped_indices.extend(wrapped_indices);
        }

        // Add all wrapped vertices, UVs, and indices to the batch
        self.vertices.extend(all_wrapped_vertices);
        self.uvs.extend(all_wrapped_uvs);
        self.indices.extend(all_wrapped_indices);
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

        if self.mode == PrimitiveMode::Lines {
            // In line mode we add the start / end vertices directly.
            let vertices = vec![
                [start[0], start[1], 1.0],
                [end[0], end[1], 1.0],
                [end[0], end[1], 1.0],     // Repeated to ensure valid triangles
                [start[0], start[1], 1.0], // Repeated to ensure valid triangles
            ];

            let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

            self.vertices.extend(vertices);
            self.uvs.extend(uvs);

            self.indices.extend(vec![
                (base_index, base_index + 1, base_index + 2),
                (base_index, base_index + 2, base_index + 3),
            ]);
        } else {
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
    }

    pub fn add_wrapped_line(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        thickness: f32,
        wrap_size: f32,
    ) {
        // Helper function to wrap a point
        let wrap_point = |p: Vec2<f32>, offset: [f32; 2]| -> Vec2<f32> {
            Vec2::new(p.x + offset[0] * wrap_size, p.y + offset[1] * wrap_size)
        };

        // Offsets for wrapping in 2D space
        let offsets = [
            [0.0, 0.0],   // Original line
            [1.0, 0.0],   // Wrap to the right
            [-1.0, 0.0],  // Wrap to the left
            [0.0, 1.0],   // Wrap to the top
            [0.0, -1.0],  // Wrap to the bottom
            [1.0, 1.0],   // Wrap top-right
            [-1.0, 1.0],  // Wrap top-left
            [1.0, -1.0],  // Wrap bottom-right
            [-1.0, -1.0], // Wrap bottom-left
        ];

        for offset in offsets.iter() {
            // Wrap start and end points
            let wrapped_start = wrap_point(start, *offset);
            let wrapped_end = wrap_point(end, *offset);

            // Add the wrapped line using the standard add_line logic
            self.add_line(wrapped_start, wrapped_end, thickness);
        }
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

    /// Set the 3D transform matrix for this batch
    pub fn transform(mut self, transform: Mat3<f32>) -> Self {
        self.transform_2d = transform;
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

                crate::Edges::new(
                    [
                        [v0[0], v0[1]], // First edge start
                        [v1[0], v1[1]], // Second edge start
                        [v2[0], v2[1]], // Third edge start
                    ],
                    [
                        [v1[0], v1[1]], // First edge end
                        [v2[0], v2[1]], // Second edge end
                        [v0[0], v0[1]], // Third edge end
                    ],
                    true,
                )
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
