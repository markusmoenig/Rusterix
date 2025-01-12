// use crate::PrimitiveMode::*;
use crate::SceneBuilder;
use crate::Texture;
use crate::{Batch, GridShader, Map, MapToolType, Pixel, Scene, Shader, Tile, Value, WHITE};
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
    pub look_at: vek::Vec3<f32>,
    /// Material Mode
    pub material_mode: bool,
}

impl SceneBuilder for D2PreviewBuilder {
    fn new() -> Self {
        Self {
            selection_color: [187, 122, 208, 255],
            map_tool_type: Linedef,

            hover: (None, None, None),
            hover_cursor: None,

            camera_pos: None,
            look_at: vek::Vec3::zero(),

            material_mode: false,
        }
    }

    fn build(
        &self,
        map: &Map,
        tiles: &FxHashMap<Uuid, Tile>,
        atlas: Texture,
        screen_size: Vec2<f32>,
        _camera_id: &str,
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
        {
            for sector in &map.sectors {
                if let Some(geo) = sector.generate_geometry(map) {
                    let mut vertices: Vec<[f32; 3]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];
                    let bbox = sector.bounding_box(map);

                    let repeat = true;
                    let tile_size = 100;

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
                                            + ((vertex[0] - bbox.0.x) / (bbox.1.x - bbox.0.x)
                                                * tile.uvs[index].z as f32))
                                            / atlas_size,
                                        ((tile.uvs[index].y as f32
                                            + (vertex[1] - bbox.0.y) / (bbox.1.y - bbox.0.y)
                                                * tile.uvs[index].w as f32)
                                            / atlas_size),
                                    ];
                                    uvs.push(uv);
                                } else {
                                    let texture_scale = 1.0;
                                    let uv = [
                                        (vertex[0] - bbox.0.x) / texture_scale,
                                        (vertex[1] - bbox.0.y) / texture_scale,
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
                                        .texture_index(texture_index);

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
        }

        // Add Vertices
        if self.map_tool_type == MapToolType::Selection || self.map_tool_type == MapToolType::Vertex
        {
            for vertex in &map.vertices {
                let pos = self.map_grid_to_local(screen_size, vertex.as_vec2(), map);

                let size = 4.0;
                if self.hover.0 == Some(vertex.id) || map.selected_vertices.contains(&vertex.id) {
                    selected_batch.add_rectangle(
                        pos.x - size,
                        pos.y - size,
                        size * 2.0,
                        size * 2.0,
                    );
                } else {
                    gray_batch.add_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0);
                }
            }
        }

        // Add Lines
        if self.map_tool_type == MapToolType::Selection
            || self.map_tool_type == MapToolType::Linedef
            || self.map_tool_type == MapToolType::Sector
        {
            for linedef in &map.linedefs {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    let start_pos =
                        self.map_grid_to_local(screen_size, start_vertex.as_vec2(), map);
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        let end_pos =
                            self.map_grid_to_local(screen_size, end_vertex.as_vec2(), map);

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
                            selected_batch.add_line(start_pos, end_pos, 1.0);
                        } else {
                            white_batch.add_line(start_pos, end_pos, 1.0);
                        }
                    }
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
            let pos =
                self.map_grid_to_local(screen_size, Vec2::new(camera_pos.x, camera_pos.z), map);
            let size = 4.0;
            red_batch.add_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0);
        }

        let mut batches = repeated_batches;
        batches.extend(vec![
            atlas_batch,
            white_batch,
            selected_batch,
            yellow_batch,
            red_batch,
            gray_batch,
            gray_batch_lines,
        ]);

        scene.d2 = batches;
        scene.textures = textures;
        scene
    }

    fn set_map_tool_type(&mut self, tool: MapToolType) {
        self.map_tool_type = tool;
    }

    fn set_map_hover_info(
        &mut self,
        hover: (Option<u32>, Option<u32>, Option<u32>),
        hover_cursor: Option<Vec2<f32>>,
    ) {
        self.hover = hover;
        self.hover_cursor = hover_cursor;
    }

    fn set_camera_info(&mut self, pos: Option<vek::Vec3<f32>>, look_at: vek::Vec3<f32>) {
        self.camera_pos = pos;
        self.look_at = look_at;
    }

    fn set_material_mode(&mut self, material_mode: bool) {
        self.material_mode = material_mode;
    }
}
