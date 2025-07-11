pub mod bbox;
pub mod geometry;
pub mod light;
pub mod linedef;
pub mod meta;
pub mod mini;
pub mod particle;
pub mod pixelsource;
pub mod sector;
pub mod softrig;
pub mod tile;
pub mod vertex;

use crate::{
    BBox, Keyform, MapMini, PixelSource, ShapeFXGraph, SoftRig, SoftRigAnimator, Terrain, Value,
    ValueContainer,
};
use indexmap::IndexMap;
use ordered_float::NotNan;
use pathfinding::prelude::astar;
use theframework::prelude::{FxHashMap, FxHashSet};

use linedef::*;
use sector::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::{Vec2, Vec4};
use vertex::*;

use crate::{Entity, Item, Light};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy)]
pub enum MapCamera {
    TwoD,
    ThreeDIso,
    ThreeDFirstPerson,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy)]
pub enum MapToolType {
    General,
    Selection,
    Vertex,
    Linedef,
    Sector,
    Effects,
    Rect,
    Game,
    MiniMap,
    World,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Map {
    #[serde(default)]
    pub id: Uuid,
    pub name: String,

    pub offset: Vec2<f32>,
    pub grid_size: f32,
    pub subdivisions: f32,

    #[serde(default)]
    pub terrain: Terrain,

    // When adding linedefs we keep track of them to check if we have a closed polygon
    #[serde(skip)]
    pub possible_polygon: Vec<u32>,

    // For temporary line previews
    #[serde(skip)]
    pub curr_grid_pos: Option<Vec2<f32>>,
    #[serde(skip)]
    pub curr_mouse_pos: Option<Vec2<f32>>,
    #[serde(skip)]
    pub curr_rectangle: Option<(Vec2<f32>, Vec2<f32>)>,

    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
    pub sectors: Vec<Sector>,

    #[serde(default)]
    pub shapefx_graphs: IndexMap<Uuid, ShapeFXGraph>,

    pub sky_texture: Option<Uuid>,

    // Camera Mode
    pub camera: MapCamera,
    #[serde(skip)]
    pub camera_xz: Option<Vec2<f32>>,
    #[serde(skip)]
    pub look_at_xz: Option<Vec2<f32>>,

    // Lights
    pub lights: Vec<Light>,

    // Entities
    pub entities: Vec<Entity>,

    // Items
    pub items: Vec<Item>,

    // Selection
    pub selected_vertices: Vec<u32>,
    pub selected_linedefs: Vec<u32>,
    pub selected_sectors: Vec<u32>,

    pub selected_light: Option<u32>,
    pub selected_entity_item: Option<Uuid>,

    // Meta Data
    #[serde(default)]
    pub properties: ValueContainer,

    /// All SoftRigs in the map, each defining vertex-based keyforms
    #[serde(default)]
    pub softrigs: IndexMap<Uuid, SoftRig>,

    /// Currently edited SoftRig, or None for base geometry
    #[serde(skip)]
    pub editing_rig: Option<Uuid>,

    /// Vertex animation
    #[serde(skip)]
    pub soft_animator: Option<SoftRigAnimator>,

    // Change counter, right now only used for materials
    // to indicate when to refresh live updates
    #[serde(default)]
    pub changed: u32,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Map".to_string(),

            offset: Vec2::zero(),
            grid_size: 30.0,
            subdivisions: 1.0,

            terrain: Terrain::default(),

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],

            shapefx_graphs: IndexMap::default(),
            sky_texture: None,

            camera: MapCamera::TwoD,
            camera_xz: None,
            look_at_xz: None,

            lights: vec![],
            entities: vec![],
            items: vec![],

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            selected_light: None,
            selected_entity_item: None,

            properties: ValueContainer::default(),
            softrigs: IndexMap::default(),
            editing_rig: None,
            soft_animator: None,

            changed: 0,
        }
    }

    /// Clear temporary data
    pub fn clear_temp(&mut self) {
        self.possible_polygon = vec![];
        self.curr_grid_pos = None;
        self.curr_rectangle = None;
        self.selected_light = None;
    }

    /// Clear the selection
    pub fn clear_selection(&mut self) {
        self.selected_vertices = vec![];
        self.selected_linedefs = vec![];
        self.selected_sectors = vec![];
        self.selected_light = None;
        self.selected_entity_item = None;
    }

    /// Return the Map as MapMini
    pub fn as_mini(&self, blocking_tiles: &FxHashSet<Uuid>) -> MapMini {
        let mut linedefs: Vec<CompiledLinedef> = vec![];
        let mut occluded_sectors: Vec<(BBox, f32)> = vec![];

        let mut blocked_tiles = FxHashSet::default();

        for sector in self.sectors.iter() {
            let mut casts_shadows = true;
            let mut add_it = true;

            // We collect occluded sectors
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut bbox = sector.bounding_box(self);
                bbox.expand(Vec2::new(0.1, 0.1));
                occluded_sectors.push((bbox, occlusion));
            }

            if sector.layer.is_some() {
                let render_mode = sector.properties.get_int_default("rect_rendering", 0);
                if render_mode != 1 {
                    casts_shadows = false;
                    add_it = false;
                }
                // If the tile is explicitly set to blocking we have to add the geometry
                match sector.properties.get("floor_source") {
                    Some(Value::Source(PixelSource::TileId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                            if let Some(center) = sector.center(self) {
                                blocked_tiles.insert(center.map(|c| (c.floor()) as i32));
                            }
                        }
                    }
                    Some(Value::Source(PixelSource::MaterialId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    _ => {}
                }
            }

            if add_it {
                for linedef_id in sector.linedefs.iter() {
                    if let Some(linedef) = self.find_linedef(*linedef_id) {
                        let mut casts_shadows = casts_shadows;
                        let cs = linedef.properties.get_int_default("casts_shadows", 0);
                        if cs == 1 {
                            casts_shadows = false;
                        }
                        let wall_height = linedef.properties.get_float_default("wall_height", 0.0);

                        if wall_height > 0.5 {
                            if let Some(start) = self.find_vertex(linedef.start_vertex) {
                                if let Some(end) = self.find_vertex(linedef.end_vertex) {
                                    let cl = CompiledLinedef::new(
                                        start.as_vec2(),
                                        end.as_vec2(),
                                        linedef.properties.get_float_default("wall_width", 0.0),
                                        linedef.properties.get_float_default("wall_height", 0.0),
                                        casts_shadows,
                                    );
                                    linedefs.push(cl);
                                }
                            }
                        }
                    }
                }
            }
        }

        for l in self.linedefs.iter() {
            if l.front_sector.is_none() && l.back_sector.is_none() {
                let wall_height = l.properties.get_float_default("wall_height", 0.0);

                let mut add_it = false;

                // If the tile is explicitly set to blocking we have to add the geometry
                match l.properties.get("row1_source") {
                    Some(Value::Source(PixelSource::TileId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    Some(Value::Source(PixelSource::MaterialId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    _ => {}
                }

                if add_it {
                    if let Some(start) = self.find_vertex(l.start_vertex) {
                        if let Some(end) = self.find_vertex(l.end_vertex) {
                            let cl = CompiledLinedef::new(
                                start.as_vec2(),
                                end.as_vec2(),
                                l.properties.get_float_default("wall_width", 0.0),
                                wall_height,
                                true,
                            );
                            linedefs.push(cl);
                        }
                    }
                }
            }
        }

        let mut mini = MapMini::new(self.offset, self.grid_size, linedefs, occluded_sectors);
        mini.blocked_tiles = blocked_tiles;
        mini
    }

    /// Generate a bounding box for all vertices in the map
    pub fn bbox(&self) -> BBox {
        // Find min and max coordinates among all vertices
        let min_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);
        BBox {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }

    /// Generate a bounding box for all vertices in the map
    pub fn bounding_box(&self) -> Option<Vec4<f32>> {
        if self.vertices.is_empty() {
            return None; // No vertices in the map
        }

        // Find min and max coordinates among all vertices
        let min_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);

        // Calculate width and height
        let width = max_x - min_x;
        let height = max_y - min_y;

        // Return the bounding box as Vec4f (x, y, width, height)
        Some(Vec4::new(min_x, min_y, width, height))
    }

    /// Tick the soft animator.
    pub fn tick(&mut self, delta_time: f32) {
        if let Some(anim) = &mut self.soft_animator {
            anim.tick(delta_time);
        }
    }

    /// Get the current position of a vertex, using any keyform override in the current SoftRig.
    pub fn get_vertex(&self, vertex_id: u32) -> Option<Vec2<f32>> {
        // Base vertex lookup
        let base = self.vertices.iter().find(|v| v.id == vertex_id)?;
        let base_pos = Vec2::new(base.x, base.y);

        // 1. Try runtime animation
        if let Some(animator) = &self.soft_animator {
            if let Some(rig) = animator.get_blended_rig(self) {
                if let Some((_, pos)) = rig
                    .keyforms
                    .first()
                    .and_then(|key| key.vertex_positions.iter().find(|(id, _)| *id == vertex_id))
                {
                    return Some(*pos);
                }
            }
        }

        // 2. Try editing override (if not currently animating)
        if self.soft_animator.is_none() {
            if let Some(rig_id) = self.editing_rig {
                if let Some(rig) = self.softrigs.get(&rig_id) {
                    for keyform in &rig.keyforms {
                        if let Some((_, pos)) = keyform
                            .vertex_positions
                            .iter()
                            .find(|(id, _)| *id == vertex_id)
                        {
                            return Some(*pos);
                        }
                    }
                }
            }
        }

        // 3. Fallback to base
        Some(base_pos)
    }

    /// Update the vertex position. If a keyform in the selected rig contains this vertex, update it.
    /// Otherwise, create a new keyform for this single vertex.
    pub fn update_vertex(&mut self, vertex_id: u32, new_position: Vec2<f32>) {
        // Update in active SoftRig
        if let Some(rig_id) = self.editing_rig {
            if let Some(rig) = self.softrigs.get_mut(&rig_id) {
                // Try to find a keyform that already contains this vertex
                for keyform in &mut rig.keyforms {
                    if let Some(entry) = keyform
                        .vertex_positions
                        .iter_mut()
                        .find(|(id, _)| *id == vertex_id)
                    {
                        entry.1 = new_position;
                        return;
                    }
                }

                // No existing keyform contains this vertex → create new keyform
                let new_keyform = Keyform {
                    vertex_positions: vec![(vertex_id, new_position)],
                };

                rig.keyforms.push(new_keyform);
                return;
            }
        }

        // Otherwise update base geometry
        if let Some(v) = self.vertices.iter_mut().find(|v| v.id == vertex_id) {
            v.x = new_position.x;
            v.y = new_position.y;
        }
    }

    // Add the vertex (and snap it to the subdivsion grid)
    pub fn add_vertex_at(&mut self, mut x: f32, mut y: f32) -> u32 {
        let subdivisions = 1.0 / self.subdivisions;

        x = (x / subdivisions).round() * subdivisions;
        y = (y / subdivisions).round() * subdivisions;

        // Check if the vertex already exists
        if let Some(id) = self.find_vertex_at(x, y) {
            return id;
        }

        if let Some(id) = self.find_free_vertex_id() {
            let vertex = Vertex::new(id, x, y);
            self.vertices.push(vertex);
            id
        } else {
            println!("No free vertex ID available");
            0
        }
    }

    /// Finds a vertex at the given position and returns its ID if it exists
    pub fn find_vertex_at(&self, x: f32, y: f32) -> Option<u32> {
        self.vertices
            .iter()
            .find(|v| v.x == x && v.y == y)
            .map(|v| v.id)
    }

    /// Finds a reference to a vertex by its ID
    pub fn find_vertex(&self, id: u32) -> Option<&Vertex> {
        self.vertices.iter().find(|vertex| vertex.id == id)
    }

    /// Finds a mutable reference to a vertex by its ID
    pub fn find_vertex_mut(&mut self, id: u32) -> Option<&mut Vertex> {
        self.vertices.iter_mut().find(|vertex| vertex.id == id)
    }

    /// Finds a reference to a linedef by its ID
    pub fn find_linedef(&self, id: u32) -> Option<&Linedef> {
        self.linedefs.iter().find(|linedef| linedef.id == id)
    }

    /// Finds a reference to a linedef by its ID
    pub fn find_linedef_mut(&mut self, id: u32) -> Option<&mut Linedef> {
        self.linedefs.iter_mut().find(|linedef| linedef.id == id)
    }

    /// Finds a mutable reference to a sector by its ID
    pub fn find_sector(&self, id: u32) -> Option<&Sector> {
        self.sectors.iter().find(|sector| sector.id == id)
    }

    /// Finds a mutable reference to a sector by its ID
    pub fn find_sector_mut(&mut self, id: u32) -> Option<&mut Sector> {
        self.sectors.iter_mut().find(|sector| sector.id == id)
    }

    // Create a new (or use an existing) linedef for the given vertices.
    pub fn create_linedef(&mut self, start_vertex: u32, end_vertex: u32) -> (u32, Option<u32>) {
        let mut sector_id: Option<u32> = None;

        if let Some(id) = self.find_free_linedef_id() {
            let linedef = Linedef::new(id, start_vertex, end_vertex);
            self.linedefs.push(linedef);
            self.possible_polygon.push(id);

            if let Some(sid) = self.create_sector_from_polygon() {
                if let Some(linedef) = self.find_linedef_mut(id) {
                    linedef.front_sector = Some(sid);
                }
                sector_id = Some(sid);
            }
            (id, sector_id)
        } else {
            println!("No free linedef ID available");
            (0, None)
        }
    }

    /// Check if the `possible_polygon` forms a closed loop
    pub fn test_for_closed_polygon(&self) -> bool {
        if self.possible_polygon.len() < 3 {
            return false; // A polygon needs at least 3 edges
        }

        if let Some(first_linedef) = self.find_linedef(self.possible_polygon[0]) {
            if let Some(last_linedef) =
                self.find_linedef(self.possible_polygon[self.possible_polygon.len() - 1])
            {
                // Check if the last linedef's end_vertex matches the first linedef's start_vertex
                return last_linedef.end_vertex == first_linedef.start_vertex;
            }
        }
        false
    }

    /// Tries to create a polyon from the tracked vertices in possible_polygon
    pub fn create_sector_from_polygon(&mut self) -> Option<u32> {
        if !self.test_for_closed_polygon() {
            // println!("Polygon is not closed. Cannot create sector.");
            return None;
        }

        // Check for duplicate sector
        if self
            .find_sector_by_linedefs(&self.possible_polygon)
            .is_some()
        {
            // println!(
            //     "Polygon already exists",
            // );
            self.possible_polygon.clear();
            return None;
        }

        // Create a new sector
        if let Some(sector_id) = self.find_free_sector_id() {
            for &id in &self.possible_polygon {
                if let Some(linedef) = self.linedefs.iter_mut().find(|l| l.id == id) {
                    // Assign the sector ID to the front or back
                    if linedef.front_sector.is_none() {
                        linedef.front_sector = Some(sector_id);
                    } else if linedef.back_sector.is_none() {
                        linedef.back_sector = Some(sector_id);
                    } else {
                        println!(
                            "Warning: Linedef {} already has both front and back sectors assigned.",
                            linedef.id
                        );
                    }
                }
            }

            let sector = Sector::new(sector_id, self.possible_polygon.clone());
            self.sectors.push(sector);

            self.possible_polygon.clear();
            Some(sector_id)
        } else {
            None
        }
    }

    /// Check if a set of linedefs matches any existing sector
    fn find_sector_by_linedefs(&self, linedefs: &[u32]) -> Option<u32> {
        for sector in &self.sectors {
            if sector.linedefs.len() == linedefs.len()
                && sector.linedefs.iter().all(|id| linedefs.contains(id))
            {
                return Some(sector.id);
            }
        }
        None
    }

    /// Deletes the specified vertices, linedefs, and sectors, along with their associated geometry.
    /*
    pub fn delete_elements(&mut self, vertex_ids: &[u32], linedef_ids: &[u32], sector_ids: &[u32]) {
        let mut all_linedef_ids = linedef_ids.to_vec();
        let mut all_vertex_ids = vertex_ids.to_vec();

        // 1. Collect linedefs and vertices from the sectors to delete
        if !sector_ids.is_empty() {
            for sector in &self.sectors {
                if sector_ids.contains(&sector.id) {
                    for &linedef_id in &sector.linedefs {
                        // Check if the linedef is used by other sectors
                        let used_elsewhere = self
                            .sectors
                            .iter()
                            .any(|s| s.id != sector.id && s.linedefs.contains(&linedef_id));

                        if !used_elsewhere && !all_linedef_ids.contains(&linedef_id) {
                            all_linedef_ids.push(linedef_id);
                        }

                        // Collect vertices only if not used by other linedefs
                        if let Some(linedef) = self.find_linedef(linedef_id) {
                            for &vertex_id in &[linedef.start_vertex, linedef.end_vertex] {
                                let used_elsewhere = self.linedefs.iter().any(|l| {
                                    l.id != linedef_id
                                        && (l.start_vertex == vertex_id
                                            || l.end_vertex == vertex_id)
                                });

                                if !used_elsewhere && !all_vertex_ids.contains(&vertex_id) {
                                    all_vertex_ids.push(vertex_id);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Delete vertices (after checking they are not used elsewhere)
        if !all_vertex_ids.is_empty() {
            self.vertices
                .retain(|vertex| !all_vertex_ids.contains(&vertex.id));

            self.linedefs.retain(|linedef| {
                !all_vertex_ids.contains(&linedef.start_vertex)
                    && !all_vertex_ids.contains(&linedef.end_vertex)
            });

            self.cleanup_sectors();
        }

        // 3. Delete linedefs
        if !all_linedef_ids.is_empty() {
            self.linedefs
                .retain(|linedef| !all_linedef_ids.contains(&linedef.id));

            self.cleanup_sectors();
        }

        // 4. Delete sectors
        if !sector_ids.is_empty() {
            self.sectors
                .retain(|sector| !sector_ids.contains(&sector.id));

            for linedef in &mut self.linedefs {
                if let Some(front_sector) = linedef.front_sector {
                    if sector_ids.contains(&front_sector) {
                        linedef.front_sector = None;
                    }
                }
                if let Some(back_sector) = linedef.back_sector {
                    if sector_ids.contains(&back_sector) {
                        linedef.back_sector = None;
                    }
                }
            }
        }
    }*/
    pub fn delete_elements(&mut self, vertex_ids: &[u32], linedef_ids: &[u32], sector_ids: &[u32]) {
        let mut all_linedef_ids = linedef_ids.to_vec();
        let mut all_vertex_ids = vertex_ids.to_vec();

        // Step 1: Collect linedefs (and eventually vertices) from sectors to delete
        if !sector_ids.is_empty() {
            for sector in &self.sectors {
                if sector_ids.contains(&sector.id) {
                    for &linedef_id in &sector.linedefs {
                        // If this linedef is not used by other sectors, we can delete it
                        let used_elsewhere = self
                            .sectors
                            .iter()
                            .any(|s| s.id != sector.id && s.linedefs.contains(&linedef_id));

                        if !used_elsewhere && !all_linedef_ids.contains(&linedef_id) {
                            all_linedef_ids.push(linedef_id);
                        }
                    }
                }
            }
        }

        // Step 2: Collect vertices used *only* by linedefs being deleted
        for &linedef_id in &all_linedef_ids {
            if let Some(linedef) = self.find_linedef(linedef_id) {
                for &vertex_id in &[linedef.start_vertex, linedef.end_vertex] {
                    let used_elsewhere = self.linedefs.iter().any(|l| {
                        l.id != linedef_id
                            && (l.start_vertex == vertex_id || l.end_vertex == vertex_id)
                            && !all_linedef_ids.contains(&l.id)
                    });

                    if !used_elsewhere && !all_vertex_ids.contains(&vertex_id) {
                        all_vertex_ids.push(vertex_id);
                    }
                }
            }
        }

        // Step 3: Delete sectors
        if !sector_ids.is_empty() {
            self.sectors
                .retain(|sector| !sector_ids.contains(&sector.id));

            for linedef in &mut self.linedefs {
                if let Some(front) = linedef.front_sector {
                    if sector_ids.contains(&front) {
                        linedef.front_sector = None;
                    }
                }
                if let Some(back) = linedef.back_sector {
                    if sector_ids.contains(&back) {
                        linedef.back_sector = None;
                    }
                }
            }
        }

        // Step 4: Delete linedefs
        if !all_linedef_ids.is_empty() {
            self.linedefs
                .retain(|linedef| !all_linedef_ids.contains(&linedef.id));
        }

        self.cleanup_sectors();

        // Step 5: Delete vertices (only if explicitly requested or now truly orphaned)
        if !all_vertex_ids.is_empty() {
            self.vertices
                .retain(|vertex| !all_vertex_ids.contains(&vertex.id));
        }
    }

    /// Cleans up sectors to ensure no references to deleted linedefs remain.
    fn cleanup_sectors(&mut self) {
        let valid_linedef_ids: std::collections::HashSet<u32> =
            self.linedefs.iter().map(|l| l.id).collect();

        for sector in &mut self.sectors {
            sector
                .linedefs
                .retain(|linedef_id| valid_linedef_ids.contains(linedef_id));
        }

        // Remove empty sectors
        self.sectors.retain(|sector| !sector.linedefs.is_empty());
    }

    /// Check if a given linedef ID is part of any sector.
    pub fn is_linedef_in_closed_polygon(&self, linedef_id: u32) -> bool {
        self.sectors
            .iter()
            .any(|sector| sector.linedefs.contains(&linedef_id))
    }

    /// Add the given geometry to the selection.
    pub fn add_to_selection(&mut self, vertices: Vec<u32>, linedefs: Vec<u32>, sectors: Vec<u32>) {
        for v in &vertices {
            if !self.selected_vertices.contains(v) {
                self.selected_vertices.push(*v);
            }
        }
        for l in &linedefs {
            if !self.selected_linedefs.contains(l) {
                self.selected_linedefs.push(*l);
            }
        }
        for s in &sectors {
            if !self.selected_sectors.contains(s) {
                self.selected_sectors.push(*s);
            }
        }
    }

    /// Remove the given geometry from the selection.
    pub fn remove_from_selection(
        &mut self,
        vertices: Vec<u32>,
        linedefs: Vec<u32>,
        sectors: Vec<u32>,
    ) {
        for v in &vertices {
            self.selected_vertices.retain(|&selected| selected != *v);
        }
        for l in &linedefs {
            self.selected_linedefs.retain(|&selected| selected != *l);
        }
        for s in &sectors {
            self.selected_sectors.retain(|&selected| selected != *s);
        }
    }

    /// Returns the sectors sorted from largest to smallest by area
    pub fn sorted_sectors_by_area(&self) -> Vec<&Sector> {
        let mut sectors_with_areas: Vec<(&Sector, f32)> = self
            .sectors
            .iter()
            .map(|sector| (sector, sector.area(self))) // Calculate the area for each sector
            .collect();

        // Sort by area in descending order
        sectors_with_areas
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return the sorted sectors
        sectors_with_areas
            .into_iter()
            .map(|(sector, _)| sector)
            .collect()
    }

    /// Adds a midpoint to a specified linedef, updates the geometry, and returns the new vertex ID.
    pub fn add_midpoint(&mut self, linedef_id: u32) -> Option<u32> {
        // Step 1: Find the linedef
        let linedef = self.find_linedef(linedef_id)?.clone(); // Clone to avoid borrow issues
        let start_vertex = self.find_vertex(linedef.start_vertex)?.clone();
        let end_vertex = self.find_vertex(linedef.end_vertex)?.clone();

        // Step 2: Calculate the midpoint
        let midpoint = Vec2::new(
            (start_vertex.x + end_vertex.x) / 2.0,
            (start_vertex.y + end_vertex.y) / 2.0,
        );

        // Step 3: Add the midpoint as a new vertex
        let new_vertex_id = self.add_vertex_at(midpoint.x, midpoint.y);

        // Step 4: Create new linedefs
        let mut new_linedef_1 = Linedef::new(
            linedef_id, // Use the same ID as the old linedef for the first new linedef
            linedef.start_vertex,
            new_vertex_id,
        );
        let mut new_linedef_2 = Linedef::new(
            self.linedefs.len() as u32, // New unique ID for the second linedef
            new_vertex_id,
            linedef.end_vertex,
        );

        // Assign the old properties of the linedef to the two new ones.
        new_linedef_1.properties = linedef.properties.clone();
        new_linedef_2.properties = linedef.properties.clone();

        // Step 5: Replace the old linedef in all sectors
        for sector in self.sectors.iter_mut() {
            if let Some(position) = sector.linedefs.iter().position(|&id| id == linedef_id) {
                // Replace the old linedef with the new ones in the correct order
                sector.linedefs.splice(
                    position..=position, // Replace the single linedef
                    [new_linedef_1.id, new_linedef_2.id].iter().cloned(), // Insert the new linedefs
                );
            }
        }

        // Step 6: Update the global linedef list
        if let Some(index) = self.linedefs.iter().position(|l| l.id == linedef_id) {
            self.linedefs[index] = new_linedef_1; // Replace the old linedef with the first new one
        }
        self.linedefs.push(new_linedef_2); // Add the second new linedef at the end

        // Return the ID of the new vertex
        Some(new_vertex_id)
    }

    /// Find sectors which consist of exactly the same 4 vertices and return them.
    /// This is used for stacking tiles / layering via the RECT tool.
    pub fn find_sectors_with_vertex_indices(&self, vertex_indices: &[u32; 4]) -> Vec<u32> {
        let mut matching_sectors = Vec::new();

        let mut new_vertex_set = vertex_indices.to_vec();
        new_vertex_set.sort();

        for sector in &self.sectors {
            let mut sector_vertex_indices = Vec::new();

            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    sector_vertex_indices.push(linedef.start_vertex);
                    sector_vertex_indices.push(linedef.end_vertex);
                }
            }

            // Deduplicate and sort for consistent comparison
            sector_vertex_indices.sort();
            sector_vertex_indices.dedup();

            // If the sector contains exactly these 4 vertices, it's a match
            if sector_vertex_indices == new_vertex_set {
                matching_sectors.push(sector.id);
            }
        }

        matching_sectors
    }

    /// Returns the sector at the given position (if any).
    pub fn find_sector_at(&self, position: Vec2<f32>) -> Option<&Sector> {
        self.sectors
            .iter()
            .find(|s| s.is_inside(self, position) && s.layer.is_none())
    }

    /// Checks if a linedef is passable based on front and back sectors.
    pub fn is_linedef_passable(&self, linedef_id: u32) -> bool {
        if let Some(linedef) = self.linedefs.iter().find(|ld| ld.id == linedef_id) {
            linedef.front_sector.is_some() && linedef.back_sector.is_some()
        } else {
            false
        }
    }

    /// Gets the neighbors of a sector, considering passable linedefs
    pub fn get_neighbors(&self, sector_id: u32) -> Vec<(u32, f32)> {
        self.sectors
            .iter()
            .find(|s| s.id == sector_id)
            .map(|sector| {
                sector
                    .linedefs
                    .iter()
                    .filter_map(|&linedef_id| {
                        // Get the linedef and check if it's passable
                        self.linedefs
                            .iter()
                            .find(|ld| ld.id == linedef_id)
                            .filter(|_linedef| self.is_linedef_passable(linedef_id))
                            .and_then(|linedef| {
                                // Determine the neighbor sector
                                linedef
                                    .front_sector
                                    .filter(|&id| id != sector_id)
                                    .or_else(|| linedef.back_sector.filter(|&id| id != sector_id))
                                    .map(|neighbor_id| (neighbor_id, 1.0))
                            })
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }

    /// Finds the shortest path to a named sector using the A* algorithm.
    pub fn find_path_to_sector(
        &self,
        start_sector_id: u32,
        target_sector_name: &str,
    ) -> Option<(Vec<u32>, f32)> {
        // Find the target sector by name
        let target_sector = self.sectors.iter().find(|s| s.name == target_sector_name)?;
        let target_sector_id = target_sector.id;

        // Use A* to find the shortest path
        astar(
            &start_sector_id,
            |&current_sector_id| {
                // Get the neighbors of the current sector and their costs
                self.get_neighbors(current_sector_id)
                    .into_iter()
                    .map(|(neighbor_id, cost)| (neighbor_id, NotNan::new(cost).unwrap()))
                    .collect::<Vec<(u32, NotNan<f32>)>>()
            },
            |&current_sector_id| {
                // Heuristic: Euclidean distance between sector centers
                let current_center = self
                    .sectors
                    .iter()
                    .find(|s| s.id == current_sector_id)
                    .and_then(|s| s.center(self));
                let target_center = target_sector.center(self);

                if let (Some(current), Some(target)) = (current_center, target_center) {
                    NotNan::new((target - current).magnitude()).unwrap()
                } else {
                    NotNan::new(f32::INFINITY).unwrap()
                }
            },
            |&current_sector_id| current_sector_id == target_sector_id, // Goal condition
        )
        .map(|(path, cost)| (path, cost.into_inner())) // Convert NotNan<f32> back to f32
    }

    /// Find the linedef which connects the two sectors.
    pub fn get_connecting_linedef(
        &self,
        current_sector_id: u32,
        next_sector_id: u32,
    ) -> Option<u32> {
        self.linedefs.iter().find_map(|linedef| {
            if self.is_linedef_passable(linedef.id) {
                if (linedef.front_sector == Some(current_sector_id)
                    && linedef.back_sector == Some(next_sector_id))
                    || (linedef.back_sector == Some(current_sector_id)
                        && linedef.front_sector == Some(next_sector_id))
                {
                    Some(linedef.id)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Debug: Print all vertices with their current animated positions
    pub fn debug_print_vertices(&self) {
        for vertex in &self.vertices {
            let current_position = self
                .get_vertex(vertex.id)
                .unwrap_or(Vec2::new(vertex.x, vertex.y));
            println!(
                "Vertex ID: {}, Base: ({}, {}), Animated: ({}, {})",
                vertex.id, vertex.x, vertex.y, current_position.x, current_position.y
            );
        }
    }

    /// Returns information about the Map
    pub fn info(&self) -> String {
        format!(
            "V {}, L {}, S {}",
            self.vertices.len(),
            self.linedefs.len(),
            self.sectors.len()
        )
    }

    // /// Returns true if the given vertex is part of a sector with rect rendering enabled.
    // pub fn is_vertex_in_rect(&self, vertex_id: u32) -> bool {
    //     for sector in &self.sectors {
    //         if sector.layer.is_none() {
    //             continue;
    //         }
    //         for linedef_id in sector.linedefs.iter() {
    //             if let Some(linedef) = self.find_linedef(*linedef_id) {
    //                 if linedef.start_vertex == vertex_id || linedef.end_vertex == vertex_id {
    //                     return true;
    //                 }
    //             }
    //         }
    //     }
    //     false
    // }

    /// Returns true if the given linedef is part of a sector with rect rendering enabled.
    pub fn is_linedef_in_rect(&self, linedef_id: u32) -> bool {
        for sector in &self.sectors {
            if sector.layer.is_none() {
                continue;
            }

            if sector.linedefs.contains(&linedef_id) {
                return true;
            }
        }
        false
    }

    /// Finds a free vertex ID that can be used for creating a new vertex.
    pub fn find_free_vertex_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.vertices.iter().any(|v| v.id == id))
    }

    /// Finds a free linedef ID that can be used for creating a new linedef.
    pub fn find_free_linedef_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.linedefs.iter().any(|l| l.id == id))
    }

    /// Finds a free sector ID that can be used for creating a new sector.
    pub fn find_free_sector_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.sectors.iter().any(|s| s.id == id))
    }

    /// Check if the map has selected geometry.
    pub fn has_selection(&self) -> bool {
        !self.selected_vertices.is_empty()
            || !self.selected_linedefs.is_empty()
            || !self.selected_sectors.is_empty()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.linedefs.is_empty() && self.sectors.is_empty()
    }

    /// Copy selected geometry into a new map
    pub fn copy_selected(&mut self, cut: bool) -> Map {
        let mut clipboard = Map::new();

        let mut old_to_new_vertex: FxHashMap<u32, u32> = FxHashMap::default();
        let mut old_to_new_linedef: FxHashMap<u32, u32> = FxHashMap::default();
        // let mut old_to_new_sector: FxHashMap<u32, u32> = FxHashMap::default();

        let mut vertex_ids: FxHashSet<u32> = FxHashSet::default();
        let mut linedef_ids: FxHashSet<u32> = self.selected_linedefs.iter().copied().collect();
        let sector_ids: FxHashSet<u32> = self.selected_sectors.iter().copied().collect();

        // Add linedefs from selected sectors
        for sid in &sector_ids {
            if let Some(sector) = self.find_sector(*sid) {
                for &lid in &sector.linedefs {
                    linedef_ids.insert(lid);
                }
            }
        }

        // Add vertices from selected linedefs
        for lid in &linedef_ids {
            if let Some(ld) = self.find_linedef(*lid) {
                vertex_ids.insert(ld.start_vertex);
                vertex_ids.insert(ld.end_vertex);
            }
        }

        // Add standalone selected vertices
        for &vid in &self.selected_vertices {
            vertex_ids.insert(vid);
        }

        // Normalize vertex positions
        let copied_vertices: Vec<Vertex> = vertex_ids
            .iter()
            .filter_map(|id| self.find_vertex(*id).cloned())
            .collect();

        if copied_vertices.is_empty() {
            return clipboard;
        }

        let min_x = copied_vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let min_y = copied_vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let offset = Vec2::new(min_x, min_y);

        // Remap and store vertices
        for old in copied_vertices {
            if let Some(new_id) = clipboard.find_free_vertex_id() {
                let mut new_v = old.clone();
                new_v.id = new_id;
                new_v.x -= offset.x;
                new_v.y -= offset.y;
                old_to_new_vertex.insert(old.id, new_id);
                clipboard.vertices.push(new_v);
            }
        }

        // Remap and store linedefs
        for old_id in &linedef_ids {
            if let Some(ld) = self.find_linedef(*old_id).cloned() {
                if let Some(new_id) = clipboard.find_free_linedef_id() {
                    let mut new_ld = ld.clone();
                    new_ld.id = new_id;
                    new_ld.start_vertex = *old_to_new_vertex.get(&ld.start_vertex).unwrap();
                    new_ld.end_vertex = *old_to_new_vertex.get(&ld.end_vertex).unwrap();
                    new_ld.front_sector = None;
                    new_ld.back_sector = None;
                    old_to_new_linedef.insert(ld.id, new_id);
                    clipboard.linedefs.push(new_ld);
                }
            }
        }

        // Remap and store sectors (only those whose linedefs were copied)
        for sid in &sector_ids {
            if let Some(s) = self.find_sector(*sid).cloned() {
                if s.linedefs.iter().all(|id| linedef_ids.contains(id)) {
                    if let Some(new_id) = clipboard.find_free_sector_id() {
                        let mut new_s = s.clone();
                        new_s.id = new_id;
                        new_s.linedefs = s
                            .linedefs
                            .iter()
                            .map(|id| *old_to_new_linedef.get(id).unwrap())
                            .collect();
                        // old_to_new_sector.insert(s.id, new_id);
                        clipboard.sectors.push(new_s);
                    }
                }
            }
        }

        // Delete source geometry if cutting
        if cut {
            self.delete_elements(
                &vertex_ids.iter().copied().collect::<Vec<_>>(),
                &linedef_ids.iter().copied().collect::<Vec<_>>(),
                &sector_ids.iter().copied().collect::<Vec<_>>(),
            );
            self.clear_selection();
        }

        clipboard
    }

    /// Inserts the given map at the given position.
    pub fn paste_at_position(&mut self, local_map: &Map, position: Vec2<f32>) {
        let mut vertex_map = FxHashMap::default();
        let mut linedef_map = FxHashMap::default();

        self.clear_selection();

        // Vertices
        for v in &local_map.vertices {
            if let Some(new_id) = self.find_free_vertex_id() {
                let mut new_v = v.clone();
                new_v.id = new_id;
                new_v.x += position.x;
                new_v.y += position.y;
                self.vertices.push(new_v);
                self.selected_vertices.push(new_id);
                vertex_map.insert(v.id, new_id);
            }
        }

        // Linedefs
        for l in &local_map.linedefs {
            if let Some(new_id) = self.find_free_linedef_id() {
                let mut new_l = l.clone();
                new_l.id = new_id;
                new_l.start_vertex = *vertex_map.get(&l.start_vertex).unwrap();
                new_l.end_vertex = *vertex_map.get(&l.end_vertex).unwrap();
                // Reset front/back sector
                new_l.front_sector = None;
                new_l.back_sector = None;
                self.linedefs.push(new_l);
                self.selected_linedefs.push(new_id);
                linedef_map.insert(l.id, new_id);
            }
        }

        // Sectors
        for s in &local_map.sectors {
            if let Some(new_id) = self.find_free_sector_id() {
                let mut new_s = s.clone();
                new_s.id = new_id;
                new_s.linedefs = s
                    .linedefs
                    .iter()
                    .map(|id| *linedef_map.get(id).unwrap())
                    .collect();

                // Assign sector to each of its linedefs
                for old_lid in &s.linedefs {
                    if let Some(&new_lid) = linedef_map.get(old_lid) {
                        if let Some(ld) = self.linedefs.iter_mut().find(|l| l.id == new_lid) {
                            if ld.front_sector.is_none() {
                                ld.front_sector = Some(new_id);
                            } else if ld.back_sector.is_none() {
                                ld.back_sector = Some(new_id);
                            } else {
                                eprintln!(
                                    "Linedef {} already has front and back set. Skipping assignment.",
                                    ld.id
                                );
                            }
                        }
                    }
                }

                self.sectors.push(new_s);
                self.selected_sectors.push(new_id);
            }
        }
    }

    /// Creates a geometry_clone clone of the map containing only vertices, linedefs, and sectors.
    pub fn geometry_clone(&self) -> Map {
        Map {
            id: Uuid::new_v4(),
            name: format!("{} (geometry_clone)", self.name),

            offset: self.offset,
            grid_size: self.grid_size,
            subdivisions: self.subdivisions,

            terrain: Terrain::default(),

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: self.vertices.clone(),
            linedefs: self.linedefs.clone(),
            sectors: self.sectors.clone(),

            shapefx_graphs: self.shapefx_graphs.clone(),
            sky_texture: None,

            camera: self.camera,
            camera_xz: None,
            look_at_xz: None,

            lights: vec![],
            entities: vec![],
            items: vec![],

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            selected_light: None,
            selected_entity_item: None,

            properties: ValueContainer::default(),
            softrigs: IndexMap::default(),
            editing_rig: None,
            soft_animator: None,

            changed: 0,
        }
    }

    /// Extracts all geometry into a new Map which intersects with the given chunk bbox.
    pub fn extract_chunk_geometry(&self, bbox: BBox) -> Map {
        let mut result = Map::new();

        let mut vertex_map: FxHashMap<u32, u32> = FxHashMap::default();
        let mut linedef_map: FxHashMap<u32, u32> = FxHashMap::default();

        // Step 1: Find all linedefs that intersect the BBox
        for l in &self.linedefs {
            if let (Some(start), Some(end)) = (
                self.get_vertex(l.start_vertex),
                self.get_vertex(l.end_vertex),
            ) {
                // Check if either endpoint is inside or the segment intersects bbox
                if bbox.contains(start) || bbox.contains(end) || bbox.line_intersects(start, end) {
                    let new_id = result.find_free_linedef_id().unwrap_or(l.id);
                    let mut l_clone = l.clone();
                    l_clone.id = new_id;
                    l_clone.front_sector = None;
                    l_clone.back_sector = None;
                    result.linedefs.push(l_clone);
                    linedef_map.insert(l.id, new_id);

                    // Ensure both vertices are marked for inclusion
                    for vid in &[l.start_vertex, l.end_vertex] {
                        if !vertex_map.contains_key(vid) {
                            if let Some(v) = self.find_vertex(*vid) {
                                let new_vid = result.find_free_vertex_id().unwrap_or(v.id);
                                let mut v_clone = v.clone();
                                v_clone.id = new_vid;
                                result.vertices.push(v_clone);
                                vertex_map.insert(*vid, new_vid);
                            }
                        }
                    }

                    // Reassign the vertex IDs
                    if let Some(ld) = result.linedefs.last_mut() {
                        ld.start_vertex = vertex_map[&l.start_vertex];
                        ld.end_vertex = vertex_map[&l.end_vertex];
                    }
                }
            }
        }

        // Step 2: Add sectors that reference any included linedef
        for s in &self.sectors {
            if s.linedefs.iter().any(|lid| linedef_map.contains_key(lid)) {
                let new_id = result.find_free_sector_id().unwrap_or(s.id);
                let mut s_clone = s.clone();
                s_clone.id = new_id;
                s_clone.linedefs = s
                    .linedefs
                    .iter()
                    .filter_map(|lid| linedef_map.get(lid).copied())
                    .collect();

                // Re-link sector ID into included linedefs
                for lid in &s.linedefs {
                    if let Some(&new_lid) = linedef_map.get(lid) {
                        if let Some(ld) = result.linedefs.iter_mut().find(|l| l.id == new_lid) {
                            if ld.front_sector.is_none() {
                                ld.front_sector = Some(new_id);
                            } else if ld.back_sector.is_none() {
                                ld.back_sector = Some(new_id);
                            }
                        }
                    }
                }

                result.sectors.push(s_clone);
            }
        }

        result
    }
}

use std::cmp::PartialEq;

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        let mut v1 = self.vertices.clone();
        let mut v2 = other.vertices.clone();
        v1.sort_by_key(|v| v.id);
        v2.sort_by_key(|v| v.id);

        let mut l1 = self.linedefs.clone();
        let mut l2 = other.linedefs.clone();
        l1.sort_by_key(|l| l.id);
        l2.sort_by_key(|l| l.id);

        let mut s1 = self.sectors.clone();
        let mut s2 = other.sectors.clone();
        s1.sort_by_key(|s| s.id);
        s2.sort_by_key(|s| s.id);

        v1 == v2 && l1 == l2 && s1 == s2
    }
}
