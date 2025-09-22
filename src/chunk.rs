use crate::{BBox, Batch2D, Batch3D, CompiledLight, Pixel, Texture};
use rusteria::{Program, Rusteria};
use vek::Vec2;

/// A chunk of 2D and 3D batches which make up a Scene.
pub struct Chunk {
    pub origin: Vec2<i32>,
    pub size: i32,
    pub bbox: BBox,

    // Geometry
    pub batches2d: Vec<Batch2D>,
    pub batches3d: Vec<Batch3D>,

    // Terrain
    pub terrain_batch2d: Option<Batch2D>,
    pub terrain_batch3d: Option<Batch3D>,
    pub terrain_texture: Option<Texture>,

    // Lights
    pub lights: Vec<CompiledLight>,

    /// The list of shaders for the Batches
    pub shaders: Vec<Program>,
}

impl Chunk {
    /// Create an empty chunk at the given coordinate.
    pub fn new(origin: Vec2<i32>, size: i32) -> Self {
        let bbox = BBox::from_pos_size(origin.map(|v| v as f32), Vec2::broadcast(size as f32));
        Self {
            origin,
            size,
            bbox,
            batches2d: vec![],
            batches3d: vec![],
            terrain_batch2d: None,
            terrain_batch3d: None,
            terrain_texture: None,
            lights: vec![],
            shaders: vec![],
        }
    }

    /// Add a shader
    pub fn add_shader(&mut self, code: &str) -> Option<usize> {
        if code.is_empty() {
            return None;
        };

        let mut rs: Rusteria = Rusteria::default();
        let _module = match rs.parse_str(code) {
            Ok(module) => match rs.compile(&module) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error compiling module: {e}");
                    return None;
                }
            },
            Err(e) => {
                eprintln!("Error parsing module: {e}");
                return None;
            }
        };

        let index = self.shaders.len();
        self.shaders.push(rs.context.program.clone());

        Some(index)
    }

    /// Sample the baked terrain texture at the given world position
    pub fn sample_terrain_texture(&self, world_pos: Vec2<f32>, scale: Vec2<f32>) -> Pixel {
        let local_x = (world_pos.x / scale.x) - self.origin.x as f32;
        let local_y = (world_pos.y / scale.y) - self.origin.y as f32;

        if let Some(texture) = &self.terrain_texture {
            let pixels_per_tile = texture.width as i32 / self.size;

            let pixel_x = local_x * pixels_per_tile as f32;
            let pixel_y = local_y * pixels_per_tile as f32;

            let px = pixel_x.floor().clamp(0.0, texture.width as f32 - 1.0) as u32;
            let py = pixel_y.floor().clamp(0.0, texture.height as f32 - 1.0) as u32;

            return texture.get_pixel(px, py);
        }
        [0, 0, 0, 0]
    }
}
