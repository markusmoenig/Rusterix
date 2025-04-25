use crate::{Batch, CompiledLight, Light, LightType, MapMini, Ray, Shader, Terrain, Tile};
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

    /// 3D static batches which do not need to be changed.
    pub d3_static: Vec<Batch<[f32; 4]>>,
    /// 3D dynamic batches which can be updated dynamically.
    pub d3_dynamic: Vec<Batch<[f32; 4]>>,

    /// The 2D batches get rendered on top of the 3D batches (2D game or UI).
    /// Static 2D batches.
    pub d2_static: Vec<Batch<[f32; 2]>>,
    /// 2D dynamic batches which can be updated dynamically.
    pub d2_dynamic: Vec<Batch<[f32; 2]>>,

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

    /// Terrain Batch
    pub terrain_batch: Option<Batch<[f32; 4]>>,
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
            terrain_batch: None,
        }
    }

    // From static 2D and 3D meshes.
    pub fn from_static(d2: Vec<Batch<[f32; 2]>>, d3: Vec<Batch<[f32; 4]>>) -> Self {
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
            terrain_batch: None,
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
        self.d2_static.par_iter_mut().for_each(|batch| {
            batch.project(projection_matrix_2d);
        });

        self.d2_dynamic.par_iter_mut().for_each(|batch| {
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

        if let Some(batch) = &mut self.terrain_batch {
            batch.clip_and_project(
                view_matrix_3d,
                projection_matrix_3d,
                width as f32,
                height as f32,
            );
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

    /// Compiles all lights and returns them.
    pub fn compile_lights(&self, background_color: Option<[u8; 4]>) -> Vec<CompiledLight> {
        let mut cl = vec![];

        for l in &self.lights {
            let mut comp = l.compile();
            if comp.light_type == LightType::Daylight {
                if let Some(background_color) = background_color {
                    comp.color = [
                        background_color[0] as f32 / 255.0,
                        background_color[1] as f32 / 255.0,
                        background_color[2] as f32 / 255.0,
                    ];
                }
            }
            cl.push(comp);
        }

        for l in &self.dynamic_lights {
            cl.push(l.compile());
        }

        cl
    }

    pub fn get_hit_info(&self, _ray: &Ray) {
        // let static_hit = self
        //     .d3_static
        //     .par_iter()
        //     .filter_map(|batch| batch.intersect(ray, false))
        //     .reduce_with(|a, b| if a.t < b.t { a } else { b });

        // let dynamic_hit = self
        //     .d3_dynamic
        //     .par_iter()
        //     .filter_map(|batch| batch.intersect(ray, false))
        //     .reduce_with(|a, b| if a.t < b.t { a } else { b });

        // let terrain_hit = self
        //     .terrain_batch
        //     .as_ref()
        //     .and_then(|batch| batch.intersect(ray, false));

        // println!("static: {:?}, terrain: {:?}", static_hit, terrain_hit);
    }
}
