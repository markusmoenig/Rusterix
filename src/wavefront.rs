use crate::Batch;
use vek::{Vec2, Vec3, Vec4};

#[derive(Clone, Debug)]
pub struct Wavefront {
    pub vertices: Vec<Vec4<f32>>, // 4D vertices for compatibility with Batch
    pub texture_coords: Vec<Vec2<f32>>, // Texture coordinates
    pub normals: Vec<Vec3<f32>>,  // Normals
    pub indices: Vec<(usize, usize, usize)>, // Triangle indices
}

impl Wavefront {
    /// Create a new Wavefront object.
    pub fn new(
        vertices: Vec<Vec4<f32>>,
        indices: Vec<(usize, usize, usize)>,
        normals: Vec<Vec3<f32>>,
        texture_coords: Vec<Vec2<f32>>,
    ) -> Self {
        Wavefront {
            vertices,
            indices,
            normals,
            texture_coords,
        }
    }

    /// Parse an OBJ file from a given file path.
    pub fn parse_file(file: String) -> Self {
        let contents = std::fs::read_to_string(file).expect("Failed to read the file");
        Wavefront::parse_string(contents)
    }

    /// Parse an OBJ file from a given string.
    pub fn parse_string(contents: String) -> Self {
        let lines = contents.lines();
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut texture_coords = Vec::new();
        let mut indices = Vec::new();

        for line in lines {
            let trimmed = line.trim();

            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue; // Skip comments and empty lines
            }

            if trimmed.starts_with("v ") {
                let mut items = trimmed.split_ascii_whitespace();
                items.next().unwrap(); // Skip "v"
                let x: f32 = items.next().unwrap().parse().unwrap();
                let y: f32 = items.next().unwrap().parse().unwrap();
                let z: f32 = items.next().unwrap().parse().unwrap();
                // Convert Vec3<f32> to Vec4<f32> by appending 1.0 as the w component
                vertices.push(Vec4::new(x, y, z, 1.0));
            } else if trimmed.starts_with("vn ") {
                let mut items = trimmed.split_ascii_whitespace();
                items.next().unwrap(); // Skip "vn"
                let x: f32 = items.next().unwrap().parse().unwrap();
                let y: f32 = items.next().unwrap().parse().unwrap();
                let z: f32 = items.next().unwrap().parse().unwrap();
                normals.push(Vec3::new(x, y, z));
            } else if trimmed.starts_with("vt ") {
                let mut items = trimmed.split_ascii_whitespace();
                items.next().unwrap(); // Skip "vt"
                let u: f32 = items.next().unwrap().parse().unwrap();
                let v: f32 = items.next().unwrap().parse().unwrap();
                texture_coords.push(Vec2::new(u, v));
            } else if trimmed.starts_with("f ") {
                let mut items = trimmed.split_ascii_whitespace();
                items.next().unwrap(); // Skip "f"

                // Parse three vertices for a triangle
                let parse_face = |face_str: &str| -> usize {
                    let mut parts = face_str.split('/');
                    parts.next().unwrap().parse::<usize>().unwrap() - 1
                };

                let v0 = parse_face(items.next().unwrap());
                let v1 = parse_face(items.next().unwrap());
                let v2 = parse_face(items.next().unwrap());

                indices.push((v0, v1, v2));
            }
        }

        Wavefront::new(vertices, indices, normals, texture_coords)
    }

    /// Convert the Wavefront object into a Batch for rendering.
    pub fn to_batch(self) -> Batch<Vec4<f32>> {
        let uvs = if self.texture_coords.is_empty() {
            // Generate default UVs if none exist
            self.vertices.iter().map(|v| Vec2::new(v.x, v.y)).collect()
        } else {
            // Map texture coordinates
            self.texture_coords
        };

        Batch::new_3d(self.vertices, self.indices, uvs)
    }
}
