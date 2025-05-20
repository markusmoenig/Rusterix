use crate::prelude::*;
use crate::wavefront::Wavefront;
use crate::{Edges, Material, Rect, RepeatMode};
use crate::{HitInfo, Ray};
use bvh::aabb::{Aabb, Bounded};
use nalgebra::Point3;
use vek::{Mat4, Vec2, Vec3, Vec4};

use CullMode::*;
use PrimitiveMode::*;
use RepeatMode::*;

/// A batch of vertices, indices and their UVs which make up 3D polygons.
#[derive(Debug, Clone)]
pub struct Batch3D {
    // Render mode: triangles or lines
    pub mode: PrimitiveMode,

    /// 2D or 3D input vertices which will get projected into 2D space. 2D and 3D vertices expect 3D and 4D vecs with the last component set to 1.0.
    pub vertices: Vec<[f32; 4]>,

    /// The indices of the vertices of the batch.
    pub indices: Vec<(usize, usize, usize)>,

    /// The UVs of the batch.
    pub uvs: Vec<[f32; 2]>,

    /// Projected vertices
    pub projected_vertices: Vec<[f32; 4]>,

    /// 2D Bounding box of the projected vertices of the batch.
    pub bounding_box: Option<Rect>,

    /// Precomputed edges
    pub edges: Vec<Edges>,

    /// RepeatMode, default is ClampXY.
    pub repeat_mode: RepeatMode,

    /// CullMode, default is None.
    pub cull_mode: CullMode,

    /// The source of pixels for this batch.
    pub source: PixelSource,

    /// Output after clipping and projection
    pub clipped_indices: Vec<(usize, usize, usize)>,
    /// Output after clipping and projection
    pub clipped_uvs: Vec<[f32; 2]>,

    /// 3D Transform matrix
    pub transform_3d: Mat4<f32>,

    /// Indicates whether the batch receives lighting. True by default. Turn off for skybox etc.
    pub receives_light: bool,

    /// Normals
    pub normals: Vec<Vec3<f32>>,

    /// Clipped normals
    pub clipped_normals: Vec<Vec3<f32>>,

    // Material
    pub material: Option<Material>,
}

/// A batch of 4D vertices, indices and their UVs which make up a 3D mesh.
impl Batch3D {
    /// Empty constructor
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
            cull_mode: Off,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform_3d: Mat4::identity(),
            receives_light: true,
            normals: vec![],
            clipped_normals: vec![],
            material: None,
        }
    }

    pub fn new(
        vertices: Vec<[f32; 4]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
    ) -> Self {
        Batch3D {
            mode: Triangles,
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            repeat_mode: ClampXY,
            cull_mode: Off,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform_3d: Mat4::identity(),
            receives_light: true,
            normals: vec![],
            clipped_normals: vec![],
            material: None,
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

        Batch3D::new(vertices, indices, uvs)
    }

    /// Sets the background shader using the builder pattern.
    pub fn material(mut self, material: Material) -> Self {
        self.material = Some(material);
        self
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

    /// Set the source of pixels for this batch.
    pub fn source(mut self, pixel_source: PixelSource) -> Self {
        self.source = pixel_source;
        self
    }

    /// Set the 3D transform matrix for this batch
    pub fn transform(mut self, transform: Mat4<f32>) -> Self {
        self.transform_3d = transform;
        self
    }

    /// Set if this batch receives light.
    pub fn receives_light(mut self, receives_light: bool) -> Self {
        self.receives_light = receives_light;
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
                let v = view_matrix * self.transform_3d * Vec4::new(v[0], v[1], v[2], v[3]);
                [v.x, v.y, v.z, v.w]
            })
            .collect();

        // Near plane in camera space
        let near_plane = 0.1;

        // Initialize clipped indices and UVs with the original
        self.clipped_indices = self.indices.clone();
        self.clipped_uvs = self.uvs.clone();
        self.clipped_normals = self.normals.clone();

        // List of new vertices and their corresponding UVs and normals
        let mut new_vertices = Vec::new();
        let mut new_uvs = Vec::new();
        let mut new_normals = Vec::new();

        // Visibility flags for edges
        let mut edge_visibility = vec![true; self.indices.len()];

        // Iterate over triangles
        for (triangle_idx, &(i0, i1, i2)) in self.indices.iter().enumerate() {
            let v0 = view_space_vertices[i0];
            let v1 = view_space_vertices[i1];
            let v2 = view_space_vertices[i2];
            let uv0 = self.uvs[i0];
            let uv1 = self.uvs[i1];
            let uv2 = self.uvs[i2];
            let n0 = self.normals[i0];
            let n1 = self.normals[i1];
            let n2 = self.normals[i2];

            let is_v0_inside = v0[2] <= -near_plane;
            let is_v1_inside = v1[2] <= -near_plane;
            let is_v2_inside = v2[2] <= -near_plane;

            if is_v0_inside && is_v1_inside && is_v2_inside {
                // All vertices are inside the near plane, keep the triangle
                continue;
            }

            edge_visibility[triangle_idx] = false;

            if !is_v0_inside && !is_v1_inside && !is_v2_inside {
                // All vertices are outside, continue
                continue;
            }

            // Mixed case: Calculate intersections and append new vertices
            let vertices = [(v0, uv0, n0), (v1, uv1, n1), (v2, uv2, n2)];
            let mut clipped_indices = Vec::new();
            let mut new_edge_visibility = Vec::new();

            for i in 0..3 {
                let (current, uv_current, n_current) = vertices[i];
                let (next, uv_next, n_next) = vertices[(i + 1) % 3];

                if current[2] <= -near_plane {
                    new_vertices.push(current);
                    new_uvs.push(uv_current);
                    new_normals.push(n_current);
                    clipped_indices.push(self.vertices.len() + new_vertices.len() - 1);
                    new_edge_visibility.push(true);
                }

                if (current[2] <= -near_plane) != (next[2] <= -near_plane) {
                    // Edge intersects the near plane, calculate intersection
                    let t = (-near_plane - current[2]) / (next[2] - current[2]);
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
                    let interpolated_normal = (n_current * (1.0 - t) + n_next * t).normalized();

                    new_vertices.push(intersection);
                    new_uvs.push(interpolated_uv);
                    new_normals.push(interpolated_normal);
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

        // Extend the vertex, UV and normal lists with new values
        view_space_vertices.extend(new_vertices);
        self.clipped_uvs.extend(new_uvs);
        self.clipped_normals.extend(new_normals);

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

                let edge_visible =
                    edge_visibility.get(triangle_idx).copied().unwrap_or(true) && visible;

                crate::Edges::new(
                    [[v0[0], v0[1]], [v1[0], v1[1]], [v2[0], v2[1]]],
                    [[v1[0], v1[1]], [v2[0], v2[1]], [v0[0], v0[1]]],
                    edge_visible,
                )
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

    /// Compute smooth vertex normals for the mesh in-place.
    pub fn compute_vertex_normals(&mut self) {
        self.normals = vec![Vec3::zero(); self.vertices.len()];
        let mut counts = vec![0u32; self.vertices.len()];

        for &(i0, i1, i2) in &self.indices {
            let p0 = Vec3::new(
                self.vertices[i0][0],
                self.vertices[i0][1],
                self.vertices[i0][2],
            );
            let p1 = Vec3::new(
                self.vertices[i1][0],
                self.vertices[i1][1],
                self.vertices[i1][2],
            );
            let p2 = Vec3::new(
                self.vertices[i2][0],
                self.vertices[i2][1],
                self.vertices[i2][2],
            );

            let normal = (p1 - p0).cross(p2 - p0).normalized();

            self.normals[i0] += normal;
            self.normals[i1] += normal;
            self.normals[i2] += normal;

            counts[i0] += 1;
            counts[i1] += 1;
            counts[i2] += 1;
        }

        for (n, &count) in self.normals.iter_mut().zip(counts.iter()) {
            if count > 0 {
                *n /= count as f32;
                *n = n.normalized();
            }
        }
    }

    /// Returns a new Batch3D with computed smooth vertex normals.
    pub fn with_computed_normals(&self) -> Self {
        let mut new = self.clone();

        new.normals = vec![Vec3::zero(); new.vertices.len()];
        let mut counts = vec![0u32; new.vertices.len()];

        for &(i0, i1, i2) in &new.indices {
            let p0 = Vec3::from_slice(&new.vertices[i0][..3]);
            let p1 = Vec3::from_slice(&new.vertices[i1][..3]);
            let p2 = Vec3::from_slice(&new.vertices[i2][..3]);

            let normal = (p1 - p0).cross(p2 - p0).normalized();

            new.normals[i0] += normal;
            new.normals[i1] += normal;
            new.normals[i2] += normal;

            counts[i0] += 1;
            counts[i1] += 1;
            counts[i2] += 1;
        }

        for (n, &count) in new.normals.iter_mut().zip(counts.iter()) {
            if count > 0 {
                *n /= count as f32;
                *n = n.normalized();
            }
        }

        new
    }

    /// Perform a brute-force ray intersection against all triangles in the batch.
    /// If `simplified` is true, skips UV and normal computation (useful for shadow rays).
    pub fn intersect(&self, ray: &Ray, simplified: bool) -> Option<HitInfo> {
        let local_origin = ray.origin;
        let local_dir = ray.dir.normalized();

        let mut closest: Option<HitInfo> = None;

        for (i, &(i0, i1, i2)) in self.indices.iter().enumerate() {
            let p0 = Vec3::new(
                self.vertices[i0][0],
                self.vertices[i0][1],
                self.vertices[i0][2],
            );
            let p1 = Vec3::new(
                self.vertices[i1][0],
                self.vertices[i1][1],
                self.vertices[i1][2],
            );
            let p2 = Vec3::new(
                self.vertices[i2][0],
                self.vertices[i2][1],
                self.vertices[i2][2],
            );

            let edge1 = p1 - p0;
            let edge2 = p2 - p0;
            let h = local_dir.cross(edge2);
            let a = edge1.dot(h);

            if a.abs() < 1e-6 {
                continue;
            }

            let f = 1.0 / a;
            let s = local_origin - p0;
            let u = f * s.dot(h);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }

            let q = s.cross(edge1);
            let v = f * local_dir.dot(q);
            if v < 0.0 || u + v > 1.0 {
                continue;
            }

            let t = f * edge2.dot(q);
            if t > 1e-4 {
                match &closest {
                    Some(c) if t >= c.t => {}
                    _ => {
                        if simplified {
                            closest = Some(HitInfo {
                                t,
                                uv: Vec2::zero(),
                                triangle_index: i,
                                ..Default::default()
                            });
                        } else {
                            let w = 1.0 - u - v;
                            let uv0 = self.uvs[i0];
                            let uv1 = self.uvs[i1];
                            let uv2 = self.uvs[i2];
                            let uv = Vec2::new(
                                w * uv0[0] + u * uv1[0] + v * uv2[0],
                                w * uv0[1] + u * uv1[1] + v * uv2[1],
                            );

                            let mut normal = if !self.normals.is_empty() {
                                let n0 = self.normals[i0];
                                let n1 = self.normals[i1];
                                let n2 = self.normals[i2];
                                (n0 * w + n1 * u + n2 * v).normalized()
                            } else {
                                (p1 - p0).cross(p2 - p0).normalized()
                            };

                            // Make sure normal faces the camera
                            if normal.dot(ray.dir) > 0.0 {
                                normal = -normal;
                            }

                            closest = Some(HitInfo {
                                t,
                                uv,
                                triangle_index: i,
                                normal: Some(normal),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        closest
    }
}

impl Bounded<f32, 3> for Batch3D {
    fn aabb(&self) -> Aabb<f32, 3> {
        let mut aabb = Aabb::empty();

        for v in &self.vertices {
            let p = Point3::new(v[0], v[1], v[2]);
            aabb = aabb.grow(&p);
        }

        aabb
    }
}
