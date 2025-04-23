use crate::{
    Assets, Batch, D3Camera, Map, PixelSource, SampleMode, Scene, Tile, Value, ValueContainer,
};
use theframework::prelude::*;
use vek::Vec2;

pub struct D3Builder {
    map: Map,
    tile_size: i32,
}

impl Default for D3Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D3Builder {
    pub fn new() -> Self {
        Self {
            map: Map::default(),
            tile_size: 128,
        }
    }

    pub fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        _screen_size: Vec2<f32>,
        camera_id: &str,
        properties: &ValueContainer,
    ) -> Scene {
        self.map = map.clone();

        let mut sample_mode = SampleMode::Nearest;
        if let Some(Value::SampleMode(sm)) = properties.get("sample_mode") {
            sample_mode = *sm;
        }

        let mut scene = Scene::empty();
        // let atlas_size = atlas.width as f32;
        self.tile_size = properties.get_int_default("tile_size", 128);

        let mut textures = vec![Tile::from_texture(assets.atlas.clone())];

        let atlas_batch = Batch::emptyd3();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 4]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        // Create sectors
        for sector in &map.sectors {
            // Add Floor Light
            if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
                if let Some(center) = sector.center(map) {
                    let bbox = sector.bounding_box(map);
                    let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                    scene.lights.push(light);
                }
            }
            // Add Ceiling Light
            if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
                if let Some(center) = sector.center(map) {
                    let bbox = sector.bounding_box(map);
                    let light = light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                    scene.lights.push(light);
                }
            }

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
                if let Some((vertices, indices)) = sector.generate_geometry(map) {
                    let sector_elevation = sector.properties.get_float_default("floor_height", 0.0);

                    // Generate floor geometry
                    if !add_it_as_box {
                        if let Some(Value::Source(pixelsource)) =
                            sector.properties.get("floor_source")
                        {
                            if let Some(tile) = pixelsource.to_tile(
                                assets,
                                self.tile_size as usize,
                                &sector.properties,
                                map,
                            ) {
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
                                        .sample_mode(sample_mode)
                                        .texture_index(texture_index);

                                    batch.add(floor_vertices, indices.clone(), floor_uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                }
                            }
                        }
                    }

                    // Generate ceiling geometry

                    let mut create_ceiling = true;
                    if camera_id == "iso"
                        && sector.properties.get_int_default("ceiling_in_iso", 0) == 1
                    {
                        create_ceiling = false;
                    }

                    if create_ceiling || add_it_as_box {
                        let source = if add_it_as_box {
                            sector.properties.get("floor_source")
                        } else {
                            sector.properties.get("ceiling_source")
                        };

                        if let Some(Value::Source(pixelsource)) = &source {
                            if let Some(tile) = pixelsource.to_tile(
                                assets,
                                self.tile_size as usize,
                                &sector.properties,
                                map,
                            ) {
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

                                let floor_uvs = vertices.iter().map(|&v| [v[0], v[1]]).collect();

                                if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(
                                        ceiling_vertices,
                                        indices,
                                        floor_uvs,
                                    );
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch::emptyd3()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .sample_mode(sample_mode)
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
                    if !add_it_as_floor {
                        for &linedef_id in &sector.linedefs {
                            if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                                        // ---
                                        // Check for wall lights
                                        //
                                        for i in 1..=4 {
                                            let light_name = format!("row{}_light", i);
                                            if let Some(Value::Light(light)) =
                                                linedef.properties.get(&light_name)
                                            {
                                                let light = light.from_linedef(
                                                    start_vertex.as_vec2(),
                                                    end_vertex.as_vec2(),
                                                    i as f32 - 0.5,
                                                );
                                                scene.lights.push(light);
                                            }
                                        }
                                        // --

                                        let repeat_sources =
                                            linedef.properties.get_int_default("source_repeat", 0)
                                                == 0;
                                        self.add_wall(
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
                                            &mut repeated_offsets,
                                            &mut repeated_batches,
                                            &mut textures,
                                            &sample_mode,
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
                        self.add_wall(
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
                            &mut repeated_offsets,
                            &mut repeated_batches,
                            &mut textures,
                            &sample_mode,
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
                    &assets.tiles,
                    &mut repeated_offsets,
                    &mut repeated_batches,
                    &mut textures,
                );
            }
        }

        // ---

        let mut batches = repeated_batches;
        batches.extend(vec![atlas_batch]);

        scene.mapmini = map.as_mini(&assets.blocking_tiles());
        scene.d3_static = batches;
        scene.textures = textures;
        scene.compute_static_normals();

        scene
    }

    pub fn build_entities_items(
        &self,
        map: &Map,
        camera: &dyn D3Camera,
        assets: &Assets,
        scene: &mut Scene,
    ) {
        scene.dynamic_lights = vec![];
        let mut textures = vec![];
        let mut batches = vec![];

        fn add_billboard(
            start_vertex: &Vec2<f32>,
            end_vertex: &Vec2<f32>,
            scale: f32,
            batch: &mut Batch<[f32; 4]>,
        ) {
            let wall_vertices = vec![
                [start_vertex.x, 0.0, start_vertex.y, 1.0],
                [start_vertex.x, scale, start_vertex.y, 1.0],
                [end_vertex.x, scale, end_vertex.y, 1.0],
                [end_vertex.x, 0.0, end_vertex.y, 1.0],
            ];

            let wall_uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

            let wall_indices = vec![(0, 1, 2), (0, 2, 3)];
            batch.add(wall_vertices, wall_indices, wall_uvs);
        }

        let camera_pos = Vec2::new(camera.position().x, camera.position().z);
        let mut index = 0;

        // Billboard sectors (Rect)
        for sector in self.map.sectors.iter() {
            if sector.layer.is_some() {
                let render_mode = sector.properties.get_int_default("rect_rendering", 0);

                if let Some(Value::Source(source)) = sector.properties.get("floor_source") {
                    if render_mode == 0 {
                        // Billboard
                        let mut scale = 1.0;
                        if let PixelSource::TileId(tile_id) = source {
                            if let Some(tile) = assets.tiles.get(tile_id) {
                                scale = tile.scale;
                            }
                        }
                        if let Some(position) = sector.center(&self.map) {
                            let direction_to_camera = (camera_pos - position).normalized();
                            let perpendicular =
                                Vec2::new(-direction_to_camera.y, direction_to_camera.x);
                            let start = position + perpendicular * 0.5 * scale;
                            let end = position - perpendicular * 0.5 * scale;

                            let mut batch = Batch::emptyd3()
                                .texture_index(index)
                                .repeat_mode(crate::RepeatMode::RepeatXY);

                            add_billboard(&start, &end, scale, &mut batch);

                            if let Some(tile) = source.to_tile(
                                assets,
                                self.tile_size as usize,
                                &sector.properties,
                                map,
                            ) {
                                textures.push(tile);
                            }

                            batches.push(batch);
                            index += 1;
                        }
                    }
                }
            }
        }

        // Entities
        for entity in &map.entities {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                // Find light on entity
                if let Some(Value::Light(light)) = entity.attributes.get("light") {
                    let mut light = light.clone();
                    light.set_position(entity.position);
                    scene.dynamic_lights.push(light);
                }

                // Find light on entity items
                for item in entity.iter_inventory() {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        let mut light = light.clone();
                        light.set_position(entity.position);
                        scene.dynamic_lights.push(light);
                    }
                }

                if let Some(Value::Source(source)) = entity.attributes.get("source") {
                    if entity.attributes.get_bool_default("visible", false) {
                        let entity_pos = Vec2::new(entity.position.x, entity.position.z);
                        let direction_to_camera = (camera_pos - entity_pos).normalized();

                        // Calculate perpendicular vector on the XZ plane
                        let perpendicular =
                            Vec2::new(-direction_to_camera.y, direction_to_camera.x);
                        let start = entity_pos + perpendicular * 0.5;
                        let end = entity_pos - perpendicular * 0.5;

                        let mut batch = Batch::emptyd3()
                            .texture_index(index)
                            .repeat_mode(crate::RepeatMode::RepeatXY);

                        add_billboard(&start, &end, 2.0, &mut batch);

                        if let Some(tile) =
                            source.to_tile(assets, self.tile_size as usize, &entity.attributes, map)
                        {
                            textures.push(tile);
                        }

                        batches.push(batch);
                        index += 1;
                    }
                }
            }
        }

        // Items
        for item in &map.items {
            let show_entity = true; // !(entity.is_player() && camera.id() == "firstp");

            if show_entity {
                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    let mut light = light.clone();
                    light.set_position(item.position);
                    scene.dynamic_lights.push(light);
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if item.attributes.get_bool_default("visible", false) {
                        let item_pos = Vec2::new(item.position.x, item.position.z);
                        let direction_to_camera = (camera_pos - item_pos).normalized();

                        // Calculate perpendicular vector on the XZ plane
                        let perpendicular =
                            Vec2::new(-direction_to_camera.y, direction_to_camera.x);
                        let start = item_pos + perpendicular * 0.5;
                        let end = item_pos - perpendicular * 0.5;

                        let mut batch = Batch::emptyd3()
                            .texture_index(index)
                            .repeat_mode(crate::RepeatMode::RepeatXY);

                        add_billboard(&start, &end, 1.0, &mut batch);

                        if let Some(tile) =
                            source.to_tile(assets, self.tile_size as usize, &item.attributes, map)
                        {
                            textures.push(tile);
                        }

                        batches.push(batch);
                        index += 1;
                    }
                }
            }
        }

        scene.d3_dynamic = batches;
        scene.dynamic_textures = textures;
        scene.compute_dynamic_normals();
    }

    /// Adds a wall to the appropriate batch based on up to 4 input textures.
    #[allow(clippy::too_many_arguments)]
    fn add_wall(
        &self,
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
        repeated_offsets: &mut FxHashMap<Uuid, usize>,
        repeated_batches: &mut Vec<Batch<[f32; 4]>>,
        textures: &mut Vec<Tile>,
        sample_mode: &SampleMode,
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

            if let Some(offset) = repeated_offsets.get(&tile.id) {
                repeated_batches[*offset].add(row_vertices, row_indices, row_uvs);
            } else {
                let texture_index = textures.len();

                let mut batch = Batch::emptyd3()
                    .repeat_mode(crate::RepeatMode::RepeatXY)
                    .cull_mode(crate::CullMode::Off)
                    .sample_mode(*sample_mode)
                    .texture_index(texture_index);

                batch.add(row_vertices, row_indices, row_uvs);
                textures.push(tile.clone());
                repeated_offsets.insert(tile.id, repeated_batches.len());
                repeated_batches.push(batch);
            }
        };

        let sources = [row1_source, row2_source, row3_source, row4_source];
        let mut current_height = 0.0;
        let mut last_tile: Option<Tile> = None;

        for (i, height) in row_heights.iter().enumerate() {
            if current_height >= wall_height {
                break;
            }

            let source_tile = sources[i].and_then(|source| {
                source.to_tile(assets, self.tile_size as usize, properties, map)
            });

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
