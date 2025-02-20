// use crate::PrimitiveMode::*;

use crate::Texture;
use crate::{Batch, Map, Rasterizer, Scene, Tile, Value};
use theframework::prelude::*;
use vek::Vec2;

pub struct D2MaterialBuilder {}

impl Default for D2MaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2MaterialBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build_texture(&self, map: &Map, tiles: &FxHashMap<Uuid, Tile>, texture: &mut Texture) {
        let mut textures = vec![];
        let mut batches: Vec<Batch<[f32; 3]>> = vec![];
        let size = texture.width;

        let to_local = |vertex: &[f32; 2]| -> Vec2<f32> {
            let tx = (((vertex[0] + 5.0) / 10.0) * size as f32).floor();
            let ty = (((vertex[1] + 5.0) / 10.0) * size as f32).floor();
            Vec2::new(tx, ty)
        };

        let sorted_sectors = map.sorted_sectors_by_area();
        for sector in &sorted_sectors {
            if let Some(geo) = sector.generate_geometry(map) {
                let mut vertices: Vec<[f32; 3]> = vec![];
                let mut uvs: Vec<[f32; 2]> = vec![];
                let bbox = sector.bounding_box(map);

                let repeat = false;
                let index = 0;

                if let Some(Value::Source(pixelsource)) = sector.properties.get("floor_source") {
                    if let Some(tile) = pixelsource.to_tile(tiles, size, &sector.properties) {
                        for vertex in &geo.0 {
                            let local = to_local(vertex);

                            if !repeat {
                                let uv = [
                                    (tile.uvs[index].x as f32
                                        + ((vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x)
                                            * tile.uvs[index].z as f32))
                                        / size as f32,
                                    ((tile.uvs[index].y as f32
                                        + (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y)
                                            * tile.uvs[index].w as f32)
                                        / size as f32),
                                ];
                                uvs.push(uv);
                            } else {
                                let texture_scale = 1.0;
                                let uv = [
                                    (vertex[0] - bbox.min.x) / texture_scale,
                                    (vertex[1] - bbox.min.y) / texture_scale,
                                ];
                                uvs.push(uv);
                            }
                            vertices.push([local.x, local.y, 1.0]);
                        }

                        let texture_index = textures.len();

                        let mut batch = Batch::emptyd2()
                            .repeat_mode(crate::RepeatMode::RepeatXY)
                            .texture_index(texture_index);

                        batch.add_wrapped(vertices, geo.1, uvs, size as f32);
                        batches.push(batch);
                        textures.push(tile.clone());
                    }
                }
            }
        }

        // Add Lines
        for linedef in &map.linedefs {
            if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
                if let Some(Value::Source(pixelsource)) = linedef.properties.get("row1_source") {
                    if let Some(tile) = pixelsource.to_tile(tiles, size, &linedef.properties) {
                        if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                            let start_pos = to_local(&[start_vertex.x, start_vertex.y]);
                            if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                let end_pos = to_local(&[end_vertex.x, end_vertex.y]);

                                let texture_index = textures.len();
                                let width =
                                    linedef.properties.get_float_default("material_width", 1.0);

                                let mut batch = Batch::emptyd2()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .texture_index(texture_index);
                                batch.add_wrapped_line(start_pos, end_pos, width, size as f32);
                                batches.push(batch);
                                textures.push(tile);
                            }
                        }
                    }
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
