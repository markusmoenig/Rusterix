// use crate::PrimitiveMode::*;
use crate::SceneBuilder;
use crate::Texture;
use crate::{Batch, GridShader, Map, MapToolType, Pixel, Scene, Shader, Tile, WHITE};
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
}

impl SceneBuilder for D2PreviewBuilder {
    fn new() -> Self {
        Self {
            selection_color: [187, 122, 208, 255],
            map_tool_type: Linedef,

            hover: (None, None, None),
            hover_cursor: None,
        }
    }

    fn build(&self, map: &Map, _tiles: FxHashMap<Uuid, Tile>, screen_size: Vec2<f32>) -> Scene {
        let mut scene = Scene::empty();
        let mut grid_shader = GridShader::new();

        scene.textures = vec![
            Texture::from_color(WHITE),
            Texture::from_color(self.selection_color),
            Texture::from_color(vek::Rgba::yellow().into_array()),
            Texture::from_color([128, 128, 128, 255]),
        ];

        let mut white_batch = Batch::emptyd2();
        let mut selected_batch = Batch::emptyd2().texture_index(1);

        //let mut yellow_rect = Batch::emptyd2().texture_index(1);

        // Grid
        grid_shader.set_parameter_f32("grid_size", map.grid_size);
        grid_shader.set_parameter_f32("subdivisions", map.subdivisions);
        grid_shader.set_parameter_vec2("offset", Vec2::new(map.offset.x, -map.offset.y));
        scene.background = Some(Box::new(grid_shader));

        // Draw Vertices
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
                    white_batch.add_rectangle(pos.x - size, pos.y - size, size * 2.0, size * 2.0);
                }

                // let size = 4.0;
                // drawer.add_box(
                //     pos.x - size,
                //     pos.y - size,
                //     size * 2.0,
                //     size * 2.0,
                //     Rgba::new(color[0], color[1], color[2], color[3]),
                // );
            }
        }

        // Draw Lines
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

        scene.d2 = vec![white_batch, selected_batch];
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
}
