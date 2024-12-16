// use crate::{vec4_to_pixel, Pixel, Shader};
use crate::SceneBuilder;
use crate::{GridShader, Map, Scene, Shader};
use vek::Vec2;

pub struct D2PreviewBuilder;

impl SceneBuilder for D2PreviewBuilder {
    fn new() -> Self {
        D2PreviewBuilder
    }

    fn build(&self, map: &Map) -> Scene {
        let mut scene = Scene::empty();
        let mut grid_shader = GridShader::new();

        grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));

        scene.background = Some(Box::new(grid_shader));

        scene
    }
}
