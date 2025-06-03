pub mod material;
pub mod shape;
pub mod shapecontext;
pub mod shapefx;
pub mod shapefxgraph;

use crate::{Assets, Map, PixelSource, ShapeContext, ShapeFXGraph, Texture, Value};
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

    /// Render the shapes into a character or item texture.
    pub fn render_shape(&mut self, buffer: &mut Texture, map: &Map, assets: &Assets) {
        let width = buffer.width;
        let height = buffer.height;
        let area_size = self.area_max - self.area_min;

        let px = area_size.x / width as f32;

        #[inline(always)]
        fn smooth_min(d1: f32, d2: f32, k: f32) -> f32 {
            let h = (0.5 + 0.5 * (d2 - d1) / k).clamp(0.0, 1.0);
            f32::lerp(d1, d2, h) - k * h * (1.0 - h)
        }

        struct ShapeHit<'a> {
            sdf: f32,
            ctx: ShapeContext,
            graph: &'a ShapeFXGraph,
            node_index: usize,
        }

        buffer
            .data
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);
                    let world = self.area_min + uv * area_size;

                    let mut hits: Vec<ShapeHit> = vec![];
                    let mut final_sdf = f32::MAX;

                    // Vertices
                    for vertex in map.vertices.iter() {
                        if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                            vertex.properties.get("shape_graph")
                        {
                            if let Some(v) = map.get_vertex(vertex.id) {
                                if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                                    let (shape_sdf, node_index) =
                                        graph.evaluate_shape_distance(world, &[v]);

                                    let blend_k = graph.nodes[node_index]
                                        .values
                                        .get_float_default("blend_k", 0.0);

                                    final_sdf = if blend_k > 0.0 {
                                        smooth_min(final_sdf, shape_sdf, blend_k)
                                    } else {
                                        final_sdf.min(shape_sdf)
                                    };

                                    let ctx = ShapeContext {
                                        point_world: world,
                                        point: world / px,
                                        uv,
                                        distance_world: shape_sdf * px,
                                        distance: shape_sdf,
                                        shape_id: vertex.id,
                                        px,
                                        anti_aliasing: 1.0,
                                        t: None,
                                        line_dir: None,
                                    };

                                    hits.push(ShapeHit {
                                        sdf: shape_sdf,
                                        ctx,
                                        graph,
                                        node_index,
                                    });
                                }
                            }
                        }
                    }

                    // And now the standalone linedefs
                    for linedef in &map.linedefs {
                        if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                            linedef.properties.get("shape_graph")
                        {
                            if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                                if let Some(start) = map.get_vertex(linedef.start_vertex) {
                                    if let Some(end) = map.get_vertex(linedef.end_vertex) {
                                        let vertices = [start, end];

                                        let (shape_sdf, node_index) =
                                            graph.evaluate_shape_distance(world, &vertices);

                                        let blend_k = graph.nodes[node_index]
                                            .values
                                            .get_float_default("blend_k", 0.0);

                                        final_sdf = if blend_k > 0.0 {
                                            smooth_min(final_sdf, shape_sdf, blend_k)
                                        } else {
                                            final_sdf.min(shape_sdf)
                                        };

                                        let ab = vertices[1] - vertices[0];
                                        let ab_len = ab.magnitude();
                                        let ab_dir = ab / ab_len;

                                        let ap = world - vertices[0];
                                        let t = ap.dot(ab_dir) / ab_len;
                                        let t_clamped = t.clamp(0.0, 1.0);
                                        let closest_point =
                                            vertices[0] + ab_dir * (t_clamped * ab_len);

                                        // Construct UV aligned to the line
                                        let local = world - closest_point;
                                        let mut local_uv =
                                            Vec2::new(t_clamped.fract(), 0.5 + shape_sdf);

                                        // Optional: rotate local_uv into a line-aligned frame if needed
                                        // For pattern alignment, you may want:
                                        local_uv = Vec2::new(
                                            local.dot(ab_dir),
                                            local.dot(Vec2::new(-ab_dir.y, ab_dir.x)),
                                        );

                                        let ctx = ShapeContext {
                                            point_world: world,
                                            point: world / px,
                                            uv: local_uv,
                                            distance_world: shape_sdf * px,
                                            distance: shape_sdf,
                                            shape_id: linedef.id,
                                            px,
                                            anti_aliasing: linedef
                                                .properties
                                                .get_float_default("material_a_a", 1.0),
                                            t: Some(t),
                                            line_dir: Some(ab_dir),
                                        };

                                        hits.push(ShapeHit {
                                            sdf: shape_sdf,
                                            ctx,
                                            graph,
                                            node_index,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // Determine which shape owns the pixel
                    let mut color_owner: Option<&ShapeHit> = None;
                    let mut min_color_sdf = f32::MAX;

                    for hit in &hits {
                        if hit.sdf < min_color_sdf && min_color_sdf > 0.0 {
                            min_color_sdf = hit.sdf;
                            color_owner = Some(hit);
                        }
                    }

                    let mut final_color = Vec4::zero();
                    if final_sdf <= 0.0 {
                        if let Some(owner) = color_owner {
                            if let Some(col) = owner.graph.evaluate_shape_color(
                                &owner.ctx,
                                owner.node_index,
                                assets,
                            ) {
                                final_color = col;
                                final_color.w = 1.0;

                                // Optional: soft edge fade
                                // let edge_softness = 3.0;
                                // let alpha = (-(final_sdf * edge_softness)).exp().clamp(0.0, 1.0);
                                // final_color *= alpha;
                                // final_color.w = alpha;
                            }
                        }
                    }

                    // if final_sdf <= 0.0 {
                    //     final_color = Vec4::one();
                    // } else {
                    //     final_color = Vec4::zero();
                    // }
                    pixel.copy_from_slice(&TheColor::from_vec4f(final_color).to_u8_array());
                }
            });
    }

    /// Render the geometry into a material texture
    pub fn render_geometry(&mut self, buffer: &mut Texture, map: &Map, assets: &Assets) {
        let width = buffer.width;
        let height = buffer.height;
        let area_size = self.area_max - self.area_min;

        // let pixel_size = Vec2::new(area_size.x / width as f32, area_size.y / height as f32);
        let px = area_size.x / width as f32;
        // let px = pixel_size.x.max(pixel_size.y);

        buffer
            .data
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);
                    let world = self.area_min + uv * area_size;

                    let mut color = Vec4::new(0.0, 0.0, 0.0, 0.0);

                    // Do the sectors
                    let sorted_sectors = map.sorted_sectors_by_area();
                    for sector in sorted_sectors {
                        let bbox = sector.bounding_box(map);

                        if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                            sector.properties.get("floor_source")
                        {
                            if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                                let mut best_ctx = None;
                                let mut min_sdf = f32::MAX;

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
                                            let sdf = distance / px
                                                - sector
                                                    .properties
                                                    .get_float_default("material_rounding", 0.0);

                                            if sdf < min_sdf {
                                                min_sdf = sdf;
                                                best_ctx = Some(ShapeContext {
                                                    point_world: shifted_point,
                                                    point: shifted_point / px,
                                                    uv,
                                                    distance_world: distance,
                                                    distance: sdf,
                                                    shape_id: sector.id,
                                                    px,
                                                    anti_aliasing: sector
                                                        .properties
                                                        .get_float_default("material_a_a", 1.0),
                                                    t: None,
                                                    line_dir: None,
                                                });
                                            }
                                        }
                                    }
                                }

                                if let Some(ctx) = best_ctx {
                                    if let Some(col) = graph.evaluate_material(&ctx, color, assets)
                                    {
                                        color = Vec4::lerp(color, col, col.w);
                                    }
                                }
                            }
                        }
                    }

                    // And now the standalone linedefs
                    for linedef in &map.linedefs {
                        if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
                            if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                                linedef.properties.get("row1_source")
                            {
                                if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                                    if let Some(start) = map.find_vertex(linedef.start_vertex) {
                                        if let Some(end) = map.find_vertex(linedef.end_vertex) {
                                            let a = start.as_vec2();
                                            let b = end.as_vec2();

                                            let tile_size = Vec2::new(10.0, 10.0); // or store in graph
                                            let px = tile_size.x / width as f32;
                                            let line_width_px = linedef
                                                .properties
                                                .get_float_default("material_width", 1.0);

                                            let ab = b - a;
                                            let ab_len = ab.magnitude();
                                            let ab_dir = ab / ab_len;
                                            // let normal = Vec2::new(-ab_dir.y, ab_dir.x);

                                            let mut min_sdf = f32::MAX;
                                            let mut final_t = 0.0;
                                            let mut final_dir = Vec2::zero();

                                            for dx in -1..=1 {
                                                for dy in -1..=1 {
                                                    let offset = Vec2::new(
                                                        (dx as f32) * tile_size.x,
                                                        (dy as f32) * tile_size.y,
                                                    );

                                                    let shifted_point = world - offset;
                                                    let ap = shifted_point - a;

                                                    let t = ap.dot(ab_dir) / ab_len;
                                                    let t_clamped = t.clamp(0.0, 1.0);
                                                    let closest = a + ab_dir * (t_clamped * ab_len);

                                                    let sdf_px =
                                                        (shifted_point - closest).magnitude() / px
                                                            - line_width_px * 0.5;

                                                    if sdf_px < min_sdf {
                                                        min_sdf = sdf_px;
                                                        final_t = t;
                                                        final_dir = ab_dir;
                                                    }
                                                }
                                            }

                                            let ctx = ShapeContext {
                                                point_world: world,
                                                point: world / px,
                                                uv: Vec2::new(final_t.fract(), 0.5 + min_sdf), // optional, depends on effect
                                                distance_world: min_sdf * px,
                                                distance: min_sdf,
                                                shape_id: 0,
                                                px,
                                                anti_aliasing: linedef
                                                    .properties
                                                    .get_float_default("material_a_a", 1.0),
                                                t: Some(final_t),
                                                line_dir: Some(final_dir),
                                            };

                                            if let Some(col) =
                                                graph.evaluate_material(&ctx, color, assets)
                                            {
                                                color = Vec4::lerp(color, col, col.w);
                                            }
                                        }
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
