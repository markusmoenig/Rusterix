// use crate::PrimitiveMode::*;
use crate::SceneBuilder;
use crate::Texture;
use crate::{Batch, Map, Rasterizer, Scene, Tile};
use theframework::prelude::*;
use vek::Vec2;

pub struct D2MaterialBuilder {}

impl SceneBuilder for D2MaterialBuilder {
    fn new() -> Self {
        Self {}
    }

    fn build_texture(&self, map: &Map, _tiles: &FxHashMap<Uuid, Tile>, texture: &mut Texture) {
        let mut textures = vec![];
        let mut batches: Vec<Batch<[f32; 3]>> = vec![];
        let size = texture.width;

        let to_local = |vertex: &[f32; 2]| -> Vec2<f32> {
            let tx = (((vertex[0] + 5.0) / 10.0) * size as f32).floor();
            let ty = (((vertex[1] + 5.0) / 10.0) * size as f32).floor();
            Vec2::new(tx, ty)
        };

        for sector in &map.sectors {
            if let Some(geo) = sector.generate_geometry(map) {
                let mut vertices: Vec<[f32; 3]> = vec![];
                let mut uvs: Vec<[f32; 2]> = vec![];
                let bbox = sector.bounding_box(map);

                for vertex in &geo.0 {
                    let local = to_local(vertex);

                    let texture_scale = 1.0;
                    let uv = [
                        (vertex[0] - bbox.0.x) / texture_scale,
                        (vertex[1] - bbox.0.y) / texture_scale,
                    ];
                    uvs.push(uv);
                    vertices.push([local.x, local.y, 1.0]);
                }

                let texture_index = textures.len();
                let texture = Texture::from_color([128, 128, 128, 255]);

                let mut batch = Batch::emptyd2()
                    .repeat_mode(crate::RepeatMode::RepeatXY)
                    .texture_index(texture_index);

                batch.add_wrapped(vertices, geo.1, uvs, size as f32);
                batches.push(batch);
                textures.push(texture);
            }
        }

        // Add Lines
        for linedef in &map.linedefs {
            if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                let start_pos = to_local(&[start_vertex.x, start_vertex.y]);
                if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                    let end_pos = to_local(&[end_vertex.x, end_vertex.y]);

                    let texture_index = textures.len();
                    let texture = Texture::from_color([128, 128, 128, 255]);

                    let mut batch = Batch::emptyd2()
                        .repeat_mode(crate::RepeatMode::RepeatXY)
                        .texture_index(texture_index)
                        .mode(crate::PrimitiveMode::Lines);
                    batch.add_wrapped_line(start_pos, end_pos, 1.0, size as f32);
                    batches.push(batch);
                    textures.push(texture);
                }
            }
        }

        let mut scene = Scene::empty();
        scene.d2 = batches;
        scene.textures = textures;

        Rasterizer::setup(None, Mat4::identity(), Mat4::identity()).rasterize(
            &mut scene,
            &mut texture.data,
            texture.width,
            texture.height,
            10,
        );
    }
}
