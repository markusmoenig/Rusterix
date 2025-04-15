pub mod shape;
pub mod shapecontext;
pub mod shapefx;
pub mod shapefxgraph;

use crate::{Map, PixelSource, ShapeContext, Value};
use rayon::prelude::*;
use theframework::prelude::*;
use vek::Vec2;

pub struct ShapeStack {
    area_min: Vec2<f32>,
    area_max: Vec2<f32>,
}

impl ShapeStack {
    pub fn new(area_min: Vec2<f32>, area_max: Vec2<f32>) -> Self {
        Self { area_min, area_max }
    }

    pub fn render(&mut self, buffer: &mut TheRGBABuffer, map: &mut Map, palette: &ThePalette) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;
        let area_size = self.area_max - self.area_min;

        // let pixel_size = Vec2::new(area_size.x / width as f32, area_size.y / height as f32);
        let px = area_size.x / width as f32;
        // let px = pixel_size.x.max(pixel_size.y);

        // let effects = vec![ShapeFX::new(ShapeFXRole::VerticalGradient)];

        for (_, fx) in map.effect_graphs.iter_mut() {
            fx.load(palette);
        }

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);

                    let world = self.area_min + uv * area_size;

                    let mut color = Vec4::new(0.0, 0.0, 0.0, 1.0);

                    let sorted_sectors = map.sorted_sectors_by_area();
                    for sector in sorted_sectors {
                        let bbox = sector.bounding_box(map);
                        let mut found = false;

                        if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                            sector.properties.get("floor_source")
                        {
                            if let Some(graph) = map.effect_graphs.get(graph_id) {
                                for dx in -1..=1 {
                                    for dy in -1..=1 {
                                        let offset = Vec2::new(
                                            (dx as f32) * area_size.x,
                                            (dy as f32) * area_size.y,
                                        );

                                        let shifted_point = world - offset;

                                        let uv = Vec2::new(
                                            (shifted_point.x - bbox.min.x)
                                                / (bbox.max.x - bbox.min.x),
                                            (shifted_point.y - bbox.min.y)
                                                / (bbox.max.y - bbox.min.y),
                                        );

                                        if let Some(distance) =
                                            sector.signed_distance(map, shifted_point)
                                        {
                                            let ctx = ShapeContext {
                                                point_world: shifted_point,
                                                point: shifted_point / px,
                                                uv,
                                                distance_world: distance,
                                                distance: distance / px,
                                                shape_id: sector.id,
                                                px,
                                            };

                                            if let Some(col) = graph.evaluate(&ctx, palette) {
                                                color = Vec4::lerp(color, col, col.w);
                                                found = true;
                                                break;
                                            }

                                            // for effect in effects.iter() {
                                            //     if let Some(col) = effect.evaluate(&ctx) {
                                            //         color = Vec4::lerp(color, col, col.w);
                                            //         found = true;
                                            //         break;
                                            //     }
                                            // }
                                        }
                                    }
                                    if found {
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });
    }
}
