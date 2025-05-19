use crate::prelude::*;
use crate::{Edges, Material, Rect, RepeatMode};
use vek::{Mat3, Vec2, Vec3};

use PrimitiveMode::*;
use RepeatMode::*;

/// A batch of 2D vertices, indices and their UVs which make up 2D polygons.
#[derive(Debug, Clone)]
pub struct Batch2D {
    // Render mode: triangles or lines
    pub mode: PrimitiveMode,

    /// 2D vertices which will get projected into 2D space.
    pub vertices: Vec<[f32; 2]>,

    /// The indices of the vertices of the batch.
    pub indices: Vec<(usize, usize, usize)>,

    /// The UVs of the batch.
    pub uvs: Vec<[f32; 2]>,

    /// Projected vertices
    pub projected_vertices: Vec<[f32; 2]>,

    /// 2D Bounding box of the projected vertices of the batch.
    pub bounding_box: Option<Rect>,

    /// Precomputed edges
    pub edges: Vec<Edges>,

    /// RepeatMode, default is ClampXY.
    pub repeat_mode: RepeatMode,

    /// The source of pixels for this batch.
    pub source: PixelSource,

    // Output after clipping and projection
    pub clipped_indices: Vec<(usize, usize, usize)>,
    pub clipped_uvs: Vec<[f32; 2]>,

    /// Transform matrix
    pub transform: Mat3<f32>,

    /// Indicates whether the batch receives lighting. True by default. Turn off for skybox etc.
    pub receives_light: bool,

    /// The material for the batch.
    pub material: Option<Material>,
}

impl Default for Batch2D {
    fn default() -> Self {
        Self::empty()
    }
}

impl Batch2D {
    /// Empty constructor (the default)
    pub fn empty() -> Self {
        Self {
            mode: Triangles,
            vertices: vec![],
            indices: vec![],
            uvs: vec![],
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            repeat_mode: ClampXY,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform: Mat3::identity(),
            receives_light: true,
            material: None,
        }
    }

    /// A new batch
    pub fn new(
        vertices: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
    ) -> Self {
        Self {
            mode: Triangles,
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            repeat_mode: ClampXY,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform: Mat3::identity(),
            receives_light: true,
            material: None,
        }
    }

    /// Create a Batch for a rectangle.
    pub fn from_rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            [x, y],                  // Bottom-left
            [x, y + height],         // Top-left
            [x + width, y + height], // Top-right
            [x + width, y],          // Bottom-right
        ];

        let indices = vec![(0, 1, 2), (0, 2, 3)];

        let uvs = vec![
            [0.0, 0.0], // Top-left
            [0.0, 1.0], // Bottom-left
            [1.0, 1.0], // Bottom-right
            [1.0, 0.0], // Top-right
        ];

        Batch2D::new(vertices, indices, uvs)
    }

    /// Append a rectangle to the existing batch
    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let base_index = self.vertices.len();

        // Add vertices
        self.vertices.extend(vec![
            [x, y],                  // Bottom-left
            [x, y + height],         // Top-left
            [x + width, y + height], // Top-right
            [x + width, y],          // Bottom-right
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
        vertices: Vec<[f32; 2]>,
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
        vertices: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
        wrap_size: f32,
    ) {
        let wrap_vertex = |v: [f32; 2], offset: [f32; 2]| -> [f32; 2] {
            [v[0] + offset[0] * wrap_size, v[1] + offset[1] * wrap_size]
        };

        let offsets = [
            [0.0, 0.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [0.0, 1.0],
            [0.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
        ];

        let mut all_wrapped_vertices = vec![];
        let mut all_wrapped_uvs = vec![];
        let mut all_wrapped_indices = vec![];

        for offset in offsets.iter() {
            let wrapped_vertices: Vec<[f32; 2]> =
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
                [start[0], start[1]],
                [end[0], end[1]],
                [end[0], end[1]],     // Repeated to ensure valid triangles
                [start[0], start[1]], // Repeated to ensure valid triangles
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
                [start[0] - normal[0], start[1] - normal[1]],
                [start[0] + normal[0], start[1] + normal[1]],
                [end[0] + normal[0], end[1] + normal[1]],
                [end[0] - normal[0], end[1] - normal[1]],
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

    /// Add a line which wraps around the wrap_size parameter
    pub fn add_wrapped_line(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        thickness: f32,
        wrap_size: f32,
    ) {
        let wrap_point = |p: Vec2<f32>, offset: [f32; 2]| -> Vec2<f32> {
            Vec2::new(p.x + offset[0] * wrap_size, p.y + offset[1] * wrap_size)
        };

        let offsets = [
            [0.0, 0.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [0.0, 1.0],
            [0.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
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

    /// Sets the repeat mode for the batch using the builder pattern.
    pub fn repeat_mode(mut self, repeat_mode: RepeatMode) -> Self {
        self.repeat_mode = repeat_mode;
        self
    }

    /// Set the source of pixels for this batch.
    pub fn source(mut self, pixel_source: PixelSource) -> Self {
        self.source = pixel_source;
        self
    }

    /// Set the 3D transform matrix for this batch
    pub fn transform(mut self, transform: Mat3<f32>) -> Self {
        self.transform = transform;
        self
    }

    /// Set if the batch receives light
    pub fn receives_light(mut self, receives_light: bool) -> Self {
        self.receives_light = receives_light;
        self
    }

    /// Project 2D vertices using a optional Mat3 transformation matrix
    pub fn project(&mut self, matrix: Option<Mat3<f32>>) {
        if let Some(matrix) = matrix {
            self.projected_vertices = self
                .vertices
                .iter()
                .map(|&v| {
                    let result = matrix * Vec3::new(v[0], v[1], 1.0);
                    [result.x, result.y]
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
