// use crate::PrimitiveMode::*;

// use crate::Texture;
use crate::{Assets, Batch, Map, Scene, Tile, Value};
use theframework::prelude::*;
use vek::Vec2;

pub struct D2Builder {
    pub activated_widgets: Vec<u32>,
}

impl Default for D2Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2Builder {
    pub fn new() -> Self {
        Self {
            activated_widgets: vec![],
        }
    }

    pub fn build(&mut self, map: &Map, assets: &Assets, screen_size: Vec2<f32>) -> Scene {
        let mut scene = Scene::empty();
        let atlas_size = assets.atlas.width as f32;

        let mut textures = vec![Tile::from_texture(assets.atlas.clone())];

        // --

        let mut atlas_batch = Batch::emptyd2();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 2]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        for sector in &map.sectors {
            if let Some(geo) = sector.generate_geometry(map) {
                let mut vertices: Vec<[f32; 2]> = vec![];
                let mut uvs: Vec<[f32; 2]> = vec![];
                let bbox = sector.bounding_box(map);

                let mut repeat = true;
                let tile_size = 100;

                if sector.properties.get_int_default("tile_mode", 1) == 0 {
                    repeat = false;
                }

                // Add Floor Light
                if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
                    if let Some(center) = sector.center(map) {
                        let light =
                            light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                        scene.lights.push(light);
                    }
                }
                // Add Ceiling Light
                if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
                    if let Some(center) = sector.center(map) {
                        let light =
                            light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                        scene.lights.push(light);
                    }
                }

                // Use the floor or ceiling source
                let mut source = sector.properties.get("floor_source");
                if source.is_none() || self.activated_widgets.contains(&sector.id) {
                    source = sector.properties.get("ceiling_source");
                }

                if let Some(Value::Source(pixelsource)) = source {
                    if let Some(tile) =
                        pixelsource.to_tile(assets, tile_size, &sector.properties, map)
                    {
                        for vertex in &geo.0 {
                            let local = self.map_grid_to_local(
                                screen_size,
                                Vec2::new(vertex[0], vertex[1]),
                                map,
                            );

                            let index = 0;

                            if !repeat {
                                let uv = [
                                    (tile.uvs[index].x as f32
                                        + ((vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x)
                                            * tile.uvs[index].z as f32))
                                        / atlas_size,
                                    ((tile.uvs[index].y as f32
                                        + (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y)
                                            * tile.uvs[index].w as f32)
                                        / atlas_size),
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
                            vertices.push([local.x, local.y]);
                        }

                        if repeat {
                            if let Some(offset) = repeated_offsets.get(&tile.id) {
                                repeated_batches[*offset].add(vertices, geo.1, uvs);
                            } else {
                                let texture_index = textures.len();

                                let mut batch = Batch::emptyd2()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .texture_index(texture_index)
                                    .receives_light(true);

                                batch.add(vertices, geo.1, uvs);

                                textures.push(tile.clone());
                                repeated_offsets.insert(tile.id, repeated_batches.len());
                                repeated_batches.push(batch);
                            }
                        } else {
                            atlas_batch.add(vertices, geo.1, uvs);
                        }
                    }
                }
            }
        }

        // Walls
        for sector in &map.sectors {
            if let Some(hash) = sector.generate_wall_geometry_by_linedef(map) {
                for (linedef_id, geo) in hash.iter() {
                    let mut source = None;

                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }
                    }

                    let mut vertices: Vec<[f32; 2]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];
                    let bbox = sector.bounding_box(map);

                    let repeat = true;
                    let tile_size = 100;

                    if let Some(pixelsource) = source {
                        if let Some(tile) =
                            pixelsource.to_tile(assets, tile_size, &sector.properties, map)
                        {
                            for vertex in &geo.0 {
                                let local = self.map_grid_to_local(
                                    screen_size,
                                    Vec2::new(vertex[0], vertex[1]),
                                    map,
                                );

                                let index = 0;

                                if !repeat {
                                    let uv = [
                                        (tile.uvs[index].x as f32
                                            + ((vertex[0] - bbox.min.x)
                                                / (bbox.max.x - bbox.min.x)
                                                * tile.uvs[index].z as f32))
                                            / atlas_size,
                                        ((tile.uvs[index].y as f32
                                            + (vertex[1] - bbox.min.y)
                                                / (bbox.max.y - bbox.min.y)
                                                * tile.uvs[index].w as f32)
                                            / atlas_size),
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
                                vertices.push([local.x, local.y]);
                            }

                            if repeat {
                                if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(vertices, geo.1.clone(), uvs);
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch::emptyd2()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .texture_index(texture_index)
                                        .receives_light(true);

                                    batch.add(vertices, geo.1.clone(), uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                }
                            } else {
                                atlas_batch.add(vertices, geo.1.clone(), uvs);
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            if linedef.front_sector.is_none()
                && linedef.back_sector.is_none()
                && linedef.properties.get_float_default("wall_width", 0.0) > 0.0
            {
                if let Some(hash) =
                    crate::map::geometry::generate_line_segments_d2(map, &[linedef.id])
                {
                    for (_linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];

                        let tile_size = 100;
                        if let Some(pixelsource) = source {
                            if let Some(tile) =
                                pixelsource.to_tile(assets, tile_size, &linedef.properties, map)
                            {
                                for vertex in &geo.0 {
                                    let local = self.map_grid_to_local(
                                        screen_size,
                                        Vec2::new(vertex[0], vertex[1]),
                                        map,
                                    );

                                    let texture_scale = 1.0;
                                    let uv =
                                        [(vertex[0]) / texture_scale, (vertex[1]) / texture_scale];
                                    uvs.push(uv);
                                    vertices.push([local.x, local.y]);
                                }

                                if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(vertices, geo.1.clone(), uvs);
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch::emptyd2()
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .texture_index(texture_index)
                                        .receives_light(true);

                                    batch.add(vertices, geo.1.clone(), uvs);

                                    textures.push(tile.clone());
                                    repeated_offsets.insert(tile.id, repeated_batches.len());
                                    repeated_batches.push(batch);
                                }
                            } else {
                                // atlas_batch.add(vertices, geo.1.clone(), uvs);
                            }
                        }
                    }
                }
            }
        }

        let mut batches = repeated_batches;
        batches.extend(vec![atlas_batch]);

        let tiles = assets.blocking_tiles();
        scene.mapmini = map.as_mini(&tiles);
        scene.d2_static = batches;
        scene.textures = textures;
        scene
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_entities_items(
        &self,
        map: &Map,
        assets: &Assets,
        scene: &mut Scene,
        screen_size: Vec2<f32>,
    ) {
        scene.dynamic_lights = vec![];

        let mut repeated_batches: Vec<Batch<[f32; 2]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        let mut textures = vec![];

        // We dont show entities and items in Effects Mode to avoid overlapping icons
        // Entities
        for entity in &map.entities {
            let entity_pos = Vec2::new(entity.position.x, entity.position.z);
            let pos =
                self.map_grid_to_local(screen_size, Vec2::new(entity_pos.x, entity_pos.y), map);
            let size = 1.0;
            let hsize = 0.5;

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
                    if let Some(tile) = source.to_tile(assets, 100, &entity.attributes, map) {
                        let texture_index = textures.len();

                        let mut batch = Batch::emptyd2()
                            .texture_index(texture_index)
                            .receives_light(true);

                        batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                        textures.push(tile.clone());
                        repeated_offsets.insert(tile.id, repeated_batches.len());
                        repeated_batches.push(batch);
                    }
                }
            }
        }

        // Items
        for item in &map.items {
            let item_pos = Vec2::new(item.position.x, item.position.z);
            let pos = self.map_grid_to_local(screen_size, Vec2::new(item_pos.x, item_pos.y), map);
            let size = 1.0;
            let hsize = 0.5;

            if let Some(Value::Light(light)) = item.attributes.get("light") {
                let mut light = light.clone();
                light.set_position(item.position);
                scene.dynamic_lights.push(light);
            }

            if let Some(Value::Source(source)) = item.attributes.get("source") {
                if item.attributes.get_bool_default("visible", false) {
                    if let Some(tile) = source.to_tile(assets, 100, &item.attributes, map) {
                        let texture_index = textures.len();

                        let mut batch = Batch::emptyd2()
                            .texture_index(texture_index)
                            .receives_light(true);

                        batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                        textures.push(tile.clone());
                        repeated_offsets.insert(tile.id, repeated_batches.len());
                        repeated_batches.push(batch);
                    }
                }
            }
        }

        scene.d2_dynamic = repeated_batches;
        scene.dynamic_textures = textures;
    }

    #[inline(always)]
    fn map_grid_to_local(
        &self,
        _screen_size: Vec2<f32>,
        grid_pos: Vec2<f32>,
        _map: &Map,
    ) -> Vec2<f32> {
        // let grid_space_pos = grid_pos * map.grid_size;
        // grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
        grid_pos
    }
}
