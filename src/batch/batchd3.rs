use crate::prelude::*;
use crate::wavefront::Wavefront;
use vek::{Mat4, Vec4};

use CullMode::*;
use PrimitiveMode::*;
use RepeatMode::*;
use SampleMode::*;

/// A batch of 4D vertices, indices and their UVs which make up a 3D mesh.
impl Batch<[f32; 4]> {
    /// Empty constructor
    pub fn emptyd3() -> Self {
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
        }
    }

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
            clipped_indices: vec![],
            clipped_uvs: vec![],
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
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            // Back face
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            // Left face
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            // Right face
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            // Top face
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
            // Bottom face
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0],
        ];

        Batch::new_3d(vertices, indices, uvs)
    }

    /// Adds a wall between two ground vertices with a specified height
    pub fn add_wall(&mut self, v1: [f32; 3], v2: [f32; 3], height: f32) {
        let base_index = self.vertices.len();

        // Define the vertices for the wall
        self.vertices.push([v1[0], v1[1], v1[2], 1.0]); // Bottom-left
        self.vertices.push([v2[0], v2[1], v2[2], 1.0]); // Bottom-right
        self.vertices.push([v2[0], v2[1] + height, v2[2], 1.0]); // Top-right
        self.vertices.push([v1[0], v1[1] + height, v1[2], 1.0]); // Top-left

        // Define the UVs for the wall
        self.uvs.push([0.0, 0.0]);
        self.uvs.push([1.0, 0.0]);
        self.uvs.push([1.0, 1.0]);
        self.uvs.push([0.0, 1.0]);

        // Define the indices for the wall (two triangles)
        self.indices
            .push((base_index, base_index + 1, base_index + 2)); // Bottom-right triangle
        self.indices
            .push((base_index, base_index + 2, base_index + 3)); // Top-left triangle
    }

    /// Add a set of geometry to the batch.
    pub fn add(
        &mut self,
        vertices: Vec<[f32; 4]>,
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
    pub fn clip_and_project(
        &mut self,
        view_matrix: Mat4<f32>,
        projection_matrix: Mat4<f32>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        let mut view_space_vertices: Vec<[f32; 4]> = self
            .vertices
            .iter()
            .map(|&v| {
                let v = view_matrix * Vec4::new(v[0], v[1], v[2], v[3]);
                [v.x, v.y, v.z, v.w]
            })
            .collect();

        // Near plane in camera space
        let near_plane = 0.1;

        // Initialize clipped indices and UVs with the original
        self.clipped_indices = self.indices.clone();
        self.clipped_uvs = self.uvs.clone();

        // List of new vertices and their corresponding UVs
        let mut new_vertices = Vec::new();
        let mut new_uvs = Vec::new();

        // Visibility flags for edges
        let mut edge_visibility = vec![true; self.indices.len() * 3];
        // let mut added_vertex_indices: FxHashSet<usize> = FxHashSet::default();

        // Iterate over triangles
        for (triangle_idx, &(i0, i1, i2)) in self.indices.iter().enumerate() {
            let v0 = view_space_vertices[i0];
            let v1 = view_space_vertices[i1];
            let v2 = view_space_vertices[i2];
            let uv0 = self.uvs[i0];
            let uv1 = self.uvs[i1];
            let uv2 = self.uvs[i2];

            let is_v0_inside = v0[2] >= near_plane;
            let is_v1_inside = v1[2] >= near_plane;
            let is_v2_inside = v2[2] >= near_plane;

            if is_v0_inside && is_v1_inside && is_v2_inside {
                // All vertices are inside the near plane, keep the triangle
                continue;
            }

            edge_visibility[triangle_idx * 3] = false;
            edge_visibility[triangle_idx * 3 + 1] = false;
            edge_visibility[triangle_idx * 3 + 2] = false;

            if !is_v0_inside && !is_v1_inside && !is_v2_inside {
                // All vertices are outside, continue
                continue;
            }

            // Mixed case: Calculate intersections and append new vertices
            let vertices = [(v0, uv0, i0), (v1, uv1, i1), (v2, uv2, i2)];
            let mut clipped_indices = Vec::new();
            let mut new_edge_visibility = Vec::new();

            for i in 0..3 {
                let (current, uv_current, _idx_current) = vertices[i];
                let (next, uv_next, _idx_next) = vertices[(i + 1) % 3];

                if current[2] >= near_plane {
                    new_vertices.push(current);
                    // clipped_indices.push(idx_current);
                    clipped_indices.push(self.vertices.len() + new_vertices.len() - 1);
                    new_uvs.push(uv_current);
                    new_edge_visibility.push(true);
                }

                if (current[2] >= near_plane) != (next[2] >= near_plane) {
                    // Edge intersects the near plane, calculate intersection
                    let t = (near_plane - current[2]) / (next[2] - current[2]);
                    let intersection = [
                        current[0] + t * (next[0] - current[0]),
                        current[1] + t * (next[1] - current[1]),
                        current[2] + t * (next[2] - current[2]),
                        current[3] + t * (next[3] - current[3]),
                    ];
                    let interpolated_uv = [
                        uv_current[0] + t * (uv_next[0] - uv_current[0]),
                        uv_current[1] + t * (uv_next[1] - uv_current[1]),
                    ];

                    new_vertices.push(intersection);
                    new_uvs.push(interpolated_uv);
                    clipped_indices.push(self.vertices.len() + new_vertices.len() - 1);
                    new_edge_visibility.push(true);
                }
            }

            // Add new triangles to clipped indices
            for i in 1..clipped_indices.len() - 1 {
                self.clipped_indices.push((
                    clipped_indices[0],
                    clipped_indices[i],
                    clipped_indices[i + 1],
                ));
            }

            edge_visibility.extend(new_edge_visibility);
        }

        // Extend the vertex and UV lists with new vertices
        view_space_vertices.extend(new_vertices);
        self.clipped_uvs.extend(new_uvs);

        // Perform projection
        self.projected_vertices = view_space_vertices
            .iter()
            .map(|&v| {
                let result = projection_matrix * Vec4::new(v[0], v[1], v[2], v[3]);
                let w = result.w;
                [
                    ((result.x / w) * 0.5 + 0.5) * viewport_width,
                    ((-result.y / w) * 0.5 + 0.5) * viewport_height,
                    result.z / w,
                    result.w,
                ]
            })
            .collect();

        // Precompute batch bounding box
        self.bounding_box = Some(self.calculate_bounding_box());

        // Update edges
        self.edges = self
            .clipped_indices
            .iter()
            .enumerate()
            .map(|(triangle_idx, &(i0, i1, i2))| {
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

                let edge0_visible = edge_visibility
                    .get(triangle_idx * 3)
                    .copied()
                    .unwrap_or(true)
                    && visible;
                // let edge1_visible = edge_visibility
                //     .get(triangle_idx * 3 + 1)
                //     .copied()
                //     .unwrap_or(true);
                // let edge2_visible = edge_visibility
                //     .get(triangle_idx * 3 + 2)
                //     .copied()
                //     .unwrap_or(true);

                [
                    Edge::new(&[v0[0], v0[1]], &[v1[0], v1[1]], edge0_visible),
                    Edge::new(&[v1[0], v1[1]], &[v2[0], v2[1]], edge0_visible),
                    Edge::new(&[v2[0], v2[1]], &[v0[0], v0[1]], edge0_visible),
                ]
            })
            .collect();
        // Precompute edges for each triangle
        /*
        self.edges = self
            .indices
            .iter()
            .map(|&(i0, i1, i2)| {
                let v0 = self.projected_vertices[i0];
                let mut v1 = self.projected_vertices[i1];
                let mut v2 = self.projected_vertices[i2];

                /*
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
                };*/

                let visible = true;

                [
                    Edge::new(&[v0[0], v0[1]], &[v1[0], v1[1]], visible),
                    Edge::new(&[v1[0], v1[1]], &[v2[0], v2[1]], visible),
                    Edge::new(&[v2[0], v2[1]], &[v0[0], v0[1]], visible),
                ]
            })
            .collect();
        */
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
