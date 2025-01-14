use crate::{Map, Value, ValueContainer};
use earcutr::earcut;
use theframework::prelude::*;

use super::pixelsource::PixelSource;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sector {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    pub linedefs: Vec<u32>,

    #[serde(default)]
    pub properties: ValueContainer,

    pub neighbours: Vec<u32>,
}

impl Sector {
    pub fn new(id: u32, linedefs: Vec<u32>) -> Self {
        let mut properties = ValueContainer::default();
        properties.set("floor_height", Value::Float(0.0));
        properties.set("ceiling_height", Value::Float(0.0));
        properties.set("floor_source", Value::Source(PixelSource::Off));
        properties.set("ceiling_source", Value::Source(PixelSource::Off));
        properties.set("row1_source", Value::Source(PixelSource::Off));
        properties.set("row2_source", Value::Source(PixelSource::Off));
        properties.set("row3_source", Value::Source(PixelSource::Off));

        Self {
            id,
            name: String::new(),
            linedefs,
            properties,
            neighbours: vec![],
        }
    }

    // Generate a bounding box for the sector
    pub fn bounding_box(&self, map: &Map) -> (Vec2<f32>, Vec2<f32>) {
        // Collect all vertices for the sector
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                if let Some(start_vertex) = map.vertices.get(linedef.start_vertex as usize) {
                    vertices.push(Vec2::new(start_vertex.x, start_vertex.y));
                    if let Some(end_vertex) = map.vertices.get(linedef.end_vertex as usize) {
                        vertices.push(Vec2::new(end_vertex.x, end_vertex.y));
                    }
                }
            }
        }

        // Find min and max coordinates
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

        // Return the bounding box corners
        (Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
    }

    /// Sets the wall height for all linedefs in the sector.
    pub fn set_wall_height(&mut self, _map: &mut Map, height: f32) {
        self.properties.set("wall_height", Value::Float(height));
        // for &linedef_id in &self.linedefs {
        //     if let Some(linedef) = map.linedefs.iter_mut().find(|l| l.id == linedef_id) {
        //         linedef.wall_height = height;
        //     }
        // }
    }

    /// Calculate the area of the sector (for sorting).
    pub fn area(&self, map: &Map) -> f32 {
        // Generate geometry for the sector
        if let Some((vertices, indices)) = self.generate_geometry(map) {
            // Calculate the area by summing up the areas of individual triangles
            indices.iter().fold(0.0, |acc, &(i1, i2, i3)| {
                let v1 = vertices[i1];
                let v2 = vertices[i2];
                let v3 = vertices[i3];

                // Calculate the area of the triangle using the shoelace formula
                acc + 0.5
                    * ((v1[0] * v2[1] + v2[0] * v3[1] + v3[0] * v1[1])
                        - (v1[1] * v2[0] + v2[1] * v3[0] + v3[1] * v1[0]))
                        .abs()
            })
        } else {
            0.0 // Return 0 if the geometry couldn't be generated
        }
    }

    /// Generate geometry (vertices and indices) for the polygon using earcutr
    #[allow(clippy::type_complexity)]
    pub fn generate_geometry(
        &self,
        map: &Map,
    ) -> Option<(Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> {
        // Collect unique vertices from the Linedefs in order
        let mut vertices = Vec::new();
        for &linedef_id in self.linedefs.iter() {
            let linedef = map.linedefs.get(linedef_id as usize)?;
            let start_vertex = map.get_vertex(linedef.start_vertex)?;
            let vertex = [start_vertex.x, start_vertex.y];

            // Add the vertex to the list if it isn't already there
            // if vertices.last() != Some(&vertex) {
            //     vertices.push(vertex);
            // }
            //
            if !vertices.contains(&vertex) {
                vertices.push(vertex);
            }
        }

        // Flatten the vertices for earcutr
        let flattened_vertices: Vec<f64> = vertices
            .iter()
            .flat_map(|v| vec![v[0] as f64, v[1] as f64])
            .collect();

        // No holes in this example, so pass an empty holes array
        let holes: Vec<usize> = Vec::new();

        // Perform triangulation
        if let Ok(indices) = earcut(&flattened_vertices, &holes, 2) {
            let indices: Vec<(usize, usize, usize)> = indices
                .chunks_exact(3)
                .map(|chunk| (chunk[2], chunk[1], chunk[0]))
                .collect();
            Some((vertices, indices))
        } else {
            None
        }
    }
}
