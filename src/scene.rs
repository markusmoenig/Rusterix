use crate::{Batch, Light, Shader, Texture};
use rayon::prelude::*;
use vek::{Mat3, Mat4};

/// A scene of 2D and 3D batches which are passed to the rasterizer for rasterization.
pub struct Scene {
    /// Background shader
    pub background: Option<Box<dyn Shader>>,

    /// The lights in the scene
    pub lights: Vec<Light>,

    /// 3D static batches which do not need to be changed, i.e. no animation for textures or the mesh itself.
    pub d3_static: Vec<Batch<[f32; 4]>>,
    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch<[f32; 4]>>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    pub d2: Vec<Batch<[f32; 3]>>,

    /// The list of textures which the batches index into.
    pub textures: Vec<Texture>,
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
            d3_static: vec![],
            d3_dynamic: vec![],
            d2: vec![],
            textures: vec![],
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch<[f32; 3]>>, d3: Vec<Batch<[f32; 4]>>) -> Self {
        Self {
            background: None,
            lights: vec![],
            d3_static: d3,
            d3_dynamic: vec![],
            d2,
            textures: vec![],
        }
    }

    /// Sets the background shader using the builder pattern.
    pub fn background(mut self, background: Box<dyn Shader>) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets the background shader using the builder pattern.
    pub fn textures(mut self, textures: Vec<Texture>) -> Self {
        self.textures = textures;
        self
    }

    /// Sets the lights using the builder pattern.
    pub fn lights(mut self, lights: Vec<Light>) -> Self {
        self.lights = lights;
        self
    }

    /// Project the batches using the given matrices (which represent the global camera).
    pub fn project(
        &mut self,
        projection_matrix_2d: Option<Mat3<f32>>,
        view_matrix_3d: Mat4<f32>,
        projection_matrix_3d: Mat4<f32>,
        width: usize,
        height: usize,
    ) {
        self.d2.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(
                view_matrix_3d,
                projection_matrix_3d,
                width as f32,
                height as f32,
            );
        });

        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.clip_and_project(
                view_matrix_3d,
                projection_matrix_3d,
                width as f32,
                height as f32,
            );
        });
    }
}
