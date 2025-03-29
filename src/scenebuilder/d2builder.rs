// use crate::PrimitiveMode::*;

use crate::Texture;
use crate::{
    Assets, Batch, GridShader, Map, MapToolType, Pixel, Rect, Scene, Shader, Tile, Value,
    ValueContainer, WHITE,
};
use theframework::prelude::*;
use vek::Vec2;

use MapToolType::*;

pub struct D2Builder {
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

    /// Clipping rectangle
    pub clip_rect: Option<Rect>,

    /// Draw Grid Switch
    pub draw_grid: bool,

    /// Stores textures for dynamic access
    pub textures: Vec<Tile>,

    /// Do not draw Rect based geometry
    no_rect_geo: bool,
}

impl Default for D2Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2Builder {
    pub fn new() -> Self {
        Self {
            selection_color: [187, 122, 208, 255],
            map_tool_type: Linedef,

            hover: (None, None, None),
            hover_cursor: None,

            camera_pos: None,
            look_at: None,

            clip_rect: None,
            draw_grid: true,

            textures: Vec::new(),

            no_rect_geo: false,
        }
    }

    pub fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        screen_size: Vec2<f32>,
        properties: &ValueContainer,
    ) -> Scene {
        let mut scene = Scene::empty();
        let atlas_size = assets.atlas.width as f32;
        self.no_rect_geo = properties.get_bool_default("no_rect_geo", true);

        // Grid
        if self.draw_grid {
            let mut grid_shader = GridShader::new();
            grid_shader.set_parameter_f32("grid_size", map.grid_size);
            grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
            grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
            scene.background = Some(Box::new(grid_shader));
        } else {
            scene.background = None;
        }

        let mut textures = vec![
            Tile::from_texture(assets.atlas.clone()),
            Tile::from_texture(Texture::from_color(WHITE)),
            Tile::from_texture(Texture::from_color(self.selection_color)),
            Tile::from_texture(Texture::from_color([128, 128, 128, 255])),
        ];

        // Add preview icon textures into the dynamic storage

        self.textures = vec![];
        self.textures
            .push(Tile::from_texture(Texture::from_color(WHITE)));

        self.textures.push(Tile::from_texture(Texture::from_color(
            self.selection_color,
        )));

        self.textures.push(Tile::from_texture(Texture::from_color([
            128, 128, 128, 255,
        ])));

        self.textures
            .push(Tile::from_texture(Texture::from_color([255, 255, 255, 30])));

        self.textures.push(Tile::from_texture(Texture::from_color(
            vek::Rgba::yellow().into_array(),
        )));
        self.textures.push(Tile::from_texture(Texture::from_color(
            vek::Rgba::red().into_array(),
        )));

        if let Some(Value::Texture(tex)) = properties.get("character_on") {
            self.textures.push(Tile::from_texture(tex.clone()));
        } else {
            self.textures.push(Tile::from_texture(Texture::white()));
        }

        if let Some(Value::Texture(tex)) = properties.get("character_off") {
            self.textures.push(Tile::from_texture(tex.clone()));
        } else {
            self.textures.push(Tile::from_texture(Texture::black()));
        }

        if let Some(Value::Texture(tex)) = properties.get("treasure_on") {
            self.textures.push(Tile::from_texture(tex.clone()));
        } else {
            self.textures.push(Tile::from_texture(Texture::white()));
        }

        if let Some(Value::Texture(tex)) = properties.get("treasure_off") {
            self.textures.push(Tile::from_texture(tex.clone()));
        } else {
            self.textures.push(Tile::from_texture(Texture::black()));
        }

        // if self.map_tool_type == MapToolType::Effects {
        //     if let Some(Value::Texture(tex)) = properties.get("light_on") {
        //         textures.push(Tile::from_texture(tex.clone()));
        //     } else {
        //         textures.push(Tile::from_texture(Texture::white()));
        //     }

        //     if let Some(Value::Texture(tex)) = properties.get("light_off") {
        //         textures.push(Tile::from_texture(tex.clone()));
        //     } else {
        //         textures.push(Tile::from_texture(Texture::black()));
        //     }
        // }

        // --

        let mut atlas_batch = Batch::emptyd2();

        // Repeated tile textures have their own batches
        let mut repeated_batches: Vec<Batch<[f32; 3]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

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

                    // Use the floor or ceiling source
                    let mut source = sector.properties.get("floor_source");
                    if source.is_none() {
                        source = sector.properties.get("ceiling_source");
                    }

                    if let Some(Value::Source(pixelsource)) = source {
                        if let Some(tile) =
                            pixelsource.to_tile(assets, tile_size, &sector.properties)
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
                                pixelsource.to_tile(assets, tile_size, &sector.properties)
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

                            let mut vertices: Vec<[f32; 3]> = vec![];
                            let mut uvs: Vec<[f32; 2]> = vec![];

                            let tile_size = 100;
                            if let Some(pixelsource) = source {
                                if let Some(tile) =
                                    pixelsource.to_tile(assets, tile_size, &linedef.properties)
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

        // Adjust the grid shader
        if let Some(grid_shader) = &mut scene.background {
            grid_shader.set_parameter_f32("grid_size", map.grid_size);
            grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
            grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
        }

        let mut textures = self.textures.clone();

        let mut white_batch = Batch::emptyd2()
            .texture_index(0)
            .mode(crate::PrimitiveMode::Lines);
        let mut selected_batch = Batch::emptyd2().texture_index(1);
        let mut selected_batch_lines = Batch::emptyd2()
            .color(self.selection_color)
            .mode(crate::PrimitiveMode::Lines);
        let mut gray_batch = Batch::emptyd2().texture_index(2);
        let mut gray_batch_lines = Batch::emptyd2()
            .texture_index(2)
            .color([128, 128, 128, 255])
            .mode(crate::PrimitiveMode::Lines);
        let mut clip_batch = Batch::emptyd2().texture_index(3);
        let mut yellow_batch = Batch::emptyd2().texture_index(4);
        let mut red_batch = Batch::emptyd2().texture_index(5);
        let mut character_on_batch = Batch::emptyd2().texture_index(6);
        let mut character_off_batch = Batch::emptyd2().texture_index(7);
        let mut treasure_on_batch = Batch::emptyd2().texture_index(8);
        let mut treasure_off_batch = Batch::emptyd2().texture_index(9);
        // let mut light_on_batch = Batch::emptyd2().texture_index(10);
        // let mut light_off_batch = Batch::emptyd2().texture_index(11);

        let mut repeated_batches: Vec<Batch<[f32; 3]>> = vec![];
        let mut repeated_offsets: FxHashMap<Uuid, usize> = FxHashMap::default();

        // Add the clipping area
        if let Some(clip_rect) = self.clip_rect {
            let tl = self.map_grid_to_local(screen_size, Vec2::new(clip_rect.x, clip_rect.y), map);
            let size = self.map_grid_to_local(
                screen_size,
                Vec2::new(clip_rect.width, clip_rect.height),
                map,
            );
            clip_batch.add_rectangle(tl.x, tl.y, size.x, size.y);
        }

        // Add Vertices
        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Vertex
            || self.map_tool_type == MapToolType::Sector
            || self.map_tool_type == MapToolType::Linedef
        {
            for vertex in &map.vertices {
                // if self.no_rect_geo {
                //     //} && map.is_vertex_in_rect(vertex.id) {
                //     continue;
                // }
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

                    let size = 0.2;
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
                let mut draw = true;

                // No outlines for the rect tool based sectors in the minimap or if no_rect_geo is enabled.
                if self.map_tool_type == MapToolType::MiniMap || self.no_rect_geo {
                    // let mut found_in_sector = false;
                    for sector in &map.sectors {
                        if sector.linedefs.contains(&linedef.id) {
                            // found_in_sector = true;
                            if sector.properties.contains("rect_rendering") {
                                draw = false;
                                break;
                            }
                        }
                    }

                    // If the linedef is not found in any sector and has a wall width of 0.0, don't draw it.
                    // Prevents deleted rect tool based sectors to be drawn.
                    // Problem: Also hides standalone walls
                    // if draw
                    //     && !found_in_sector
                    //     && linedef.properties.get_float_default("wall_width", 0.0) == 0.0
                    //     && !map.possible_polygon.contains(&linedef.id)
                    // {
                    //     draw = false;
                    // }
                }

                if draw {
                    if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                        let start_pos = self.map_grid_to_local(screen_size, start_vertex, map);
                        if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                            let end_pos = self.map_grid_to_local(screen_size, end_vertex, map);

                            // ---
                            // Check for wall lights
                            //
                            for i in 1..=4 {
                                let light_name = format!("row{}_light", i);
                                if let Some(Value::Light(light)) =
                                    linedef.properties.get(&light_name)
                                {
                                    let light = light.from_linedef(
                                        start_vertex,
                                        end_vertex,
                                        i as f32 - 0.5,
                                    );
                                    scene.dynamic_lights.push(light);
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
            }

            // Draw non-selected lines first
            for (start_pos, end_pos) in non_selected_lines {
                white_batch.add_line(start_pos, end_pos, 0.05);
            }

            // Draw selected lines last
            for (start_pos, end_pos) in selected_lines {
                selected_batch_lines.add_line(start_pos, end_pos, 0.05);
            }
        }

        if self.map_tool_type != MapToolType::Effects {
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
                        if let Some(tile) = source.to_tile(assets, 100, &entity.attributes) {
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
                let size = 1.0;
                let hsize = 0.5;

                if let Some(Value::Light(light)) = item.attributes.get("light") {
                    let mut light = light.clone();
                    light.set_position(item.position);
                    scene.dynamic_lights.push(light);
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if item.attributes.get_bool_default("visible", false) {
                        if let Some(tile) = source.to_tile(assets, 100, &item.attributes) {
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
                } else if Some(item.creator_id) == map.selected_entity_item {
                    treasure_on_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                } else {
                    treasure_off_batch.add_rectangle(pos.x - hsize, pos.y - hsize, size, size);
                }
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

        // For line action previews
        if let Some(grid_pos) = map.curr_grid_pos {
            let local = self.map_grid_to_local(screen_size, grid_pos, map);
            if let Some(mouse_pos) = map.curr_mouse_pos {
                white_batch.add_line(local, mouse_pos, 1.0)
            }
        }

        // Hover Cursor
        if self.map_tool_type != MapToolType::Rect {
            if let Some(hover_pos) = self.hover_cursor {
                let pos = self.map_grid_to_local(screen_size, hover_pos, map);
                let size = 0.2;
                yellow_batch.add_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0);
            }
        }

        // Camera Pos
        if let Some(camera_pos) = self.camera_pos {
            let camera_grid_pos =
                self.map_grid_to_local(screen_size, Vec2::new(camera_pos.x, camera_pos.z), map);
            let size = 0.2;
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
            clip_batch,
            white_batch,
            selected_batch,
            selected_batch_lines,
            gray_batch,
            gray_batch_lines,
            yellow_batch,
            red_batch,
            character_on_batch,
            character_off_batch,
            treasure_on_batch,
            treasure_off_batch,
        ]);

        scene.d2_dynamic = batches;
        scene.dynamic_textures = textures;
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

    pub fn set_clip_rect(&mut self, clip_rect: Option<Rect>) {
        self.clip_rect = clip_rect;
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
