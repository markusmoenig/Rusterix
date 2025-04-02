use crate::prelude::*;
use crate::D2Builder;
use num_traits::zero;
use theframework::prelude::*;
use vek::Vec2;

pub struct ScreenWidget {
    pub builder_d2: D2Builder,

    pub position: Vec2<i32>,
    pub size: Vec2<f32>,

    pub scene: Scene,

    pub buffer: TheRGBABuffer,
    pub offset: Vec2<f32>,

    pub grid_size: f32,
}

impl Default for ScreenWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenWidget {
    pub fn new() -> Self {
        Self {
            builder_d2: D2Builder::new(),

            position: Vec2::zero(),
            size: Vec2::zero(),

            scene: Scene::default(),

            buffer: TheRGBABuffer::default(),
            offset: zero(),

            grid_size: 32.0,
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets) {
        self.scene = self.builder_d2.build(
            map,
            assets,
            Vec2::new(
                self.buffer.dim().width as f32,
                self.buffer.dim().height as f32,
            ),
        );
    }

    pub fn draw(&mut self, map: &Map, time: &TheTime) {
        self.draw_d2(map, time);
    }

    /// Draw the 2D scene.
    pub fn draw_d2(&mut self, _map: &Map, _time: &TheTime) {
        let width = self.buffer.dim().width as usize;
        let height = self.buffer.dim().height as usize;

        let screen_size = Vec2::new(width as f32, height as f32);

        // let translation_matrix = Mat3::<f32>::identity();
        let translation_matrix =
            Mat3::<f32>::translation_2d((screen_size / 2.0 - self.offset).floor());

        let scale_matrix = Mat3::new(
            self.grid_size,
            0.0,
            0.0,
            0.0,
            self.grid_size,
            0.0,
            0.0,
            0.0,
            1.0,
        );
        let transform = translation_matrix * scale_matrix;

        let mut rast = Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity());
        rast.preserve_transparency = true;
        rast.mapmini = self.scene.mapmini.clone();

        rast.rasterize(&mut self.scene, self.buffer.pixels_mut(), width, height, 40);
    }
}
