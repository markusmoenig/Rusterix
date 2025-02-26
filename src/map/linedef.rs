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

/// A "compiled" version which is used in MapMini for lighting, navigation etc
#[derive(Clone)]
pub struct CompiledLinedef {
    pub item_id: Option<u32>,

    pub start: Vec2<f32>,
    pub end: Vec2<f32>,

    pub wall_width: f32,
    pub wall_height: f32,

    pub casts_shadows: bool,
}

impl CompiledLinedef {
    pub fn new(
        item_id: Option<u32>,
        start: Vec2<f32>,
        end: Vec2<f32>,
        wall_width: f32,
        wall_height: f32,
        casts_shadows: bool,
    ) -> Self {
        Self {
            item_id,
            start,
            end,
            wall_width,
            wall_height,
            casts_shadows,
        }
    }
}
