use crate::Rect;
use vek::{Mat3, Mat4, Vec2, Vec3, Vec4};

pub struct Batch<T> {
    vertices: Vec<T>,

    pub indices: Vec<(usize, usize, usize)>,
    pub uvs: Vec<Vec2<f32>>,

    pub projected_vertices: Vec<T>,
    pub bounding_box: Option<Rect>,
}

impl Batch<Vec2<f32>> {
    /// Constructor for 2D vertices
    pub fn new_2d(
        vertices: Vec<Vec2<f32>>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<Vec2<f32>>,
    ) -> Self {
        Batch {
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
        }
    }

    /// Create a Batch for a rectangle in 2D
    pub fn from_rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            Vec2::new(x, y),                  // Bottom-left
            Vec2::new(x, y + height),         // Top-left
            Vec2::new(x + width, y + height), // Top-right
            Vec2::new(x + width, y),          // Bottom-right
        ];

        let indices = vec![
            (2, 1, 0), // First triangle
            (3, 2, 0), // Second triangle
        ];

        // UV coordinates for a rectangle
        let uvs = vec![
            Vec2::new(0.0, 0.0), // Bottom-left
            Vec2::new(0.0, 1.0), // Top-left
            Vec2::new(1.0, 1.0), // Top-right
            Vec2::new(1.0, 0.0), // Bottom-right
        ];

        // fn is_ccw(v0: Vec2<f32>, v1: Vec2<f32>, v2: Vec2<f32>) -> bool {
        //     let cross_product = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);
        //     cross_product > 0.0
        // }

        // println!("is_ccw {}", is_ccw(vertices[3], vertices[2], vertices[0]));

        Batch::new_2d(vertices, indices, uvs)
    }

    /// Project 2D vertices using a Mat3 transformation matrix
    pub fn project(&mut self, matrix: Option<Mat3<f32>>) {
        if let Some(matrix) = matrix {
            self.projected_vertices = self
                .vertices
                .iter()
                .map(|&v| {
                    // Extend Vec2 to Vec3 manually (homogeneous coordinates)
                    let extended = Vec3::new(v.x, v.y, 1.0);
                    let result = matrix * extended;
                    Vec2::new(result.x, result.y) // Discard the z component
                })
                .collect();
        } else {
            self.projected_vertices = self.vertices.clone();
        }
        self.bounding_box = Some(self.calculate_bounding_box());
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

impl Batch<Vec3<f32>> {
    /// Constructor for 3D vertices
    pub fn new_3d(
        vertices: Vec<Vec3<f32>>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<Vec2<f32>>,
    ) -> Self {
        Batch {
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
        }
    }

    /// Create a Batch for a box in 3D
    pub fn from_box(x: f32, y: f32, z: f32, width: f32, height: f32, depth: f32) -> Self {
        let vertices = vec![
            // Front face
            Vec3::new(x, y, z),                  // Bottom-left-front
            Vec3::new(x + width, y, z),          // Bottom-right-front
            Vec3::new(x + width, y + height, z), // Top-right-front
            Vec3::new(x, y + height, z),         // Top-left-front
            // Back face
            Vec3::new(x, y, z + depth),         // Bottom-left-back
            Vec3::new(x + width, y, z + depth), // Bottom-right-back
            Vec3::new(x + width, y + height, z + depth), // Top-right-back
            Vec3::new(x, y + height, z + depth), // Top-left-back
            // Left face
            Vec3::new(x, y, z),                  // Bottom-left-front
            Vec3::new(x, y + height, z),         // Top-left-front
            Vec3::new(x, y + height, z + depth), // Top-left-back
            Vec3::new(x, y, z + depth),          // Bottom-left-back
            // Right face
            Vec3::new(x + width, y, z),          // Bottom-right-front
            Vec3::new(x + width, y + height, z), // Top-right-front
            Vec3::new(x + width, y + height, z + depth), // Top-right-back
            Vec3::new(x + width, y, z + depth),  // Bottom-right-back
            // Top face
            Vec3::new(x, y + height, z),                 // Top-left-front
            Vec3::new(x + width, y + height, z),         // Top-right-front
            Vec3::new(x + width, y + height, z + depth), // Top-right-back
            Vec3::new(x, y + height, z + depth),         // Top-left-back
            // Bottom face
            Vec3::new(x, y, z),                 // Bottom-left-front
            Vec3::new(x + width, y, z),         // Bottom-right-front
            Vec3::new(x + width, y, z + depth), // Bottom-right-back
            Vec3::new(x, y, z + depth),         // Bottom-left-back
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
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Back face
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Left face
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Right face
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Top face
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
            // Bottom face
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ];

        // fn is_ccw(v0: Vec3<f32>, v1: Vec3<f32>, v2: Vec3<f32>) -> bool {
        //     let edge1 = v1 - v0;
        //     let edge2 = v2 - v0;
        //     let normal = edge1.cross(edge2);
        //     normal.z > 0.0
        // }

        // for (index, (a, b, c)) in indices.iter().enumerate() {
        //     println!(
        //         "is_ccw {}: {}",
        //         index,
        //         is_ccw(vertices[*a], vertices[*b], vertices[*c])
        //     );
        // }

        Batch::new_3d(vertices, indices, uvs)
    }

    /// Project 3D vertices using a Mat4 transformation matrix
    pub fn project(&mut self, matrix: Mat4<f32>, viewport_width: f32, viewport_height: f32) {
        self.projected_vertices = self
            .vertices
            .iter()
            .map(|&v| {
                let extended = Vec4::new(v.x, v.y, v.z, 1.0);
                let result = matrix * extended;
                let w = result.w;
                let mut vec = Vec3::new(result.x / w, result.y / w, result.z / w);

                vec.x = (result.x * 0.5 + 0.5) * viewport_width;
                vec.y = (1.0 - (result.y * 0.5 + 0.5)) * viewport_height;

                vec
            })
            .collect();

        self.bounding_box = Some(self.calculate_bounding_box());
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
