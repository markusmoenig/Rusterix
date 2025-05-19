use crate::{Batch2D, Batch3D, Chunk, CompiledLight, MapMini, Shader, Terrain, Tile};
use rayon::prelude::*;
use theframework::prelude::*;
use vek::{Mat3, Mat4};

/// A scene of 2D and 3D batches which are passed to the rasterizer for rasterization.
pub struct Scene {
    /// Background shader
    pub background: Option<Box<dyn Shader>>,

    /// The lights in the scene
    pub lights: Vec<CompiledLight>,

    /// The lights in the scene
    pub dynamic_lights: Vec<CompiledLight>,

    /// 3D static batches which never change. Only use for scene with no async chunc rendering.
    pub d3_static: Vec<Batch3D>,

    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch3D>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    /// Static 2D batches.
    pub d2_static: Vec<Batch2D>,
    /// 2D dynamic batches which can be updated dynamically.
    pub d2_dynamic: Vec<Batch2D>,

    /// The list of textures which the batches index into.
    pub textures: Vec<Tile>,

    /// The list of textures which the d3_dynamic batches index into.
    pub dynamic_textures: Vec<Tile>,

    /// The current animation frame
    pub animation_frame: usize,

    /// For 2D grid conversion when we dont use a matrix
    pub mapmini: MapMini,

    /// Optional Terrain
    pub terrain: Option<Terrain>,

    /// The build chunks
    pub chunks: FxHashMap<(i32, i32), Chunk>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene::empty()
    }
}

impl Scene {
    // An empty scene
    pub fn empty() -> Self {
        Self {
            background: None,
            lights: vec![],
            dynamic_lights: vec![],
            d3_static: vec![],
            d3_dynamic: vec![],
            d2_static: vec![],
            d2_dynamic: vec![],
            textures: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),
            terrain: None,

            chunks: FxHashMap::default(),
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch2D>, d3: Vec<Batch3D>) -> Self {
        Self {
            background: None,
            lights: vec![],
            dynamic_lights: vec![],
            d3_static: d3,
            d3_dynamic: vec![],
            d2_static: d2,
            d2_dynamic: vec![],
            textures: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),
            terrain: None,

            chunks: FxHashMap::default(),
        }
    }

    /// Sets the background shader using the builder pattern.
    pub fn background(mut self, background: Box<dyn Shader>) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets the background shader using the builder pattern.
    pub fn textures(mut self, textures: Vec<Tile>) -> Self {
        self.textures = textures;
        self
    }

    /// Sets the lights using the builder pattern.
    pub fn lights(mut self, lights: Vec<CompiledLight>) -> Self {
        self.lights = lights;
        self
    }

    /// Increase the animation frame counter.
    pub fn anim_tick(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
    }

    /// Project the batches using the given matrices (which represent the global camera).
    pub fn project(
        &mut self,
        projection_matrix_2d: Option<Mat3<f32>>,
        view_matrix_3d: Mat4<f32>,
        projection_matrix_3d: Mat4<f32>,
        width: f32,
        height: f32,
    ) {
        self.chunks.par_iter_mut().for_each(|chunk| {
            for chunk2d in &mut chunk.1.batches2d {
                chunk2d.project(projection_matrix_2d);
            }
            for chunk3d in &mut chunk.1.batches3d {
                chunk3d.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
            }
        });

        self.d2_static.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d2_dynamic.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
        });

        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
        });

        if let Some(terrain) = &mut self.terrain {
            terrain.chunks.par_iter_mut().for_each(|batch| {
                if let Some(batch) = &mut batch.1.batch {
                    batch.clip_and_project(view_matrix_3d, projection_matrix_3d, width, height);
                }
                if let Some(batch) = &mut batch.1.batch_d2 {
                    batch.project(projection_matrix_2d);
                }
            });
        }
    }

    /// Computes the normals for the static models
    pub fn compute_static_normals(&mut self) {
        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.compute_vertex_normals();
        });
    }

    /// Computes the normals for the dynamic models
    pub fn compute_dynamic_normals(&mut self) {
        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.compute_vertex_normals();
        });
    }
}
