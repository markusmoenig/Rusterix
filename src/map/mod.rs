pub mod light;
pub mod linedef;
pub mod meta;
pub mod pixelsource;
pub mod sector;
pub mod state;
pub mod tile;
pub mod vertex;

use crate::VertexAnimationSystem;

use linedef::*;
use sector::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::{Vec2, Vec4};
use vertex::*;

use crate::{Entity, Light};

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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Map {
    #[serde(default)]
    pub id: Uuid,
    pub name: String,

    pub offset: Vec2<f32>,
    pub grid_size: f32,
    pub subdivisions: f32,

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

    pub sky_texture: Option<Uuid>,

    // Camera Mode
    pub camera: MapCamera,
    #[serde(skip)]
    pub camera_xz: Option<Vec2<f32>>,

    // Lights
    pub lights: Vec<Light>,

    // Entities
    pub entities: Vec<Entity>,

    // Selection
    pub selected_vertices: Vec<u32>,
    pub selected_linedefs: Vec<u32>,
    pub selected_sectors: Vec<u32>,

    // Animation
    #[serde(default)]
    pub animation: VertexAnimationSystem,
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

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],

            sky_texture: None,

            camera: MapCamera::TwoD,
            camera_xz: None,

            lights: vec![],
            entities: vec![],

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            animation: VertexAnimationSystem::default(),
        }
    }

    // Clear temporary data
    pub fn clear_temp(&mut self) {
        self.possible_polygon = vec![];
        self.curr_grid_pos = None;
        self.curr_rectangle = None;
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

    /// Generate a bounding box for the sector, applying animation states
    pub fn bounding_box_animated(&self, map: &Map) -> Option<(Vec2<f32>, Vec2<f32>)> {
        let mut vertices = Vec::new();

        for linedef in &self.linedefs {
            if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                vertices.push(start_vertex);
            }
            if let Some(end_vertex) = map.get_vertex(linedef.end_vertex) {
                vertices.push(end_vertex);
            }
        }

        if vertices.is_empty() {
            return None;
        }

        // Calculate bounding box
        let min_x = vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
        let max_x = vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let max_y = vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);

        Some((Vec2::new(min_x, min_y), Vec2::new(max_x, max_y)))
    }

    /// Update the animation system
    pub fn tick(&mut self, delta_time: f32) {
        self.animation.update(delta_time, &mut self.vertices);
    }

    /// Get the current position of a vertex, applying the animation state and interpolation if available
    pub fn get_vertex(&self, vertex_id: u32) -> Option<Vec2<f32>> {
        // If there is a next state but no current state, interpolate with the base map
        if self.animation.current_state.is_none() {
            if let Some(next_state) = self.animation.next_state {
                let progress = self.animation.transition_progress;
                let adjusted_progress = self.animation.states[next_state]
                    .interpolation
                    .adjust_progress(progress);

                // Find the vertex in the base map
                let base_position = self
                    .vertices
                    .iter()
                    .find(|v| v.id == vertex_id)
                    .map(|v| Vec2::new(v.x, v.y));

                // Find the vertex in the next state
                let next_position = self.animation.states[next_state]
                    .vertices
                    .iter()
                    .find(|v| v.id == vertex_id)
                    .map(|v| v.position);

                // Interpolate between base and next state
                if let (Some(base_pos), Some(next_pos)) = (base_position, next_position) {
                    return Some(Vec2::lerp(base_pos, next_pos, adjusted_progress));
                }

                // Fallback to the next state if the vertex is not in the base map
                return next_position;
            }
        }

        // If both current and next states are set, interpolate between them
        if let Some(current_state) = self.animation.current_state {
            if let Some(next_state) = self.animation.next_state {
                let progress = self.animation.transition_progress;
                let adjusted_progress = self.animation.states[next_state]
                    .interpolation
                    .adjust_progress(progress);

                // Find the vertex in the current state
                let current_position = self.animation.states[current_state]
                    .vertices
                    .iter()
                    .find(|v| v.id == vertex_id)
                    .map(|v| v.position);

                // Find the vertex in the next state
                let next_position = self.animation.states[next_state]
                    .vertices
                    .iter()
                    .find(|v| v.id == vertex_id)
                    .map(|v| v.position);

                // Interpolate between the current and next state
                if let (Some(current_pos), Some(next_pos)) = (current_position, next_position) {
                    return Some(Vec2::lerp(current_pos, next_pos, adjusted_progress));
                }

                // Fallback to the current or next state
                return current_position.or(next_position);
            }

            // If only the current state is set, return its position
            if let Some(animated_vertex) = self.animation.states[current_state]
                .vertices
                .iter()
                .find(|v| v.id == vertex_id)
            {
                return Some(animated_vertex.position);
            }
        }

        // Fallback to the base vertex position
        self.vertices
            .iter()
            .find(|v| v.id == vertex_id)
            .map(|v| Vec2::new(v.x, v.y))
    }

    /// Update a vertex in the map, considering the animation state
    pub fn update_vertex(&mut self, vertex_id: u32, new_position: Vec2<f32>) {
        if let Some(current_state_index) = self.animation.current_state {
            // Update the vertex in the current animation state
            let current_state = &mut self.animation.states[current_state_index];
            current_state.update_or_add(vertex_id, new_position);
        } else {
            // Update the base vertex directly
            if let Some(base_vertex) = self.vertices.iter_mut().find(|v| v.id == vertex_id) {
                base_vertex.x = new_position.x;
                base_vertex.y = new_position.y;
            }
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

        let id = self.vertices.len() as u32;

        let vertex = Vertex::new(id, x, y);
        self.vertices.push(vertex);

        id
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
        let id = self.linedefs.len() as u32;
        let mut sector_id: Option<u32> = None;

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
    }

    /// Check if the `possible_polygon` forms a closed loop
    pub fn test_for_closed_polygon(&self) -> bool {
        if self.possible_polygon.len() < 3 {
            return false; // A polygon needs at least 3 edges
        }

        let first_linedef = &self.linedefs[self.possible_polygon[0] as usize];
        let last_linedef =
            &self.linedefs[self.possible_polygon[self.possible_polygon.len() - 1] as usize];

        // Check if the last linedef's end_vertex matches the first linedef's start_vertex
        last_linedef.end_vertex == first_linedef.start_vertex
    }

    /// Tries to create a polyon from the tracked vertices in possible_polygon
    pub fn create_sector_from_polygon(&mut self) -> Option<u32> {
        if !self.test_for_closed_polygon() {
            //println!("Polygon is not closed. Cannot create sector.");
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
        let sector_id = self.sectors.len() as u32;
        // println!(
        //     "Created sector ID: {} with linedefs: {:?}",
        //     sector_id, self.possible_polygon
        // );

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

        self.possible_polygon.clear(); // Reset after creating the sector
        Some(sector_id)
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
    pub fn delete_elements(&mut self, vertex_ids: &[u32], linedef_ids: &[u32], sector_ids: &[u32]) {
        // 1. Delete specified vertices
        if !vertex_ids.is_empty() {
            // Remove vertices
            self.vertices
                .retain(|vertex| !vertex_ids.contains(&vertex.id));

            // Remove any linedefs that depend on the deleted vertices
            self.linedefs.retain(|linedef| {
                !vertex_ids.contains(&linedef.start_vertex)
                    && !vertex_ids.contains(&linedef.end_vertex)
            });

            // Remove references to these linedefs in sectors
            self.cleanup_sectors();
        }

        // 2. Delete specified linedefs
        if !linedef_ids.is_empty() {
            // Remove linedefs
            self.linedefs
                .retain(|linedef| !linedef_ids.contains(&linedef.id));

            // Remove references to these linedefs in sectors
            self.cleanup_sectors();
        }

        // 3. Delete specified sectors
        if !sector_ids.is_empty() {
            // Remove sectors
            self.sectors
                .retain(|sector| !sector_ids.contains(&sector.id));

            // Remove references to these sectors in linedefs
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
}
