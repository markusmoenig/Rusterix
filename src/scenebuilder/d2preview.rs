// use crate::PrimitiveMode::*;

use crate::Texture;
use crate::{
    Batch, GridShader, Map, MapToolType, Pixel, Scene, Shader, Tile, Value, ValueContainer, WHITE,
};
use theframework::prelude::*;
use vek::Vec2;

use MapToolType::*;

pub struct D2PreviewBuilder {
    selection_color: Pixel,
    map_tool_type: MapToolType,
    /// Hover geometry info
    pub hover: (Option<u32>, Option<u32>, Option<u32>),
    /// The current grid hover position
    pub hover_cursor: Option<Vec2<f32>>,
    /// Camera Position
    pub camera_pos: Option<vek::Vec3<f32>>,
    /// Camera Center
    pub look_at: Option<Vec3<f32>>,
    /// Material Mode
    pub material_mode: bool,
}

impl Default for D2PreviewBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2PreviewBuilder {
    pub fn new() -> Self {
        Self {
            selection_color: [187, 122, 208, 255],
            map_tool_type: Linedef,

            hover: (None, None, None),
            hover_cursor: None,

            camera_pos: None,
            look_at: None,

            material_mode: false,
        }
    }

    pub fn build(
        &self,
        map: &Map,
        tiles: &FxHashMap<Uuid, Tile>,
        atlas: Texture,
        screen_size: Vec2<f32>,
        _camera_id: &str,
        properties: &ValueContainer,
    ) -> Scene {
        let mut scene = Scene::empty();
        let mut grid_shader = GridShader::new();
        let atlas_size = atlas.width as f32;

        let mut textures = vec![
            Tile::from_texture(atlas),
            Tile::from_texture(Texture::from_color(WHITE)),
            Tile::from_texture(Texture::from_color(self.selection_color)),
            Tile::from_texture(Texture::from_color(vek::Rgba::yellow().into_array())),
            Tile::from_texture(Texture::from_color(vek::Rgba::red().into_array())),
            Tile::from_texture(Texture::from_color([128, 128, 128, 255])),
        ];

        // Add preview icon textures

        if let Some(Value::Texture(tex)) = properties.get("character_on") {
            textures.push(Tile::from_texture(tex.clone()));
        } else {
            textures.push(Tile::from_texture(Texture::white()));
        }

        if let Some(Value::Texture(tex)) = properties.get("character_off") {
            textures.push(Tile::from_texture(tex.clone()));
        } else {
            textures.push(Tile::from_texture(Texture::black()));
        }

        if let Some(Value::Texture(tex)) = properties.get("treasure_on") {
            textures.push(Tile::from_texture(tex.clone()));
        } else {
            textures.push(Tile::from_texture(Texture::white()));
        }

        if let Some(Value::Texture(tex)) = properties.get("treasure_off") {
            textures.push(Tile::from_texture(tex.clone()));
        } else {
            textures.push(Tile::from_texture(Texture::black()));
        }

        if self.map_tool_type == MapToolType::Effects {
            if let Some(Value::Texture(tex)) = properties.get("light_on") {
                textures.push(Tile::from_texture(tex.clone()));
            } else {
                textures.push(Tile::from_texture(Texture::white()));
            }

            if let Some(Value::Texture(tex)) = properties.get("light_off") {
                textures.push(Tile::from_texture(tex.clone()));
            } else {
                textures.push(Tile::from_texture(Texture::black()));
            }
        }

        let mut atlas_batch = Batch::emptyd2();
        let mut white_batch = Batch::emptyd2()
            .texture_index(1)
            .mode(crate::PrimitiveMode::Lines);
        let mut selected_batch = Batch::emptyd2().texture_index(2);
        let mut yellow_batch = Batch::emptyd2().texture_index(3);
        let mut red_batch = Batch::emptyd2().texture_index(4);
        let mut gray_batch = Batch::emptyd2().texture_index(5);
        let mut gray_batch_lines = Batch::emptyd2()
            .texture_index(5)
            .color([128, 128, 128, 255])
            .mode(crate::PrimitiveMode::Lines);

        let mut character_on_batch = Batch::emptyd2().texture_index(6);
        let mut character_off_batch = Batch::emptyd2().texture_index(7);

        let mut treasure_on_batch = Batch::emptyd2().texture_index(8);
        let mut treasure_off_batch = Batch::emptyd2().texture_index(9);

        let mut light_on_batch = Batch::emptyd2().texture_index(10);
        let mut light_off_batch = Batch::emptyd2().texture_index(11);

        // Add the material clipping area
        if self.material_mode {
            let tl = self.map_grid_to_local(screen_size, Vec2::new(-5.0, -5.0), map);
            let tr = self.map_grid_to_local(screen_size, Vec2::new(5.0, -5.0), map);
            let bl = self.map_grid_to_local(screen_size, Vec2::new(-5.0, 5.0), map);
            let br = self.map_grid_to_local(screen_size, Vec2::new(5.0, 5.0), map);
            gray_batch_lines.add_line(tl, tr, 1.0);
            gray_batch_lines.add_line(tl, bl, 1.0);
            gray_batch_lines.add_line(tr, br, 1.0);
            gray_batch_lines.add_line(bl, br, 1.0);
        }

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 3]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();
        //let mut yellow_rect = Batch::emptyd2().texture_index(1);

        // Grid
        grid_shader.set_parameter_f32("grid_size", map.grid_size);
        grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
        grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
        scene.background = Some(Box::new(grid_shader));

        // Add Sectors
        if self.map_tool_type == MapToolType::General
            || self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Rect
            || self.map_tool_type == MapToolType::Effects
            || self.map_tool_type == MapToolType::Linedef
            || self.map_tool_type == MapToolType::Vertex
            || self.map_tool_type == MapToolType::Game
            || self.map_tool_type == MapToolType::MiniMap
        {
            for sector in &map.sectors {
                if let Some(geo) = sector.generate_geometry(map) {
                    let mut vertices: Vec<[f32; 3]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];
                    let bbox = sector.bounding_box(map);

                    let repeat = true;
                    let tile_size = 100;

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

                    if let Some(Value::Source(pixelsource)) = sector.properties.get("floor_source")
                    {
                        if let Some(tile) =
                            pixelsource.to_tile(tiles, tile_size, &sector.properties)
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
                                vertices.push([local.x, local.y, 1.0]);
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

                        let mut vertices: Vec<[f32; 3]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];
                        let bbox = sector.bounding_box(map);

                        let repeat = true;
                        let tile_size = 100;

                        if let Some(pixelsource) = source {
                            if let Some(tile) =
                                pixelsource.to_tile(tiles, tile_size, &sector.properties)
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
                                    vertices.push([local.x, local.y, 1.0]);
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
                if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
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

                            let mut vertices: Vec<[f32; 3]> = vec![];
                            let mut uvs: Vec<[f32; 2]> = vec![];

                            let tile_size = 100;
                            if let Some(pixelsource) = source {
                                if let Some(tile) =
                                    pixelsource.to_tile(tiles, tile_size, &linedef.properties)
                                {
                                    for vertex in &geo.0 {
                                        let local = self.map_grid_to_local(
                                            screen_size,
                                            Vec2::new(vertex[0], vertex[1]),
                                            map,
                                        );

                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0]) / texture_scale,
                                            (vertex[1]) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                        vertices.push([local.x, local.y, 1.0]);
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
        }

        // Add Vertices
        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Vertex
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Linedef
        {
            for vertex in &map.vertices {
                if let Some(vertex_pos) = map.get_vertex(vertex.id) {
                    if self.map_tool_type == MapToolType::Linedef {
                        // In linedef mode, only show vertices that are part of selected linedefs
                        let mut found = false;
                        for linedef_id in map.selected_linedefs.iter() {
                            if let Some(linedef) = map.find_linedef(*linedef_id) {
                                if linedef.start_vertex == vertex.id
                                    || linedef.end_vertex == vertex.id
                                {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            continue;
                        }
                    } else if self.map_tool_type == MapToolType::Sector {
                        // In sector mode, only show vertices that are part of selected sectors
                        let mut found = false;
                        for sector_id in map.selected_sectors.iter() {
                            if let Some(sector) = map.find_sector(*sector_id) {
                                for linedef_id in sector.linedefs.iter() {
                                    if let Some(linedef) = map.find_linedef(*linedef_id) {
                                        if linedef.start_vertex == vertex.id
                                            || linedef.end_vertex == vertex.id
                                        {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if !found {
                            continue;
                        }
                    }

                    let pos = self.map_grid_to_local(screen_size, vertex_pos, map);

                    let size = 4.0;
                    if self.hover.0 == Some(vertex.id) || map.selected_vertices.contains(&vertex.id)
                    {
                        selected_batch.add_rectangle(
                            pos.x - size,
                            pos.y - size,
                            size * 2.0,
                            size * 2.0,
                        );
                    } else {
                        gray_batch.add_rectangle(
                            pos.x - size,
                            pos.y - size,
                            size * 2.0,
                            size * 2.0,
                        );
                    }
                }
            }
        }

        // Add Lines
        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Linedef
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Effects
            || self.map_tool_type == MapToolType::MiniMap
        {
            let mut selected_lines = vec![];
            let mut non_selected_lines = vec![];

            for linedef in &map.linedefs {
                if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                    let start_pos = self.map_grid_to_local(screen_size, start_vertex, map);
                    if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                        let end_pos = self.map_grid_to_local(screen_size, end_vertex, map);

                        // ---
                        // Check for wall lights
                        //
                        for i in 1..=4 {
                            let light_name = format!("row{}_light", i);
                            if let Some(Value::Light(light)) = linedef.properties.get(&light_name) {
                                let light =
                                    light.from_linedef(start_vertex, end_vertex, i as f32 - 0.5);
                                scene.lights.push(light);
                            }
                        }
                        // --

                        let mut selected = false;
                        if self.hover.1 == Some(linedef.id)
                            || map.selected_linedefs.contains(&linedef.id)
                        {
                            selected = true;
                        } else if self.map_tool_type == MapToolType::Sector
                            || self.map_tool_type == MapToolType::General
                            || self.map_tool_type == MapToolType::Selection
                        {
                            // Check for sector selection when in sector mode.
                            if let Some(front_sector) = linedef.front_sector {
                                if let Some(sector) = map.find_sector(front_sector) {
                                    if self.hover.2 == Some(sector.id)
                                        || map.selected_sectors.contains(&sector.id)
                                    {
                                        selected = true;
                                    }
                                }
                            }
                            if let Some(back_sector) = linedef.back_sector {
                                if let Some(sector) = map.find_sector(back_sector) {
                                    if self.hover.2 == Some(sector.id)
                                        || map.selected_sectors.contains(&sector.id)
                                    {
                                        selected = true;
                                    }
                                }
                            }
                        }

                        if selected {
                            selected_lines.push((start_pos, end_pos));
                        } else {
                            non_selected_lines.push((start_pos, end_pos));
                        }
                    }
                }
            }

            // Draw non-selected lines first
            for (start_pos, end_pos) in non_selected_lines {
                white_batch.add_line(start_pos, end_pos, 1.0);
            }

            // Draw selected lines last
            for (start_pos, end_pos) in selected_lines {
                selected_batch.add_line(start_pos, end_pos, 1.0);
            }
        }

        // For rectangle selection preview
        if let Some(rect) = map.curr_rectangle {
            white_batch.add_line(
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.1.x, rect.0.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.0.x, rect.0.y),
                Vec2::new(rect.0.x, rect.1.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.1.x, rect.0.y),
                1.0,
            );
            white_batch.add_line(
                Vec2::new(rect.1.x, rect.1.y),
                Vec2::new(rect.0.x, rect.1.y),
                1.0,
            );
        }

        // Lights
        if self.map_tool_type == MapToolType::Effects {
            for (index, l) in map.lights.iter().enumerate() {
                let position = l.position();
                let pos =
                    self.map_grid_to_local(screen_size, Vec2::new(position.x, position.z), map);
                let size = map.grid_size;
                let hsize = map.grid_size / 2.0;
                if Some(index as u32) == map.selected_light {
                    light_on_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                } else {
                    light_off_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                }
            }
        } else {
            // We dont show entities and items in Effects Mode to avoid overlapping icons
            // Entities
            for entity in &map.entities {
                let entity_pos = Vec2::new(entity.position.x, entity.position.z);
                let pos =
                    self.map_grid_to_local(screen_size, Vec2::new(entity_pos.x, entity_pos.y), map);
                let size = map.grid_size;
                let hsize = map.grid_size / 2.0;

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
                    if let Some(tile) = source.to_tile(tiles, 100, &entity.attributes) {
                        let texture_index = textures.len();

                        let mut batch = Batch::emptyd2()
                            .texture_index(texture_index)
                            .receives_light(true);

                        batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                        textures.push(tile.clone());
                        repeated_offsets.insert(tile.id, repeated_batches.len());
                        repeated_batches.push(batch);
                    }
                } else if Some(entity.creator_id) == map.selected_entity_item {
                    character_on_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                } else {
                    character_off_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                }
            }

            // Items
            for item in &map.items {
                let item_pos = Vec2::new(item.position.x, item.position.z);
                let pos =
                    self.map_grid_to_local(screen_size, Vec2::new(item_pos.x, item_pos.y), map);
                let size = map.grid_size;
                let hsize = map.grid_size / 2.0;

                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    let mut light = light.clone();
                    light.set_position(item.position);
                    scene.dynamic_lights.push(light);
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if let Some(tile) = source.to_tile(tiles, 100, &item.attributes) {
                        let texture_index = textures.len();

                        let mut batch = Batch::emptyd2()
                            .texture_index(texture_index)
                            .receives_light(true);

                        batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                        textures.push(tile.clone());
                        repeated_offsets.insert(tile.id, repeated_batches.len());
                        repeated_batches.push(batch);
                    }
                } else if Some(item.creator_id) == map.selected_entity_item {
                    treasure_on_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                } else {
                    treasure_off_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                }
            }
        }

        // For line action previews
        if let Some(grid_pos) = map.curr_grid_pos {
            let local = self.map_grid_to_local(screen_size, grid_pos, map);
            if let Some(mouse_pos) = map.curr_mouse_pos {
                white_batch.add_line(local, mouse_pos, 1.0)
            }
        }

        // Hover Cursor
        if let Some(hover_pos) = self.hover_cursor {
            let pos = self.map_grid_to_local(screen_size, hover_pos, map);
            let size = 4.0;
            yellow_batch.add_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0);
        }

        // Camera Pos
        if let Some(camera_pos) = self.camera_pos {
            let camera_grid_pos =
                self.map_grid_to_local(screen_size, Vec2::new(camera_pos.x, camera_pos.z), map);
            let size = 4.0;
            red_batch.add_rectangle(
                camera_grid_pos.x - size,
                camera_grid_pos.y - size,
                size * 2.0,
                size * 2.0,
            );

            // Look At Pos
            if let Some(look_at) = self.look_at {
                let look_at_grid_pos =
                    self.map_grid_to_local(screen_size, Vec2::new(look_at.x, look_at.z), map);
                gray_batch_lines.add_line(camera_grid_pos, look_at_grid_pos, 1.0);
                yellow_batch.add_rectangle(
                    look_at_grid_pos.x - size,
                    look_at_grid_pos.y - size,
                    size * 2.0,
                    size * 2.0,
                );
            }
        }

        let mut batches = repeated_batches;
        batches.extend(vec![
            atlas_batch,
            white_batch,
            selected_batch,
            gray_batch,
            gray_batch_lines,
            yellow_batch,
            red_batch,
            light_on_batch,
            light_off_batch,
            character_on_batch,
            character_off_batch,
            treasure_on_batch,
            treasure_off_batch,
        ]);

        scene.mapmini = map.as_mini();
        scene.d2 = batches;
        scene.textures = textures;
        scene
    }

    pub fn set_map_tool_type(&mut self, tool: MapToolType) {
        self.map_tool_type = tool;
    }

    pub fn set_map_hover_info(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.hover = hover;
        self.hover_cursor = hover_cursor;
    }

    pub fn set_camera_info(&mut self, pos: Option<vek::Vec3<f32>>, look_at: Option<Vec3<f32>>) {
        self.camera_pos = pos;
        self.look_at = look_at;
    }

    pub fn set_material_mode(&mut self, material_mode: bool) {
        self.material_mode = material_mode;
    }

    fn map_grid_to_local(
        &self,
        screen_size: Vec2<f32>,
        grid_pos: Vec2<f32>,
        map: &Map,
    ) -> Vec2<f32> {
        let grid_space_pos = grid_pos * map.grid_size;
        grid_space_pos + Vec2::new(map.offset.x, -map.offset.y) + screen_size / 2.0
    }
}
