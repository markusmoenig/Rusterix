use crate::{Batch, CompiledLight, Light, MapMini, Shader, Tile};
use rayon::prelude::*;
use vek::{Mat3, Mat4};

/// A scene of 2D and 3D batches which are passed to the rasterizer for rasterization.
pub struct Scene {
    /// Background shader
    pub background: Option<Box<dyn Shader>>,

    /// The lights in the scene
    pub lights: Vec<Light>,

    /// The lights in the scene
    pub dynamic_lights: Vec<Light>,

    /// 3D static batches which do not need to be changed, i.e. no animation for textures or the mesh itself.
    pub d3_static: Vec<Batch<[f32; 4]>>,
    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch<[f32; 4]>>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    pub d2: Vec<Batch<[f32; 3]>>,

    /// The list of textures which the batches index into.
    pub textures: Vec<Tile>,

    /// The list of textures which the d3_dynamic batches index into.
    pub dynamic_textures: Vec<Tile>,

    /// The current animation frame
    pub animation_frame: usize,

    /// For 2D grid conversion when we dont use a matrix
    pub mapmini: MapMini,
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
            d2: vec![],
            textures: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch<[f32; 3]>>, d3: Vec<Batch<[f32; 4]>>) -> Self {
        Self {
            background: None,
            lights: vec![],
            dynamic_lights: vec![],
            d3_static: d3,
            d3_dynamic: vec![],
            d2,
            textures: vec![],
            dynamic_textures: vec![],

            animation_frame: 1,

            mapmini: MapMini::default(),
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
    pub fn lights(mut self, lights: Vec<Light>) -> Self {
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

    /// Compiles all lights and returns them.
    pub fn compile_lights(&self) -> Vec<CompiledLight> {
        let mut cl = vec![];

        for l in &self.lights {
            cl.push(l.compile());
        }

        for l in &self.dynamic_lights {
            cl.push(l.compile());
        }

        cl
    }
}
