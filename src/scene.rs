use crate::{Batch, Shader};
use rayon::prelude::*;
use vek::{Mat3, Mat4, Vec3, Vec4};

/// A scene of 2D and 3D batches which are passed to the rasterizer for rasterization.
pub struct Scene {
    pub background: Option<Box<dyn Shader>>,

    /// 3D static batches which do not need to be changed, i.e. no animation for textures or the mesh itself.
    pub d3_static: Vec<Batch<Vec4<f32>>>,
    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch<Vec4<f32>>>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    pub d2: Vec<Batch<Vec3<f32>>>,
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
            d3_static: vec![],
            d3_dynamic: vec![],
            d2: vec![],
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch<Vec3<f32>>>, d3: Vec<Batch<Vec4<f32>>>) -> Self {
        Self {
            background: None,
            d3_static: d3,
            d3_dynamic: vec![],
            d2,
        }
    }

    /// Project the batches using the given matrices (which represent the global camera).
    pub fn project(
        &mut self,
        projection_matrix_2d: Option<Mat3<f32>>,
        projection_matrix_3d: Mat4<f32>,
        width: usize,
        height: usize,
    ) {
        self.d2.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d3_static.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_3d, width as f32, height as f32);
        });

        self.d3_dynamic.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_3d, width as f32, height as f32);
        });
    }
}
