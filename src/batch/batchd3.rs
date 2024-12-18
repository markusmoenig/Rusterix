use crate::prelude::*;
use crate::wavefront::Wavefront;
use vek::{Mat4, Vec4};

use CullMode::*;
use PrimitiveMode::*;
use RepeatMode::*;
use SampleMode::*;

/// A batch of 4D vertices, indices and their UVs which make up a 3D mesh.
impl Batch<[f32; 4]> {
    pub fn new_3d(
        vertices: Vec<[f32; 4]>,
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

    /// Create a Batch for a box in 3D
    pub fn from_box(x: f32, y: f32, z: f32, width: f32, height: f32, depth: f32) -> Self {
        let vertices = vec![
            // Front face
            [x, y, z, 1.0],                  // Bottom-left-front
            [x + width, y, z, 1.0],          // Bottom-right-front
            [x + width, y + height, z, 1.0], // Top-right-front
            [x, y + height, z, 1.0],         // Top-left-front
            // Back face
            [x, y, z + depth, 1.0],                  // Bottom-left-back
            [x + width, y, z + depth, 1.0],          // Bottom-right-back
            [x + width, y + height, z + depth, 1.0], // Top-right-back
            [x, y + height, z + depth, 1.0],         // Top-left-back
            // Left face
            [x, y, z, 1.0],                  // Bottom-left-front
            [x, y + height, z, 1.0],         // Top-left-front
            [x, y + height, z + depth, 1.0], // Top-left-back
            [x, y, z + depth, 1.0],          // Bottom-left-back
            // Right face
            [x + width, y, z, 1.0],                  // Bottom-right-front
            [x + width, y + height, z, 1.0],         // Top-right-front
            [x + width, y + height, z + depth, 1.0], // Top-right-back
            [x + width, y, z + depth, 1.0],          // Bottom-right-back
            // Top face
            [x, y + height, z, 1.0],                 // Top-left-front
            [x + width, y + height, z, 1.0],         // Top-right-front
            [x + width, y + height, z + depth, 1.0], // Top-right-back
            [x, y + height, z + depth, 1.0],         // Top-left-back
            // Bottom face
            [x, y, z, 1.0],                 // Bottom-left-front
            [x + width, y, z, 1.0],         // Bottom-right-front
            [x + width, y, z + depth, 1.0], // Bottom-right-back
            [x, y, z + depth, 1.0],         // Bottom-left-back
        ];

        let indices = vec![
            // Front face (+Z)
            (0, 1, 2),
            (0, 2, 3),
            // Back face (-Z)
            (4, 6, 5),
            (4, 7, 6),
            // Left face (-X)
            (8, 9, 10),
            (8, 10, 11),
            // Right face (+X)
            (12, 14, 13),
            (12, 15, 14),
            // Top face (+Y) - Fixed
            (16, 17, 18),
            (16, 18, 19),
            // Bottom face (-Y) - Fixed
            (20, 23, 22),
            (20, 22, 21),
        ];

        let uvs = vec![
            // Front face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Back face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Left face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Right face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Top face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            // Bottom face
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];

        Batch::new_3d(vertices, indices, uvs)
    }

    /// Load a Batch from an OBJ file using the Wavefront struct.
    pub fn from_obj(input: impl IntoDataInput) -> Self {
        // Load data using the flexible input trait
        let data = input
            .load_data()
            .expect("Failed to load data from the provided input source");

        // Parse the OBJ data
        let obj_data = String::from_utf8(data).expect("Input data is not valid UTF-8");
        let wavefront = Wavefront::parse_string(obj_data);

        // Convert the Wavefront object into a Batch
        wavefront.to_batch()
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

    /// Project 3D vertices using a Mat4 transformation matrix
    pub fn project(&mut self, matrix: Mat4<f32>, viewport_width: f32, viewport_height: f32) {
        self.projected_vertices = self
            .vertices
            .iter()
            .map(|&v| {
                let result = matrix * Vec4::new(v[0], v[1], v[2], v[3]);
                let w = 1.0; //result.w;
                [
                    ((result.x / w) * 0.5 + 0.5) * viewport_width,
                    ((result.y / w) * 0.5 + 0.5) * viewport_height,
                    result.z / result.w,
                    1.0,
                ]
            })
            .collect();

        // Precompute batch bounding box
        self.bounding_box = Some(self.calculate_bounding_box());

        // Precompute edges for each triangle
        self.edges = self
            .indices
            .iter()
            .map(|&(i0, i1, i2)| {
                let v0 = self.projected_vertices[i0];
                let mut v1 = self.projected_vertices[i1];
                let mut v2 = self.projected_vertices[i2];

                let visible = match self.cull_mode {
                    CullMode::Off => {
                        if self.is_front_facing(&v0, &v1, &v2) {
                            std::mem::swap(&mut v1, &mut v2);
                        }
                        true
                    }
                    CullMode::Front => !self.is_front_facing(&v0, &v1, &v2),
                    CullMode::Back => {
                        if self.is_front_facing(&v0, &v1, &v2) {
                            std::mem::swap(&mut v1, &mut v2);
                            true
                        } else {
                            false
                        }
                    }
                };

                [
                    Edge::new([v0[0], v0[1]], [v1[0], v1[1]], visible),
                    Edge::new([v1[0], v1[1]], [v2[0], v2[1]], visible),
                    Edge::new([v2[0], v2[1]], [v0[0], v0[1]], visible),
                ]
            })
            .collect();
    }

    /// Returns true if the triangle faces to the front
    fn is_front_facing(&self, v0: &[f32; 4], v1: &[f32; 4], v2: &[f32; 4]) -> bool {
        let orientation = (v1[0] - v0[0]) * (v2[1] - v0[1]) - (v1[1] - v0[1]) * (v2[0] - v0[0]);
        orientation > 0.0 // CCW convention for front-facing
    }

    /// Calculate the bounding box for the projected vertices
    fn calculate_bounding_box(&self) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for v in &self.projected_vertices {
            min_x = min_x.min(v[0]); // x coordinate
            max_x = max_x.max(v[0]);
            min_y = min_y.min(v[1]); // y coordinate
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
