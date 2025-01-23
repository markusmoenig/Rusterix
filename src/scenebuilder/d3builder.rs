// use crate::PrimitiveMode::*;
use crate::SceneBuilder;
use crate::Texture;
use crate::{Batch, D3Camera, Entity, Map, PixelSource, Scene, Tile, Value, ValueContainer};
use theframework::prelude::*;
use vek::Vec2;

pub struct D3Builder {}

impl SceneBuilder for D3Builder {
    fn new() -> Self {
        Self {}
    }

    fn build(
        &self,
        map: &Map,
        tiles: &FxHashMap<Uuid, Tile>,
        atlas: Texture,
        _screen_size: Vec2<f32>,
        camera_id: &str,
        _properties: &ValueContainer,
    ) -> Scene {
        let mut scene = Scene::empty();
        // let atlas_size = atlas.width as f32;
        let tile_size = 100;

        let mut textures = vec![Tile::from_texture(atlas)];

        let atlas_batch = Batch::emptyd3();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 4]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        // Create sectors
        for sector in &map.sectors {
            if let Some((vertices, indices)) = sector.generate_geometry(map) {
                let sector_elevation = sector.properties.get_float_default("floor_height", 0.0);

                // Generate floor geometry

                if let Some(Value::Source(pixelsource)) = sector.properties.get("floor_source") {
                    if let Some(tile) = pixelsource.to_tile(tiles, tile_size, &sector.properties) {
                        let floor_vertices = vertices
                            .iter()
                            .map(|&v| {
                                [
                                    v[0],
                                    sector.properties.get_float_default("floor_height", 0.0),
                                    v[1],
                                    1.0,
                                ]
                            })
                            .collect();

                        let floor_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();

                        if let Some(offset) = repeated_offsets.get(&tile.id) {
                            repeated_batches[*offset].add(
                                floor_vertices,
                                indices.clone(),
                                floor_uvs,
                            );
                        } else {
                            let texture_index = textures.len();

                            let mut batch = Batch::emptyd3()
                                .repeat_mode(crate::RepeatMode::RepeatXY)
                                .sample_mode(crate::SampleMode::Linear)
                                .texture_index(texture_index);

                            batch.add(floor_vertices, indices.clone(), floor_uvs);

                            textures.push(tile.clone());
                            repeated_offsets.insert(tile.id, repeated_batches.len());
                            repeated_batches.push(batch);
                        }
                    }
                }

                // Generate ceiling geometry

                let mut create_ceiling = true;
                if camera_id == "iso" && sector.properties.get_int_default("ceiling_in_iso", 0) == 1
                {
                    create_ceiling = false;
                }

                if create_ceiling {
                    if let Some(Value::Source(PixelSource::TileId(id))) =
                        &sector.properties.get("ceiling_source")
                    {
                        if let Some(tile) = tiles.get(id) {
                            let ceiling_vertices = vertices
                                .iter()
                                .map(|&v| {
                                    [
                                        v[0],
                                        sector.properties.get_float_default("ceiling_height", 0.0),
                                        v[1],
                                        1.0,
                                    ]
                                })
                                .collect();

                            let floor_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();

                            if let Some(offset) = repeated_offsets.get(&tile.id) {
                                repeated_batches[*offset].add(ceiling_vertices, indices, floor_uvs);
                            } else {
                                let texture_index = textures.len();

                                let mut batch = Batch::emptyd3()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .sample_mode(crate::SampleMode::Linear)
                                    .texture_index(texture_index);

                                batch.add(ceiling_vertices, indices, floor_uvs);

                                textures.push(tile.clone());
                                repeated_offsets.insert(tile.id, repeated_batches.len());
                                repeated_batches.push(batch);
                            }
                        }
                    }
                }

                // Generate wall geometry
                for &linedef_id in &sector.linedefs {
                    if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                        if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                            if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                let repeat_sources =
                                    linedef.properties.get_int_default("source_repeat", 0) == 0;
                                Self::add_wall2(
                                    sector_elevation,
                                    &start_vertex.as_vec2(),
                                    &end_vertex.as_vec2(),
                                    linedef.properties.get_float_default("wall_height", 0.0),
                                    if let Some(Value::Source(PixelSource::TileId(id))) =
                                        linedef.properties.get("row1_source")
                                    {
                                        Some(*id)
                                    } else {
                                        None
                                    },
                                    if let Some(Value::Source(PixelSource::TileId(id))) =
                                        linedef.properties.get("row2_source")
                                    {
                                        Some(*id)
                                    } else {
                                        None
                                    },
                                    if let Some(Value::Source(PixelSource::TileId(id))) =
                                        linedef.properties.get("row3_source")
                                    {
                                        Some(*id)
                                    } else {
                                        None
                                    },
                                    if let Some(Value::Source(PixelSource::TileId(id))) =
                                        linedef.properties.get("row4_source")
                                    {
                                        Some(*id)
                                    } else {
                                        None
                                    },
                                    repeat_sources,
                                    tiles,
                                    &mut repeated_offsets,
                                    &mut repeated_batches,
                                    &mut textures,
                                );
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
                        Self::add_wall2(
                            0.0,
                            &start_vertex.as_vec2(),
                            &end_vertex.as_vec2(),
                            linedef.properties.get_float_default("wall_height", 0.0),
                            if let Some(Value::Source(PixelSource::TileId(id))) =
                                linedef.properties.get("row1_source")
                            {
                                Some(*id)
                            } else {
                                None
                            },
                            if let Some(Value::Source(PixelSource::TileId(id))) =
                                linedef.properties.get("row2_source")
                            {
                                Some(*id)
                            } else {
                                None
                            },
                            if let Some(Value::Source(PixelSource::TileId(id))) =
                                linedef.properties.get("row3_source")
                            {
                                Some(*id)
                            } else {
                                None
                            },
                            if let Some(Value::Source(PixelSource::TileId(id))) =
                                linedef.properties.get("row4_source")
                            {
                                Some(*id)
                            } else {
                                None
                            },
                            repeat_sources,
                            tiles,
                            &mut repeated_offsets,
                            &mut repeated_batches,
                            &mut textures,
                        );
                    }
                }
            }
        }

        if camera_id != "iso" {
            // Add Sky
            if let Some(sky_texture_id) = map.sky_texture {
                Self::add_sky(
                    &sky_texture_id,
                    tiles,
                    &mut repeated_offsets,
                    &mut repeated_batches,
                    &mut textures,
                );
            }
        }

        // ---

        let mut batches = repeated_batches;
        batches.extend(vec![atlas_batch]);

        scene.d3_static = batches;
        scene.textures = textures;
        scene.lights = map.lights.clone();
        scene
    }

    fn build_entities_d3(
        &self,
        entities: &[Entity],
        camera: &dyn D3Camera,
        tiles: &FxHashMap<Uuid, Tile>,
        scene: &mut Scene,
    ) {
        let mut textures = vec![];
        let mut batches = vec![];

        fn add_entity_billboard(
            start_vertex: &Vec2<f32>,
            end_vertex: &Vec2<f32>,
            wall_height: f32,
            batch: &mut Batch<[f32; 4]>,
        ) {
            let wall_vertices = vec![
                [start_vertex.x, 0.0, start_vertex.y, 1.0],
                [start_vertex.x, wall_height, start_vertex.y, 1.0],
                [end_vertex.x, wall_height, end_vertex.y, 1.0],
                [end_vertex.x, 0.0, end_vertex.y, 1.0],
            ];

            let wall_uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

            let wall_indices = vec![(0, 1, 2), (0, 2, 3)];
            batch.add(wall_vertices, wall_indices, wall_uvs);
        }

        let mut index = 0;
        for entity in entities {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                if let Some(id) = entity.get_attr_uuid("tile_id") {
                    let entity_pos = Vec2::new(entity.position.x, entity.position.z);
                    let camera_pos = Vec2::new(camera.position().x, camera.position().z);
                    let direction_to_camera = (camera_pos - entity_pos).normalized();

                    // Calculate perpendicular vector on the XZ plane
                    let perpendicular = Vec2::new(-direction_to_camera.y, direction_to_camera.x);
                    let start = entity_pos + perpendicular * 0.5;
                    let end = entity_pos - perpendicular * 0.5;

                    let mut batch = Batch::emptyd3()
                        .texture_index(index)
                        .repeat_mode(crate::RepeatMode::RepeatXY);

                    add_entity_billboard(&start, &end, 2.0, &mut batch);

                    if let Some(tile) = tiles.get(&id) {
                        textures.push(tile.clone());
                    }

                    batches.push(batch);
                    index += 1;
                }
            }
        }

        scene.d3_dynamic = batches;
        scene.dynamic_textures = textures;
    }
}

trait D3BuilderUtils {
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    fn add_wall(
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        wall_texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    );

    #[allow(clippy::too_many_arguments)]
    fn add_wall2(
        sector_elevation: f32,
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        wall_texture_id: Option<Uuid>,
        row2_texture_id: Option<Uuid>,
        row3_texture_id: Option<Uuid>,
        row4_texture_id: Option<Uuid>,
        repeat_last_row: bool,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    );

    fn add_sky(
        texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    );
}

impl D3BuilderUtils for D3Builder {
    /// Adds a wall to the appropriate batch
    fn add_wall(
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        wall_texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    ) {
        let wall_vertices = vec![
            [start_vertex.x, 0.0, start_vertex.y, 1.0],
            [start_vertex.x, wall_height, start_vertex.y, 1.0],
            [end_vertex.x, wall_height, end_vertex.y, 1.0],
            [end_vertex.x, 0.0, end_vertex.y, 1.0],
        ];

        if let Some(tile) = tiles.get(wall_texture_id) {
            let wall_uvs =
                if (end_vertex.x - start_vertex.x).abs() > (end_vertex.y - start_vertex.y).abs() {
                    // Wall is mostly aligned along the X-axis
                    vec![
                        [start_vertex.x, wall_height],
                        [start_vertex.x, 0.0],
                        [end_vertex.x, 0.0],
                        [end_vertex.x, wall_height],
                    ]
                } else {
                    // Wall is mostly aligned along the Z-axis
                    vec![
                        [start_vertex.y, wall_height],
                        [start_vertex.y, 0.0],
                        [end_vertex.y, 0.0],
                        [end_vertex.y, wall_height],
                    ]
                };

            let wall_indices = vec![(0, 1, 2), (0, 2, 3)];

            if let Some(offset) = repeated_offsets.get(&tile.id) {
                repeated_batches[*offset].add(wall_vertices, wall_indices, wall_uvs);
            } else {
                let texture_index = textures.len();

                let mut batch = Batch::emptyd3()
                    .repeat_mode(crate::RepeatMode::RepeatXY)
                    .cull_mode(crate::CullMode::Off)
                    .sample_mode(crate::SampleMode::Anisotropic { max_samples: 2 })
                    .texture_index(texture_index);

                batch.add(wall_vertices, wall_indices, wall_uvs);

                textures.push(tile.clone());
                repeated_offsets.insert(tile.id, repeated_batches.len());
                repeated_batches.push(batch);
            }
        }
    }

    /// Adds a wall to the appropriate batch based on up to 3 input textures.
    fn add_wall2(
        sector_elevation: f32,
        start_vertex: &Vec2<f32>,
        end_vertex: &Vec2<f32>,
        wall_height: f32,
        row1_texture_id: Option<Uuid>, // Optional texture for row 1
        row2_texture_id: Option<Uuid>, // Optional texture for row 2
        row3_texture_id: Option<Uuid>, // Optional texture for row 3
        row4_texture_id: Option<Uuid>, // Optional texture for row 4
        repeat_last_row: bool,         // If true, repeat the last defined row's texture
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    ) {
        // Calculate dynamic row heights based on wall_height
        let row_heights = if wall_height <= 1.0 {
            vec![wall_height] // Only row 1 fits
        } else if wall_height <= 2.0 {
            vec![1.0, wall_height - 1.0] // Row 1 + Row 2
        } else if wall_height <= 3.0 {
            vec![1.0, 1.0, wall_height - 2.0] // Row 1 + Row 2 + Row 3
        } else {
            vec![1.0, 1.0, 1.0, wall_height - 3.0] // Row 1 + Row 2 + Row 3 + Row 4
        };

        // Function to add a row geometry
        let mut add_row = |start_height: f32, end_height: f32, texture_id: &Uuid| {
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

            if let Some(tile) = tiles.get(texture_id) {
                if let Some(offset) = repeated_offsets.get(&tile.id) {
                    repeated_batches[*offset].add(row_vertices, row_indices, row_uvs);
                } else {
                    let texture_index = textures.len();

                    let mut batch = Batch::emptyd3()
                        .repeat_mode(crate::RepeatMode::RepeatXY)
                        .cull_mode(crate::CullMode::Off)
                        .sample_mode(crate::SampleMode::Linear)
                        .texture_index(texture_index);

                    batch.add(row_vertices, row_indices, row_uvs);

                    textures.push(tile.clone());
                    repeated_offsets.insert(tile.id, repeated_batches.len());
                    repeated_batches.push(batch);
                }
            }
        };

        // Generate rows based on available textures and dynamic row heights
        let mut current_height = 0.0;
        let mut last_texture_id = if repeat_last_row {
            row1_texture_id
        } else {
            None
        };

        // Row 1
        if let Some(row1_id) = row1_texture_id {
            let next_height = (current_height + row_heights[0]).min(wall_height);
            add_row(
                sector_elevation + current_height,
                sector_elevation + next_height,
                &row1_id,
            );
            current_height = next_height;
            if repeat_last_row {
                last_texture_id = Some(row1_id);
            }
        } else {
            current_height = row_heights[0]; // Skip row 1's height
        }

        // Row 2
        if current_height < wall_height {
            if let Some(row2_id) = row2_texture_id.or(if repeat_last_row {
                last_texture_id
            } else {
                None
            }) {
                let next_height =
                    (current_height + row_heights.get(1).cloned().unwrap_or(0.0)).min(wall_height);
                add_row(
                    sector_elevation + current_height,
                    sector_elevation + next_height,
                    &row2_id,
                );
                current_height = next_height;
                if repeat_last_row {
                    last_texture_id = Some(row2_id);
                }
            } else {
                current_height += row_heights.get(1).cloned().unwrap_or(0.0); // Skip row 2's height
            }
        }

        // Row 3
        if current_height < wall_height {
            if let Some(row3_id) = row3_texture_id.or(if repeat_last_row {
                last_texture_id
            } else {
                None
            }) {
                let next_height =
                    (current_height + row_heights.get(2).cloned().unwrap_or(0.0)).min(wall_height);
                add_row(
                    sector_elevation + current_height,
                    sector_elevation + next_height,
                    &row3_id,
                );
                current_height = next_height;
                if repeat_last_row {
                    last_texture_id = Some(row3_id);
                }
            } else {
                current_height += row_heights.get(2).cloned().unwrap_or(0.0); // Skip row 3's height
            }
        }

        // Row 4
        if current_height < wall_height {
            if let Some(row4_id) = row4_texture_id.or(if repeat_last_row {
                last_texture_id
            } else {
                None
            }) {
                let next_height =
                    (current_height + row_heights.get(3).cloned().unwrap_or(0.0)).min(wall_height);
                add_row(
                    sector_elevation + current_height,
                    sector_elevation + next_height,
                    &row4_id,
                );
                current_height = next_height;
            }
        }

        // Repeat the last row's texture until the wall height is filled
        if repeat_last_row && current_height < wall_height {
            if let Some(last_id) = last_texture_id {
                while current_height < wall_height {
                    let next_height = (current_height + 1.0).min(wall_height); // Use 1.0 as a default segment height
                    add_row(
                        sector_elevation + current_height,
                        sector_elevation + next_height,
                        &last_id,
                    );
                    current_height = next_height;
                }
            }
        }
    }

    /// Adds a skybox or skymap
    fn add_sky(
        texture_id: &Uuid,
        tiles: &FxHashMap<Uuid, Tile>,
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
    ) {
        // Define sky vertices
        let sky_vertices = vec![
            [-1000.0, 10.0, -1000.0, 1.0],
            [1000.0, 10.0, -1000.0, 1.0],
            [1000.0, 10.0, 1000.0, 1.0],
            [-1000.0, 10.0, 1000.0, 1.0],
        ];

        // Define UV coordinates for the sky texture
        let sky_uvs = vec![[0.0, 15.0], [15.0, 15.0], [15.0, 0.0], [0.0, 0.0]];

        // Define indices for rendering the quad
        let sky_indices = vec![(0, 1, 2), (0, 2, 3)];

        if let Some(tile) = tiles.get(texture_id) {
            // Create a new batch for the sky texture
            let texture_index = textures.len();

            let mut batch = Batch::emptyd3()
                .repeat_mode(crate::RepeatMode::RepeatXY)
                .cull_mode(crate::CullMode::Off)
                .sample_mode(crate::SampleMode::Linear)
                .texture_index(texture_index)
                .receives_light(false);

            batch.add(sky_vertices, sky_indices, sky_uvs);

            textures.push(tile.clone());
            repeated_offsets.insert(tile.id, repeated_batches.len());
            repeated_batches.push(batch);
        }
    }
}
