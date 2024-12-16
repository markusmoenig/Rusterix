// use crate::{vec4_to_pixel, Pixel, Shader};
// use vek::{Vec2, Vec4};
use crate::SceneBuilder;
use rusterix::{GridShader, Map, Scene, Shader};

pub struct D2PreviewBuilder;

impl SceneBuilder for D2PreviewBuilder {
    fn new() -> Self {
        D2PreviewBuilder
    }

    fn build(&self, map: &Map) -> Scene {
        let mut scene = Scene::empty();
        let mut grid_shader = GridShader::new();

        grid_shader.set_parameter_vec2("offset", map.offset);

        scene.background = Some(Box::new(grid_shader));

        scene
    }
}
