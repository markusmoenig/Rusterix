// use crate::PrimitiveMode::*;

use crate::Texture;
use crate::{
    Assets, Batch2D, GridShader, Map, MapToolType, Pixel, PixelSource, Rect, Scene, Shader, Tile,
    Value, ValueContainer, WHITE,
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

    /// Clipping rectangle
    pub clip_rect: Option<Rect>,

    /// Draw Grid Switch
    pub draw_grid: bool,

    /// Stores textures for dynamic access
    pub textures: Vec<Tile>,

    /// Do not draw Rect based geometry
    no_rect_geo: bool,

    tile_size: i32,
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

            clip_rect: None,
            draw_grid: true,

            textures: Vec::new(),

            no_rect_geo: false,

            tile_size: 128,
        }
    }

    pub fn set_properties(&mut self, properties: &ValueContainer) {
        self.no_rect_geo = true; //properties.get_bool_default("no_rect_geo", true);
        self.tile_size = properties.get_int_default("tile_size", 128);

        self.textures.clear();
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
    }

    pub fn build(
        &mut self,
        _map: &Map,
        _assets: &Assets,
        _screen_size: Vec2<f32>,
        _properties: &ValueContainer,
    ) -> Scene {
        /*
        let scene = Scene::empty();
        let atlas_size = assets.atlas.width as f32;
        self.no_rect_geo = properties.get_bool_default("no_rect_geo", true);
        self.tile_size = properties.get_int_default("tile_size", 128);

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
            200, 200, 200, 255,
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
        let mut repeated_batches: Vec<Batch2D> = vec![];
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
            let sorted = map.sorted_sectors_by_area();
            for sector in &sorted {
                if let Some(geo) = sector.generate_geometry(map) {
                    let mut vertices: Vec<[f32; 2]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];
                    let bbox = sector.bounding_box(map);

                    let mut repeat = true;
                    let tile_size = 100;

                    if sector.properties.get_int_default("tile_mode", 1) == 0 {
                        repeat = false;
                    }

                    // // Add Floor Light
                    // if let Some(Value::Light(light)) = sector.properties.get("floor_light") {
                    //     if let Some(center) = sector.center(map) {
                    //         let light =
                    //             light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                    //         scene.lights.push(light);
                    //     }
                    // }
                    // // Add Ceiling Light
                    // if let Some(Value::Light(light)) = sector.properties.get("ceiling_light") {
                    //     if let Some(center) = sector.center(map) {
                    //         let light =
                    //             light.from_sector(Vec3::new(center.x, 0.0, center.y), bbox.size());
                    //         scene.lights.push(light);
                    //     }
                    // }

                    let mut material: Option<Material> =
                        super::get_material_from_geo_graph(&sector.properties, 2, map);
                    if material.is_none() {
                        material = super::get_material_from_geo_graph(&sector.properties, 3, map);
                    }

                    // Use the floor or ceiling source
                    let mut source = sector.properties.get("floor_source");
                    if source.is_none() {
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

                            if material.is_some() {
                                let texture_index = textures.len();

                                let mut batch = Batch2D::empty()
                                    .repeat_mode(crate::RepeatMode::RepeatXY)
                                    .texture_index(texture_index)
                                    .receives_light(true);

                                batch.material = material;
                                batch.add(vertices, geo.1, uvs);

                                textures.push(tile.clone());
                                repeated_offsets.insert(tile.id, repeated_batches.len());
                                repeated_batches.push(batch);
                            } else if repeat {
                                if let Some(offset) = repeated_offsets.get(&tile.id) {
                                    repeated_batches[*offset].add(vertices, geo.1, uvs);
                                } else {
                                    let texture_index = textures.len();

                                    let mut batch = Batch2D::empty()
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

                                        let mut batch = Batch2D::empty()
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
                                        let uv = [
                                            (vertex[0]) / texture_scale,
                                            (vertex[1]) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                        vertices.push([local.x, local.y]);
                                    }

                                    if let Some(offset) = repeated_offsets.get(&tile.id) {
                                        repeated_batches[*offset].add(vertices, geo.1.clone(), uvs);
                                    } else {
                                        let texture_index = textures.len();

                                        let mut batch = Batch2D::empty()
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
        // batches.extend(vec![atlas_batch]);

        let tiles = assets.blocking_tiles();
        scene.mapmini = map.as_mini(&tiles);
        scene.d2_static = batches;
        scene.textures = textures;
        scene
        */
        Scene::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn build_entities_items(
        &self,
        map: &Map,
        assets: &Assets,
        scene: &mut Scene,
        screen_size: Vec2<f32>,
    ) {
        let screen_aspect = screen_size.x / screen_size.y;
        let screen_pixel_size = 4.0;
        let size_x = screen_pixel_size / map.grid_size;
        let size_y = size_x * screen_aspect / 2.0;

        scene.dynamic_lights = vec![];
        scene.d2_dynamic = vec![];

        // Grid
        if self.draw_grid {
            if scene.background.is_none() {
                let grid_shader = GridShader::new();
                scene.background = Some(Box::new(grid_shader));
            }
        } else {
            scene.background = None;
        }

        // Adjust the grid shader
        if let Some(grid_shader) = &mut scene.background {
            grid_shader.set_parameter_f32("grid_size", map.grid_size);
            grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
            grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
        }

        // Add the clipping area
        if let Some(clip_rect) = self.clip_rect {
            let size = self.map_grid_to_local(
                screen_size,
                Vec2::new(clip_rect.width, clip_rect.height),
                map,
            );
            let tl = Vec2::new(clip_rect.x, clip_rect.y);
            let batch = Batch2D::from_rectangle(tl.x, tl.y, size.x, size.y)
                .source(PixelSource::Pixel([255, 255, 255, 30]));
            scene.d2_dynamic.push(batch);
        }

        // Add Vertices

        let mut selected_batch = Batch2D::empty().source(PixelSource::Pixel(self.selection_color));
        let mut batch = Batch2D::empty().source(PixelSource::Pixel([128, 128, 128, 255]));

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

                    if self.hover.0 == Some(vertex.id) || map.selected_vertices.contains(&vertex.id)
                    {
                        selected_batch.add_rectangle(
                            pos.x - size_x,
                            pos.y - size_y,
                            size_x * 2.0,
                            size_y * 2.0,
                        );
                    } else {
                        batch.add_rectangle(
                            pos.x - size_x,
                            pos.y - size_y,
                            size_x * 2.0,
                            size_y * 2.0,
                        );
                    }
                }
            }
        }
        scene.d2_dynamic.push(selected_batch);
        scene.d2_dynamic.push(batch);

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
                            for i in 1..=4 {
                                if let Some(light) = super::get_linedef_light_from_geo_graph(
                                    &linedef.properties,
                                    i,
                                    map,
                                    start_vertex,
                                    end_vertex,
                                    i as f32 - 0.5,
                                ) {
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
            let mut batch = Batch2D::empty()
                .source(PixelSource::Pixel(WHITE))
                .mode(crate::PrimitiveMode::Lines);
            for (start_pos, end_pos) in non_selected_lines {
                batch.add_line(start_pos, end_pos, 0.05);
            }
            scene.d2_dynamic.push(batch);

            // Draw selected lines last
            let mut batch = Batch2D::empty()
                .source(PixelSource::Pixel(self.selection_color))
                .mode(crate::PrimitiveMode::Lines);
            for (start_pos, end_pos) in selected_lines {
                batch.add_line(start_pos, end_pos, 0.05);
            }
            scene.d2_dynamic.push(batch);
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
                    scene.dynamic_lights.push(light.compile());
                }

                // Find light on entity items
                for item in entity.iter_inventory() {
                    if let Some(Value::Light(light)) = item.attributes.get("light") {
                        let mut light = light.clone();
                        light.set_position(entity.position);
                        scene.dynamic_lights.push(light.compile());
                    }
                }

                if let Some(Value::Source(source)) = entity.attributes.get("source") {
                    if entity.attributes.get_bool_default("visible", false) {
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            if let Some(texture_index) = assets.tile_index(&tile.id) {
                                let batch = Batch2D::from_rectangle(
                                    pos.x - hsize,
                                    pos.y - hsize,
                                    size,
                                    size,
                                )
                                .source(PixelSource::StaticTileIndex(texture_index));
                                scene.d2_dynamic.push(batch);
                            }
                        }
                    }
                } else if Some(entity.creator_id) == map.selected_entity_item {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(0));
                    scene.d2_dynamic.push(batch);
                } else {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(1));
                    scene.d2_dynamic.push(batch);
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
                    scene.dynamic_lights.push(light.compile());
                }

                if let Some(Value::Source(source)) = item.attributes.get("source") {
                    if item.attributes.get_bool_default("visible", false) {
                        if let Some(tile) = source.tile_from_tile_list(assets) {
                            if let Some(texture_index) = assets.tile_index(&tile.id) {
                                let batch = Batch2D::from_rectangle(
                                    pos.x - hsize,
                                    pos.y - hsize,
                                    size,
                                    size,
                                )
                                .source(PixelSource::StaticTileIndex(texture_index));
                                scene.d2_dynamic.push(batch);
                            }
                        }
                    }
                } else if Some(item.creator_id) == map.selected_entity_item {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(2));
                    scene.d2_dynamic.push(batch);
                } else {
                    let batch = Batch2D::from_rectangle(pos.x - hsize, pos.y - hsize, size, size)
                        .source(PixelSource::DynamicTileIndex(3));
                    scene.d2_dynamic.push(batch);
                }
            }
        }

        let mut white_batch = Batch2D::empty()
            .source(PixelSource::Pixel(WHITE))
            .mode(crate::PrimitiveMode::Lines);

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
            if let Some(mouse_pos) = map.curr_mouse_pos {
                white_batch.add_line(grid_pos, mouse_pos, 1.0)
            }
        }
        scene.d2_dynamic.push(white_batch);

        // Hover Cursor
        if self.map_tool_type != MapToolType::Rect {
            if let Some(hover_pos) = self.hover_cursor {
                let mut yellow_batch =
                    Batch2D::empty().source(PixelSource::Pixel(vek::Rgba::yellow().into_array()));
                yellow_batch.add_rectangle(
                    hover_pos.x - size_x,
                    hover_pos.y - size_y,
                    size_x * 2.0,
                    size_y * 2.0,
                );
                scene.d2_dynamic.push(yellow_batch);
            }
        }

        /*
        // Camera Pos
        if let Some(camera_pos) = self.camera_pos {
            let camera_grid_pos =
                self.map_grid_to_local(screen_size, Vec2::new(camera_pos.x, camera_pos.z), map);
            red_batch.add_rectangle(
                camera_grid_pos.x - size_x,
                camera_grid_pos.y - size_y,
                size_x * 2.0,
                size_y * 2.0,
            );

            // Look At Pos
            if let Some(look_at) = self.look_at {
                let look_at_grid_pos =
                    self.map_grid_to_local(screen_size, Vec2::new(look_at.x, look_at.z), map);
                gray_batch_lines.add_line(camera_grid_pos, look_at_grid_pos, 1.0);
                yellow_batch.add_rectangle(
                    look_at_grid_pos.x - size_x,
                    look_at_grid_pos.y - size_y,
                    size_x * 2.0,
                    size_y * 2.0,
                );
            }
        }
        */

        scene.dynamic_textures = self.textures.clone();
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
