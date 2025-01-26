use crate::{Map, PixelSource, Value, ValueContainer};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Linedef {
    pub id: u32,

    // For editors
    pub creator_id: Uuid,

    pub name: String,
    pub start_vertex: u32,
    pub end_vertex: u32,

    pub front_sector: Option<u32>,
    pub back_sector: Option<u32>,

    pub properties: ValueContainer,
}

impl Linedef {
    pub fn new(id: u32, start_vertex: u32, end_vertex: u32) -> Self {
        let mut properties = ValueContainer::default();
        properties.set("wall_width", Value::Float(0.0));
        properties.set("wall_height", Value::Float(0.0));
        properties.set("row1_source", Value::Source(PixelSource::Off));
        properties.set("row2_source", Value::Source(PixelSource::Off));
        properties.set("row3_source", Value::Source(PixelSource::Off));
        Self {
            id,
            creator_id: Uuid::new_v4(),
            name: String::new(),
            start_vertex,
            end_vertex,
            front_sector: None,
            back_sector: None,

            properties,
        }
    }

    /// Calculate the length of the linedef, applying animation states
    pub fn length(&self, map: &Map) -> Option<f32> {
        let start = map.get_vertex(self.start_vertex)?;
        let end = map.get_vertex(self.end_vertex)?;

        Some((end - start).magnitude())
    }
}
/*
fn adjust_shared_vertex(
    map: &Map,
    linedef_a: &Linedef,
    linedef_b: &Linedef,
    shared_vertex: &Vertex,
    half_width: f32,
) -> ([f32; 3], [f32; 3]) {
    let start_a = map.vertices.get(linedef_a.start_vertex as usize).unwrap();
    let end_a = map.vertices.get(linedef_a.end_vertex as usize).unwrap();
    let start_b = map.vertices.get(linedef_b.start_vertex as usize).unwrap();
    let end_b = map.vertices.get(linedef_b.end_vertex as usize).unwrap();

    // Directions and perpendiculars for both linedefs
    let dir_a = vec2f(end_a.x - start_a.x, end_a.y - start_a.y).normalize();
    let dir_b = vec2f(end_b.x - start_b.x, end_b.y - start_b.y).normalize();

    let perp_a = vec2f(-dir_a.y, dir_a.x) * half_width;
    let perp_b = vec2f(-dir_b.y, dir_b.x) * half_width;

    // Calculate the intersection point of the offsets
    let offset_outer = intersect_lines(
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() + perp_a,
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() + perp_b,
    );

    let offset_inner = intersect_lines(
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() - perp_a,
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() - perp_b,
    );

    (
        [offset_outer.x, offset_outer.y, 0.0],
        [offset_inner.x, offset_inner.y, 0.0],
    )
}

fn intersect_lines(
    p1: Vec2f,
    p2: Vec2f,
    q1: Vec2f,
    q2: Vec2f,
) -> Vec2f {
    let a1 = p2.y - p1.y;
    let b1 = p1.x - p2.x;
    let c1 = a1 * p1.x + b1 * p1.y;

    let a2 = q2.y - q1.y;
    let b2 = q1.x - q2.x;
    let c2 = a2 * q1.x + b2 * q1.y;

    let det = a1 * b2 - a2 * b1;

    if det.abs() < f32::EPSILON {
        // Lines are parallel; return one of the endpoints
        return p1;
    }

    Vec2f::new(
        (b2 * c1 - b1 * c2) / det,
        (a1 * c2 - a2 * c1) / det,
    )
}
*/
