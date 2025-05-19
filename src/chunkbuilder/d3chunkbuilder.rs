use crate::{
    Assets, Batch3D, Chunk, ChunkBuilder, Map, Material, PixelSource, Tile, Value, ValueContainer,
};
use vek::Vec2;

pub struct D3ChunkBuilder {}

impl ChunkBuilder for D3ChunkBuilder {
    fn new() -> Self {
        Self {}
    }

    fn build(&mut self, map: &Map, assets: &Assets, chunk: &mut Chunk) {
        // Create sectors
        for sector in &map.sectors {
            // // Add Floor Light
            // if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
            //     if let Some(center) = sector.center(map) {
            //         let bbox = sector.bounding_box(map);
            //         let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
            //         scene.lights.push(light);
            //     }
            // }
            // // Add Ceiling Light
            // if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
            //     if let Some(center) = sector.center(map) {
            //         let bbox = sector.bounding_box(map);
            //         let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
            //         scene.lights.push(light);
            //     }
            // }

            let mut add_it = true;

            // Special cases from the Rect tool
            let mut add_it_as_box = false;
            let mut add_it_as_floor = false;

            // Make sure we add Rect sectors with a rendering mode of "Box" as a box
            if sector.layer.is_some() {
                let render_mode = sector.properties.get_int_default("rect_rendering", 0);
                match render_mode {
                    0 => add_it = false,
                    1 => add_it_as_box = true,
                    2 => add_it_as_floor = true,
                    _ => {}
                }
            }

            if add_it {
                let material: Option<Material> =
                    crate::scenebuilder::get_material_from_geo_graph(&sector.properties, 2, map);

                if let Some((vertices, indices)) = sector.generate_geometry(map) {
                    let sector_elevation = sector.properties.get_float_default("floor_height", 0.0);

                    // Generate floor geometry
                    if !add_it_as_box {
                        if let Some(Value::Source(pixelsource)) =
                            sector.properties.get("floor_source")
                        {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    let floor_vertices = vertices
                                        .iter()
                                        .map(|&v| {
                                            [
                                                v[0],
                                                sector_elevation
                                                    + if add_it_as_floor { 0.2 } else { 0.0 },
                                                v[1],
                                                1.0,
                                            ]
                                        })
                                        .collect();

                                    let floor_uvs =
                                        vertices.iter().map(|&v| [v[0], v[1]]).collect();

                                    let mut batch =
                                        Batch3D::new(floor_vertices, indices.clone(), floor_uvs)
                                            .repeat_mode(crate::RepeatMode::RepeatXY)
                                            .source(PixelSource::StaticTileIndex(texture_index));
                                    batch.material = material;
                                    chunk.batches3d.push(batch);

                                    /*
                                    if material.is_some() {
                                        let texture_index = textures.len();
                                        let mut batch = Batch::emptyd3()
                                            .repeat_mode(crate::RepeatMode::RepeatXY)
                                            .texture_index(texture_index);
                                        batch.material = material;
                                        batch.add(floor_vertices, indices.clone(), floor_uvs);

                                        textures.push(tile.clone());
                                        repeated_offsets.insert(tile.id, repeated_batches.len());
                                        repeated_batches.push(batch);
                                    } else if let Some(offset) = repeated_offsets.get(&tile.id) {
                                        repeated_batches[*offset].add(
                                            floor_vertices,
                                            indices.clone(),
                                            floor_uvs,
                                        );
                                    } else {
                                        let texture_index = textures.len();

                                        let mut batch = Batch::emptyd3()
                                            .repeat_mode(crate::RepeatMode::RepeatXY)
                                            .sample_mode(sample_mode)
                                            .texture_index(texture_index);

                                        batch.add(floor_vertices, indices.clone(), floor_uvs);

                                        textures.push(tile.clone());
                                        repeated_offsets.insert(tile.id, repeated_batches.len());
                                        repeated_batches.push(batch);
                                    }*/
                                }
                            }
                        }
                    }

                    // Generate ceiling geometry

                    let mut create_ceiling = true;
                    // if camera_id == "iso"
                    //     && sector.properties.get_int_default("ceiling_in_iso", 0) == 1
                    // {
                    //     create_ceiling = false;
                    // }

                    if create_ceiling || add_it_as_box {
                        let material: Option<Material> =
                            crate::scenebuilder::get_material_from_geo_graph(
                                &sector.properties,
                                3,
                                map,
                            );

                        let source = if add_it_as_box {
                            sector.properties.get("floor_source")
                        } else {
                            sector.properties.get("ceiling_source")
                        };

                        if let Some(Value::Source(pixelsource)) = &source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    let ceiling_vertices = vertices
                                        .iter()
                                        .map(|&v| {
                                            [
                                                v[0],
                                                sector
                                                    .properties
                                                    .get_float_default("ceiling_height", 0.0),
                                                v[1],
                                                1.0,
                                            ]
                                        })
                                        .collect();

                                    let ceiling_uvs =
                                        vertices.iter().map(|&v| [v[0], v[1]]).collect();
                                    // let ceiling_indices =
                                    //     indices.iter().map(|&v| (v.2, v.1, v.0)).collect();

                                    let mut batch = Batch3D::new(
                                        ceiling_vertices,
                                        indices.clone(),
                                        ceiling_uvs,
                                    )
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .source(PixelSource::StaticTileIndex(texture_index));
                                    batch.material = material;
                                    chunk.batches3d.push(batch);

                                    /*
                                    if material.is_some() {
                                        let texture_index = textures.len();
                                        let mut batch = Batch::emptyd3()
                                            .repeat_mode(crate::RepeatMode::RepeatXY)
                                            .texture_index(texture_index);
                                        batch.material = material;
                                        batch.add(ceiling_vertices, indices.clone(), ceiling_uvs);

                                        textures.push(tile.clone());
                                        repeated_offsets.insert(tile.id, repeated_batches.len());
                                        repeated_batches.push(batch);
                                    } else if let Some(offset) = repeated_offsets.get(&tile.id) {
                                        repeated_batches[*offset].add(
                                            ceiling_vertices,
                                            indices,
                                            ceiling_uvs,
                                        );
                                    } else {
                                        let texture_index = textures.len();

                                        let mut batch = Batch::emptyd3()
                                            .repeat_mode(crate::RepeatMode::RepeatXY)
                                            .sample_mode(sample_mode)
                                            .texture_index(texture_index);

                                        batch.add(ceiling_vertices, indices, ceiling_uvs);

                                        textures.push(tile.clone());
                                        repeated_offsets.insert(tile.id, repeated_batches.len());
                                        repeated_batches.push(batch);
                                    }
                                    */
                                }
                            }
                        }
                    }

                    // Generate wall geometry
                    if !add_it_as_floor {
                        for &linedef_id in &sector.linedefs {
                            if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                        // Check for wall lights
                                        for i in 1..=4 {
                                            if let Some(light) =
                                                crate::scenebuilder::get_linedef_light_from_geo_graph(
                                                    &linedef.properties,
                                                    i,
                                                    map,
                                                    start_vertex.as_vec2(),
                                                    end_vertex.as_vec2(),
                                                    i as f32 - 0.5,
                                                )
                                            {
                                                //TODO scene.lights.push(light);
                                            }
                                        }
                                        // --

                                        let repeat_sources =
                                            linedef.properties.get_int_default("source_repeat", 0)
                                                == 0;
                                        add_wall(
                                            sector_elevation,
                                            &start_vertex.as_vec2(),
                                            &end_vertex.as_vec2(),
                                            linedef
                                                .properties
                                                .get_float_default("wall_height", 0.0),
                                            linedef
                                                .properties
                                                .get("row1_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row2_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row3_source")
                                                .and_then(|v| v.to_source()),
                                            linedef
                                                .properties
                                                .get("row4_source")
                                                .and_then(|v| v.to_source()),
                                            repeat_sources,
                                            assets,
                                            &linedef.properties,
                                            map,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        let repeat_sources =
                            linedef.properties.get_int_default("source_repeat", 0) == 0;
                        add_wall(
                            0.0,
                            &start_vertex.as_vec2(),
                            &end_vertex.as_vec2(),
                            linedef.properties.get_float_default("wall_height", 0.0),
                            linedef
                                .properties
                                .get("row1_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row2_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row3_source")
                                .and_then(|v| v.to_source()),
                            linedef
                                .properties
                                .get("row4_source")
                                .and_then(|v| v.to_source()),
                            repeat_sources,
                            assets,
                            &linedef.properties,
                            map,
                        );
                    }
                }
            }
        }
    }
}

/// Adds a wall to the appropriate batch based on up to 4 input textures.
#[allow(clippy::too_many_arguments)]
fn add_wall(
    sector_elevation: f32,
    start_vertex: &Vec2<f32>,
    end_vertex: &Vec2<f32>,
    wall_height: f32,
    row1_source: Option<&PixelSource>,
    row2_source: Option<&PixelSource>,
    row3_source: Option<&PixelSource>,
    row4_source: Option<&PixelSource>,
    repeat_last_row: bool,
    assets: &Assets,
    properties: &ValueContainer,
    map: &Map,
) {
    let row_heights = if wall_height <= 1.0 {
        vec![wall_height]
    } else if wall_height <= 2.0 {
        vec![1.0, wall_height - 1.0]
    } else if wall_height <= 3.0 {
        vec![1.0, 1.0, wall_height - 2.0]
    } else {
        vec![1.0, 1.0, 1.0, wall_height - 3.0]
    };

    let mut add_row = |start_height: f32, end_height: f32, tile: &Tile| {
        let row_vertices = vec![
            [start_vertex.x, start_height, start_vertex.y, 1.0],
            [start_vertex.x, end_height, start_vertex.y, 1.0],
            [end_vertex.x, end_height, end_vertex.y, 1.0],
            [end_vertex.x, start_height, end_vertex.y, 1.0],
        ];

        let row_uvs =
            if (end_vertex.x - start_vertex.x).abs() > (end_vertex.y - start_vertex.y).abs() {
                vec![
                    [start_vertex.x, end_height],
                    [start_vertex.x, start_height],
                    [end_vertex.x, start_height],
                    [end_vertex.x, end_height],
                ]
            } else {
                vec![
                    [start_vertex.y, end_height],
                    [start_vertex.y, start_height],
                    [end_vertex.y, start_height],
                    [end_vertex.y, end_height],
                ]
            };

        let row_indices = vec![(0, 1, 2), (0, 2, 3)];

        if let Some(texture_index) = assets.tile_index(&tile.id) {
            let batch = Batch3D::new(row_vertices, row_indices, row_uvs)
                .repeat_mode(crate::RepeatMode::RepeatXY)
                .source(PixelSource::StaticTileIndex(texture_index));
        }
    };

    let sources = [row1_source, row2_source, row3_source, row4_source];
    let mut current_height = 0.0;
    let mut last_tile: Option<Tile> = None;

    for (i, height) in row_heights.iter().enumerate() {
        if current_height >= wall_height {
            break;
        }

        let source_tile = sources[i].and_then(|source| source.tile_from_tile_list(assets));

        let tile_to_use = if let Some(tile) = source_tile {
            last_tile = Some(tile.clone());
            Some(tile)
        } else if repeat_last_row {
            last_tile.clone()
        } else {
            None
        };

        if let Some(tile) = tile_to_use {
            let next_height = (current_height + height).min(wall_height);
            add_row(
                sector_elevation + current_height,
                sector_elevation + next_height,
                &tile,
            );
            current_height = next_height;
        } else {
            current_height += height;
        }
    }

    // Fill to the top with the last tile if repeat_last_row is enabled
    if repeat_last_row {
        if let Some(tile) = last_tile {
            while current_height < wall_height {
                let next_height = (current_height + 1.0).min(wall_height);
                add_row(
                    sector_elevation + current_height,
                    sector_elevation + next_height,
                    &tile,
                );
                current_height = next_height;
            }
        }
    }
}
